use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const RATE_LIMIT_WINDOW_SECS: u64 = 60;
const MAX_REQUESTS_PER_WINDOW: usize = 120; // Default: 120 RPM per IP per token

lazy_static! {
    // [IP_KEY] -> (Count, Expiration)
    static ref RATE_LIMITER: Mutex<HashMap<String, (usize, Instant)>> = Mutex::new(HashMap::new());

    // UserToken_ID -> HashSet<IP>
    static ref TOKEN_IPS: Mutex<HashMap<String, HashSet<String>>> = Mutex::new(HashMap::new());

    // IP Rules Cache
    static ref IP_RULES_CACHE: std::sync::RwLock<Vec<crate::models::IpRule>> = std::sync::RwLock::new(Vec::new());
}

pub struct SecurityShield;

pub enum SecurityAction {
    Allow,
    Deny(String), // Deny reason
}

fn ip_rule_matches(ip: &str, rule: &str) -> bool {
    let rule = rule.trim();
    let ip = ip.trim();

    if rule.is_empty() || ip.is_empty() {
        return false;
    }

    if rule.contains('/') {
        return cidr_matches(ip, rule);
    }

    if rule.ends_with('*') {
        let prefix = rule.trim_end_matches('*');
        return !prefix.is_empty() && ip.starts_with(prefix);
    }

    match (ip.parse::<IpAddr>(), rule.parse::<IpAddr>()) {
        (Ok(ip_addr), Ok(rule_addr)) => ip_addr == rule_addr,
        _ => ip == rule,
    }
}

fn cidr_matches(ip: &str, cidr: &str) -> bool {
    let (network, prefix) = match cidr.split_once('/') {
        Some(parts) => parts,
        None => return false,
    };

    let prefix = match prefix.trim().parse::<u8>() {
        Ok(prefix) => prefix,
        Err(_) => return false,
    };

    match (ip.parse::<IpAddr>(), network.trim().parse::<IpAddr>()) {
        (Ok(IpAddr::V4(ip_addr)), Ok(IpAddr::V4(network_addr))) => {
            if prefix > 32 {
                return false;
            }
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - prefix)
            };
            (u32::from(ip_addr) & mask) == (u32::from(network_addr) & mask)
        }
        (Ok(IpAddr::V6(ip_addr)), Ok(IpAddr::V6(network_addr))) => {
            if prefix > 128 {
                return false;
            }
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - prefix)
            };
            (u128::from(ip_addr) & mask) == (u128::from(network_addr) & mask)
        }
        _ => false,
    }
}

impl SecurityShield {
    pub fn sync_rules(db: &crate::db::Database) -> crate::error::AppResult<()> {
        let rules: Vec<crate::models::IpRule> = db.query_rows(
            "SELECT id, ip_cidr, rule_type, notes, is_active, created_at FROM ip_rules WHERE is_active = 1",
            &[],
            |r| {
                Ok(crate::models::IpRule {
                    id: r.get(0)?,
                    ip_cidr: r.get(1)?,
                    rule_type: r.get(2)?,
                    notes: {
                        let n: String = r.get(3)?;
                        if n.is_empty() { None } else { Some(n) }
                    },
                    is_active: r.get(4)?,
                    created_at: r.get(5)?,
                })
            },
        )?;

        if let Ok(mut cache) = IP_RULES_CACHE.write() {
            *cache = rules;
        }
        Ok(())
    }

    /// 校验 IP 是否被黑白名单拦截
    pub fn verify_ip_rule(ip: &str) -> SecurityAction {
        if let Ok(rules) = IP_RULES_CACHE.read() {
            let mut has_whitelist = false;
            let mut matched_whitelist = false;

            for r in rules.iter() {
                let is_match = ip_rule_matches(ip, &r.ip_cidr);

                if r.rule_type == "whitelist" {
                    has_whitelist = true;
                    if is_match {
                        matched_whitelist = true;
                    }
                }

                if r.rule_type == "blacklist" && is_match {
                    return SecurityAction::Deny("IP is blacklisted by administrator".to_string());
                }
            }

            // 如果配置了白名单机制，且没有命中任何白名单，则拦截
            if has_whitelist && !matched_whitelist {
                return SecurityAction::Deny("IP is not in whitelist".to_string());
            }
        }
        SecurityAction::Allow
    }

    /// 检查并扣除基于 IP + Token 的限制
    pub fn check_rate_limit(ip: &str, token_id: Option<&str>) -> SecurityAction {
        let key = if let Some(t_id) = token_id {
            format!("{}_{}", t_id, ip)
        } else {
            ip.to_string()
        };

        let mut limiter = RATE_LIMITER.lock().unwrap();
        let now = Instant::now();

        // 垃圾回收，保持 HashMap 干净（此处简化为每次访问时顺手清理当前 key，实际高并发可用专门线程或更复杂的结构）
        let (count, expires) = limiter
            .entry(key.clone())
            .or_insert((0, now + Duration::from_secs(RATE_LIMIT_WINDOW_SECS)));

        if now > *expires {
            *count = 1;
            *expires = now + Duration::from_secs(RATE_LIMIT_WINDOW_SECS);
            SecurityAction::Allow
        } else {
            if *count >= MAX_REQUESTS_PER_WINDOW {
                SecurityAction::Deny(format!(
                    "Rate limit exceeded: Max {} requests per {} seconds",
                    MAX_REQUESTS_PER_WINDOW, RATE_LIMIT_WINDOW_SECS
                ))
            } else {
                *count += 1;
                SecurityAction::Allow
            }
        }
    }

    /// 检查指定 Token 的并发 IP 数是否超过阈值 (max_ips)
    /// 如果 max_ips == 0，表示不限制
    pub fn verify_max_ips(token_id: &str, incoming_ip: &str, max_ips: i64) -> SecurityAction {
        if max_ips <= 0 {
            return SecurityAction::Allow;
        }

        let mut token_ip_map = TOKEN_IPS.lock().unwrap();
        let ips = token_ip_map
            .entry(token_id.to_string())
            .or_insert_with(HashSet::new);

        // 如果 IP 已经在池子里，直接放行
        if ips.contains(incoming_ip) {
            return SecurityAction::Allow;
        }

        // 如果 IP 不在池子里，且已经达到上限，直接拦截
        if ips.len() as i64 >= max_ips {
            return SecurityAction::Deny(format!(
                "Token security violation: Reached maximum concurrent bound IPs ({})",
                max_ips
            ));
        }

        // 没达到上限，加入池子
        ips.insert(incoming_ip.to_string());
        SecurityAction::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_test_rules(rules: Vec<crate::models::IpRule>) {
        let mut cache = IP_RULES_CACHE.write().unwrap();
        *cache = rules;
    }

    fn test_rule(ip_cidr: &str, rule_type: &str) -> crate::models::IpRule {
        crate::models::IpRule {
            id: format!("{}-{}", rule_type, ip_cidr),
            ip_cidr: ip_cidr.to_string(),
            rule_type: rule_type.to_string(),
            notes: None,
            is_active: true,
            created_at: 0,
        }
    }

    #[test]
    fn matches_exact_and_wildcard_rules() {
        assert!(ip_rule_matches("127.0.0.1", "127.0.0.1"));
        assert!(ip_rule_matches("127.0.0.42", "127.0.0.*"));
        assert!(!ip_rule_matches("127.0.1.42", "127.0.0.*"));
    }

    #[test]
    fn matches_ipv4_and_ipv6_cidr_rules() {
        assert!(cidr_matches("192.168.1.24", "192.168.1.0/24"));
        assert!(!cidr_matches("192.168.2.24", "192.168.1.0/24"));
        assert!(cidr_matches("2001:db8::10", "2001:db8::/64"));
        assert!(!cidr_matches("2001:db9::10", "2001:db8::/64"));
    }

    #[test]
    fn rejects_invalid_cidr_rules() {
        assert!(!cidr_matches("192.168.1.24", "192.168.1.0/not-a-prefix"));
        assert!(!cidr_matches("192.168.1.24", "192.168.1.0/33"));
        assert!(!cidr_matches("2001:db8::10", "2001:db8::/129"));
        assert!(!cidr_matches("192.168.1.24", "not-an-ip/24"));
    }

    #[test]
    fn denies_blacklisted_ip_when_rule_matches() {
        set_test_rules(vec![test_rule("10.0.0.0/8", "blacklist")]);

        let result = SecurityShield::verify_ip_rule("10.12.3.4");
        assert!(matches!(result, SecurityAction::Deny(_)));

        set_test_rules(Vec::new());
    }

    #[test]
    fn denies_non_whitelisted_ip_when_whitelist_exists() {
        set_test_rules(vec![test_rule("192.168.0.0/16", "whitelist")]);

        let denied = SecurityShield::verify_ip_rule("10.0.0.1");
        let allowed = SecurityShield::verify_ip_rule("192.168.1.10");

        assert!(matches!(denied, SecurityAction::Deny(_)));
        assert!(matches!(allowed, SecurityAction::Allow));

        set_test_rules(Vec::new());
    }
}
