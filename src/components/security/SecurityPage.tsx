import { useState, useEffect } from "react";
import { Shield, Lock, FileText, Activity, RefreshCw, Trash2, CheckCircle2, XCircle, Plus, AlertCircle } from "lucide-react";
import { api } from "../../lib/api";

type TabType = "logs" | "rules";

export default function SecurityPage() {
  const [activeTab, setActiveTab] = useState<TabType>("logs");
  
  const [logs, setLogs] = useState<any[]>([]);
  const [rules, setRules] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);

  // New Rule Form
  const [showAddRule, setShowAddRule] = useState(false);
  const [newRuleIp, setNewRuleIp] = useState("");
  const [newRuleType, setNewRuleType] = useState<"blacklist" | "whitelist">("blacklist");
  const [newRuleNotes, setNewRuleNotes] = useState("");

  const fetchData = async () => {
    setLoading(true);
    try {
      if (activeTab === "logs") {
        const data = await api.security.getAccessLogs(200);
        setLogs(data);
      } else {
        const data = await api.security.getRules();
        setRules(data);
      }
    } catch (e) {
      console.error("Failed to fetch security data:", e);
    }
    setLoading(false);
  };

  useEffect(() => {
    fetchData();
    let interval: any;
    if (activeTab === "logs") {
      // 实时刷新流水日志
      interval = setInterval(fetchData, 5000);
    }
    return () => clearInterval(interval);
  }, [activeTab]);

  const handleClearLogs = async () => {
    if (!confirm("Are you sure you want to clear all proxy access logs?")) return;
    await api.security.clearAccessLogs();
    fetchData();
  };

  const handleAddRule = async () => {
    if (!newRuleIp.trim()) return;
    try {
      await api.security.addRule(newRuleIp.trim(), newRuleType, newRuleNotes);
      setNewRuleIp("");
      setNewRuleNotes("");
      setShowAddRule(false);
      fetchData();
    } catch (e: any) {
      alert("Failed to add rule: " + e.toString());
    }
  };

  const handleDeleteRule = async (id: string) => {
    if (!confirm("Remove this rule?")) return;
    await api.security.deleteRule(id);
    fetchData();
  };

  const handleToggleRule = async (id: string, current: boolean) => {
    await api.security.toggleRule(id, !current);
    fetchData();
  };

  const getActionColor = (action: string) => {
    switch(action.toLowerCase()) {
      case "allow": return "var(--color-success)";
      case "deny": return "var(--color-error)";
      case "rate_limit": return "var(--color-warning)";
      case "blacklisted": return "var(--color-error)";
      default: return "var(--color-text-muted)";
    }
  };

  return (
    <div className="proxy-container">
      <header className="proxy-header flex-row">
        <div>
          <h1 className="proxy-title"><Shield size={22} className="text-primary" /> 哨站风控台 (Security Gateway)</h1>
          <p className="proxy-subtitle">Monitor proxy inbound traffic and manage IP access limits dynamically.</p>
        </div>
        <div style={{ display: "flex", gap: 12 }}>
          <button className="btn btn-secondary" onClick={fetchData}>
            <RefreshCw size={14} className={loading && activeTab !== "logs" ? "animate-spin" : ""} /> 
            刷新探测
          </button>
        </div>
      </header>

      {/* TABS */}
      <div style={{ display: "flex", gap: 16, borderBottom: "1px solid var(--color-border)", marginBottom: 24, paddingBottom: 12 }}>
        <button 
          className={`btn btn-sm ${activeTab === "logs" ? "btn-primary" : "btn-secondary"}`}
          onClick={() => setActiveTab("logs")}
          style={{ background: activeTab === "logs" ? "var(--color-primary-alpha)" : "transparent", border: "none" }}
        >
          <FileText size={16} /> 流水探测 (Access Logs)
        </button>
        <button 
          className={`btn btn-sm ${activeTab === "rules" ? "btn-primary" : "btn-secondary"}`}
          onClick={() => setActiveTab("rules")}
          style={{ background: activeTab === "rules" ? "var(--color-primary-alpha)" : "transparent", border: "none" }}
        >
          <Lock size={16} /> 极客黑白名单 (IP Rules Matrix)
        </button>
      </div>

      {activeTab === "logs" && (
        <div className="proxy-card fade-in">
          <div className="proxy-card-header" style={{ display: "flex", justifyContent: "space-between", marginBottom: 16 }}>
            <div>
               <Activity size={18} className="text-primary" style={{ marginRight: 8 }}/>
               实时拦截流水 (Realtime Intercept Logs)
            </div>
            <button className="btn btn-secondary btn-sm" onClick={handleClearLogs}><Trash2 size={14} /> 清空记录</button>
          </div>

          <div style={{ background: "var(--color-bg-primary)", borderRadius: 8, padding: 12, border: "1px solid var(--color-border)", minHeight: 300, maxHeight: 600, overflowY: "auto" }}>
            {logs.length === 0 ? (
              <div style={{ textAlign: "center", padding: "40px", color: "var(--color-text-muted)" }}>暂无访问记录 (No access logs)</div>
            ) : (
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
                <thead>
                  <tr style={{ borderBottom: "1px solid var(--color-border)", color: "var(--color-text-muted)", textAlign: "left" }}>
                    <th style={{ padding: "8px 12px" }}>Time</th>
                    <th style={{ padding: "8px 12px" }}>IP Address</th>
                    <th style={{ padding: "8px 12px" }}>Token Bind</th>
                    <th style={{ padding: "8px 12px" }}>Action</th>
                    <th style={{ padding: "8px 12px" }}>Reason</th>
                  </tr>
                </thead>
                <tbody>
                  {logs.map((L) => (
                    <tr key={L.id} style={{ borderBottom: "1px solid var(--color-border-light)" }}>
                       <td style={{ padding: "12px 12px", color: "var(--color-text-secondary)" }}>
                         {new Date(L.createdAt * 1000).toLocaleTimeString()}
                       </td>
                       <td style={{ padding: "12px 12px", fontFamily: "monospace" }}>{L.ipAddress}</td>
                       <td style={{ padding: "12px 12px", color: "var(--color-text-muted)" }}>{L.tokenId ? `${L.tokenId.slice(0, 8)}...` : "—"}</td>
                       <td style={{ padding: "12px 12px" }}>
                         <span style={{ 
                            background: getActionColor(L.actionTaken) + "20", 
                            color: getActionColor(L.actionTaken),
                            padding: "2px 6px", borderRadius: 4, fontWeight: 600, fontSize: 11, textTransform: "uppercase"
                          }}>
                           {L.actionTaken}
                         </span>
                       </td>
                       <td style={{ padding: "12px 12px", color: "var(--color-text-secondary)", fontSize: 12 }}>{L.reason || "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </div>
      )}

      {activeTab === "rules" && (
        <div className="proxy-card fade-in">
          <div className="proxy-card-header" style={{ display: "flex", justifyContent: "space-between", marginBottom: 16 }}>
            <div>
               <Shield size={18} className="text-primary" style={{ marginRight: 8 }}/>
               高维防御界限 (Firewall Rules)
            </div>
            <button className="btn btn-primary btn-sm" onClick={() => setShowAddRule(!showAddRule)}>
              {showAddRule ? "取消添加" : <><Plus size={14} /> 新增规则界限</>}
            </button>
          </div>

          <div style={{ marginBottom: 16, fontSize: 13, color: "var(--color-text-secondary)", borderLeft: "3px solid var(--color-primary)", paddingLeft: 12 }}>
            当设定了<strong>白名单</strong>时，所有未记录的外部 IP 均会自动被默认拒载；黑名单具有全球最高裁决优先级。可以使用 * 作为 IP 后缀通配。
          </div>

          {showAddRule && (
            <div style={{ background: "rgba(15, 23, 42, 0.05)", border: "1px solid var(--color-border)", borderRadius: 8, padding: 16, marginBottom: 24 }}>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 2fr auto", gap: 12, alignItems: "end" }}>
                <div>
                   <label style={{ display: "block", fontSize: 12, marginBottom: 4 }}>策略判定 (Rule Type)</label>
                   <select className="form-select" style={{ width: "100%" }} value={newRuleType} onChange={(e) => setNewRuleType(e.target.value as any)}>
                     <option value="blacklist">💀 数据黑洞 (Blacklist)</option>
                     <option value="whitelist">🟢 绝对放行 (Whitelist)</option>
                   </select>
                </div>
                <div>
                   <label style={{ display: "block", fontSize: 12, marginBottom: 4 }}>目标 IP (CIDR / IP)</label>
                   <input type="text" className="form-input" placeholder="例如: 192.168.1.*" value={newRuleIp} onChange={e => setNewRuleIp(e.target.value)} style={{ width: "100%" }}/>
                </div>
                <div>
                   <label style={{ display: "block", fontSize: 12, marginBottom: 4 }}>战术命名 (Notes)</label>
                   <input type="text" className="form-input" placeholder="例如: 恶意爬虫群" value={newRuleNotes} onChange={e => setNewRuleNotes(e.target.value)} style={{ width: "100%" }}/>
                </div>
                <div>
                   <button className="btn btn-primary" onClick={handleAddRule} disabled={!newRuleIp.trim()}>强制下发</button>
                </div>
              </div>
            </div>
          )}

          <div style={{ display: "grid", gap: 12 }}>
            {rules.length === 0 ? (
               <div style={{ textAlign: "center", padding: "40px", color: "var(--color-text-muted)", border: "1px dashed var(--color-border)", borderRadius: 8 }}>暂无建立自定义界限 (No custom firewall rules)</div>
            ) : rules.map((r) => (
              <div key={r.id} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: 16, borderRadius: 8, border: "1px solid var(--color-border)", background: "var(--color-bg-primary)" }}>
                <div style={{ display: "flex", gap: 16, alignItems: "center" }}>
                   <div style={{ 
                      width: 4, height: 24, borderRadius: 2,
                      background: r.ruleType === "whitelist" ? "var(--color-success)" : "var(--color-error)" 
                   }} />
                   <div>
                     <div style={{ display: "flex", alignItems: "center", gap: 8, fontFamily: "monospace", fontSize: 15, fontWeight: 600 }}>
                        {r.ipCidr}
                        {!r.isActive && <span style={{ fontSize: 10, padding: "2px 4px", background: "var(--color-bg-secondary)", borderRadius: 4 }}>OFFLINE</span>}
                     </div>
                     <div style={{ fontSize: 12, color: "var(--color-text-muted)" }}>{r.ruleType.toUpperCase()} - {r.notes || "No particular notes"}</div>
                   </div>
                </div>
                
                <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
                   <div className="toggle-switch">
                      <input type="checkbox" id={`tg_${r.id}`} checked={r.isActive} onChange={() => handleToggleRule(r.id, r.isActive)} />
                      <label htmlFor={`tg_${r.id}`}></label>
                   </div>
                   <button className="btn btn-secondary btn-sm" onClick={() => handleDeleteRule(r.id)} style={{ padding: 6, color: "var(--color-error)" }}>
                      <Trash2 size={16} />
                   </button>
                </div>
              </div>
            ))}
          </div>

        </div>
      )}
    </div>
  );
}
