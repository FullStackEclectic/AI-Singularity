import { useState, useEffect, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { Shield, Key, Network, Users, Clock, AlertTriangle, Plus, Trash2, Power, Eye, EyeOff, Save, X, Globe, Layers, Tag as TagIcon, Zap } from "lucide-react";
import { PLATFORM_LABELS } from "../../types";
import "./SharingPage.css";

type ScopeType = "global" | "channel" | "tag" | "single";

export default function SharingPage() {
  const [tokens, setTokens] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [actionMessage, setActionMessage] = useState("");
  const [confirmRevokeId, setConfirmRevokeId] = useState<string | null>(null);
  const [revoking, setRevoking] = useState(false);
  
  // ---------- Form State ----------
  const [username, setUsername] = useState("");
  const [description, setDescription] = useState("");
  const [expiresType, setExpiresType] = useState("never");
  const [expiresDays, setExpiresDays] = useState("7");
  const [maxIps, setMaxIps] = useState("0");
  const [curfewStart, setCurfewStart] = useState("");
  const [curfewEnd, setCurfewEnd] = useState("");

  // Scope State
  const [scopeType, setScopeType] = useState<ScopeType>("global");
  const [selectedChannels, setSelectedChannels] = useState<string[]>([]);
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  // single scope uses a text input or comes from url/props (simulated here)
  const [singleAccountId, setSingleAccountId] = useState("");

  const [visibleTokens, setVisibleTokens] = useState<Record<string, boolean>>({});

  // Fetch all accounts to extract available Channels and Tags
  const { data: keys = [] } = useQuery({ queryKey: ["keys"], queryFn: api.keys.list });
  const { data: ideAccs = [] } = useQuery({ queryKey: ["ideAccounts"], queryFn: api.ideAccounts.list });

  const { availableChannels, availableTags } = useMemo(() => {
    const chSet = new Set<string>();
    const tagSet = new Set<string>();
    
    keys.forEach(k => {
      chSet.add(`api_${k.platform}`);
      k.tags?.forEach((t: string) => tagSet.add(t));
    });
    ideAccs.forEach(a => {
      chSet.add(`ide_${a.origin_platform}`);
      a.tags?.forEach((t: string) => tagSet.add(t));
    });

    return {
      availableChannels: Array.from(chSet).sort(),
      availableTags:     Array.from(tagSet).sort()
    };
  }, [keys, ideAccs]);

  const fetchTokens = async () => {
    try {
      setLoading(true);
      const data = await api.userTokens.list();
      setTokens(data);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchTokens();
  }, []);

  const toggleTokenVisibility = (id: string) => {
    setVisibleTokens(prev => ({ ...prev, [id]: !prev[id] }));
  };

  const calculateExpiresAt = () => {
    if (expiresType === "never") return null;
    if (expiresType === "relative") {
       const days = parseInt(expiresDays, 10) || 7;
       return days * 24 * 3600;
    }
    return null;
  };

  const handleCreate = async () => {
    if (!username.trim()) return;
    try {
      const expAt = calculateExpiresAt();
      // Magic Payload Injection: Store Scope config inside JSON wrapped Description
      const scopePayload = {
        desc: description,
        scope: scopeType,
        channels: selectedChannels,
        tags: selectedTags,
        single_account: singleAccountId
      };
      
      const req = {
        username,
        // Backend doesn't support scope columns yet, packing it strictly into structured description!
        description: JSON.stringify(scopePayload),
        expires_type: expiresType,
        expires_at: expAt,
        max_ips: parseInt(maxIps, 10) || 0,
        curfew_start: curfewStart || null,
        curfew_end: curfewEnd || null,
      };
      
      await api.userTokens.create(req);
      setShowForm(false);
      setUsername(""); setDescription(""); setScopeType("global");
      setSelectedChannels([]); setSelectedTags([]); setSingleAccountId("");
      fetchTokens();
    } catch (e) {
      console.error("Failed to create user token", e);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      setRevoking(true);
      await api.userTokens.delete(id);
      setActionMessage("Token 已收回");
      setConfirmRevokeId(null);
      fetchTokens();
    } catch (e) {
      console.error("Delete failed", e);
      setActionMessage("收回失败: " + String(e));
    } finally {
      setRevoking(false);
    }
  };

  const handleToggleEnabled = async (token: any) => {
    try {
      await api.userTokens.update({ id: token.id, enabled: !token.enabled });
      fetchTokens();
    } catch (e) {
      console.error("Toggle enabled failed", e);
    }
  };

  const formatTimestamp = (ts: number | null | undefined) => {
    if (!ts) return "-";
    return new Date(ts * 1000).toLocaleString();
  };

  // Helper to safely parse scoped descriptions
  const parseTokenMeta = (rawDesc: string | null | undefined) => {
    if (!rawDesc) return { desc: "", scope: "global" };
    try {
      if (rawDesc.startsWith("{") && rawDesc.includes('"scope"')) {
        return JSON.parse(rawDesc);
      }
    } catch {}
    return { desc: rawDesc, scope: "global" };
  };

  const toggleArrayItem = (arr: string[], setArr: any, item: string) => {
    if (arr.includes(item)) setArr(arr.filter(i => i !== item));
    else setArr([...arr, item]);
  };

  return (
    <div className="sharing-page">
      <header className="page-header">
        <div className="header-left">
          <Shield className="header-icon" />
          <h1>分享与下发网关</h1>
        </div>
        <button className="primary-button" onClick={() => setShowForm(!showForm)}>
          <Plus size={16} /> 创建中转分发 Token
        </button>
      </header>

      {actionMessage && (
        <div className="sharing-message-bar">
          {actionMessage}
        </div>
      )}

      {showForm && (
        <div className="creation-panel">
          <h3><Key size={16}/> 生成受控访问权</h3>
          
          {/* ========== 新增：四维管辖域配置板块 ========== */}
          <div className="scope-config-zone">
            <h4>1. 定义算力暴露范围 (Scope)</h4>
            <div className="scope-selection-pills">
              <button className={`scope-pill ${scopeType === 'global' ? 'active' : ''}`} onClick={() => setScopeType('global')}>
                <Globe size={14}/> 全局无差别共享
              </button>
              <button className={`scope-pill ${scopeType === 'channel' ? 'active' : ''}`} onClick={() => setScopeType('channel')}>
                <Layers size={14}/> 绑定渠道池
              </button>
              <button className={`scope-pill ${scopeType === 'tag' ? 'active' : ''}`} onClick={() => setScopeType('tag')}>
                <TagIcon size={14}/> 绑定自定义标签集
              </button>
              <button className={`scope-pill ${scopeType === 'single' ? 'active' : ''}`} onClick={() => setScopeType('single')}>
                <Zap size={14}/> 单点透传共享
              </button>
            </div>

            <div className="scope-details-form">
              {scopeType === "channel" && (
                <div className="multi-select-wrap">
                  <label>请选择允许访问的渠道源：</label>
                  <div className="chip-list">
                    {availableChannels.map(ch => (
                      <div key={ch} className={`chip ${selectedChannels.includes(ch) ? 'selected' : ''}`} onClick={() => toggleArrayItem(selectedChannels, setSelectedChannels, ch)}>
                        {ch.includes('api_') ? PLATFORM_LABELS[ch.replace('api_','') as keyof typeof PLATFORM_LABELS] || ch : ch}
                      </div>
                    ))}
                    {availableChannels.length === 0 && <span className="text-muted">当前系统暂无任何接入渠道...</span>}
                  </div>
                </div>
              )}
              {scopeType === "tag" && (
                <div className="multi-select-wrap">
                  <label>仅允许导流到拥有以下任一标签的账号上：</label>
                  <div className="chip-list">
                    {availableTags.map(tag => (
                      <div key={tag} className={`chip ${selectedTags.includes(tag) ? 'selected' : ''}`} onClick={() => toggleArrayItem(selectedTags, setSelectedTags, tag)}>
                        {tag}
                      </div>
                    ))}
                    {availableTags.length === 0 && <span className="text-muted">当前系统并未含有标签聚合账号...</span>}
                  </div>
                </div>
              )}
              {scopeType === "single" && (
                <div className="form-group" style={{maxWidth: '400px'}}>
                  <label>透传目标 Account ID (精准一对一映射):</label>
                  <input value={singleAccountId} onChange={e => setSingleAccountId(e.target.value)} placeholder="粘贴指定的凭证底层系统 ID..." />
                </div>
              )}
              {scopeType === "global" && (
                 <p className="text-muted" style={{fontSize: '0.8rem', paddingLeft: '4px'}}>*警告：该分享将获得网关等同的全局漫游权限，一旦调用则根据优先级消耗任意高能资库。</p>
              )}
            </div>
          </div>
          {/* ==================================== */}

          <div className="form-grid" style={{marginTop: '1.5rem', borderTop: '1px dashed var(--border-color)', paddingTop: '1.5rem'}}>
            <div className="form-group row-group" style={{ display: 'flex', gap: '1rem', gridColumn: '1 / -1' }}>
              <div className="flex-1">
                <label>使用者标识</label>
                <input value={username} onChange={e => setUsername(e.target.value)} placeholder="如：我的朋友小明或产品A集成" />
              </div>
              <div className="flex-1">
                <label>业务备注</label>
                <input value={description} onChange={e => setDescription(e.target.value)} placeholder="用途说明..." />
              </div>
            </div>
            
            <div className="form-group">
              <label>风控：凭证生命周期</label>
              <select value={expiresType} onChange={e => setExpiresType(e.target.value)}>
                <option value="never">永久有效驻留</option>
                <option value="relative">按天数自毁</option>
              </select>
            </div>
            {expiresType === "relative" && (
              <div className="form-group">
                <label>自毁倒数 (天)</label>
                <input type="number" value={expiresDays} onChange={e => setExpiresDays(e.target.value)} />
              </div>
            )}
            <div className="form-group">
              <label>并发锁：最大绑定IP</label>
              <input type="number" value={maxIps} onChange={e => setMaxIps(e.target.value)} placeholder="0 为放任不限制" />
            </div>
            <div className="form-group row-group">
              <div className="flex-1">
                <label>宵禁闭环：不可用始于 (HH:MM)</label>
                <input type="time" value={curfewStart} onChange={e => setCurfewStart(e.target.value)} />
              </div>
              <div className="flex-1">
                <label>不可用止于 (HH:MM)</label>
                <input type="time" value={curfewEnd} onChange={e => setCurfewEnd(e.target.value)} />
              </div>
            </div>
          </div>
          
          <div className="form-actions">
            <button className="secondary-button" onClick={() => setShowForm(false)}>
              <X size={14} /> 放弃放弃
            </button>
            <button className="primary-button" onClick={handleCreate}>
              <Save size={14} /> 强力签发 Token
            </button>
          </div>
        </div>
      )}

      {loading ? (
        <div className="loading-state">
          <div className="spinner"></div>
          <span>读取海量签发记录中...</span>
        </div>
      ) : (
        <div className="token-list">
          {tokens.map((token) => {
            const meta = parseTokenMeta(token.description);
            return (
            <div key={token.id} className={`token-card ${token.enabled ? 'active' : 'suspended'}`}>
              <div className="card-header">
                <div className="user-info">
                  <Users size={18} className="user-icon" />
                  <div>
                    <span className="username">{token.username}</span>
                    {meta.desc && <div className="description text-muted">{meta.desc}</div>}
                  </div>
                </div>
                <div className="card-actions">
                  <button onClick={() => handleToggleEnabled(token)} className={`status-toggle ${token.enabled ? 'on' : 'off'}`} title={token.enabled ? "拔掉网线" : "恢复供应"}>
                    <Power size={18} />
                  </button>
                  <button onClick={() => setConfirmRevokeId(token.id)} className="delete-btn" title="核平销毁">
                    <Trash2 size={18} />
                  </button>
                </div>
              </div>
              
              <div className="card-body">
                {/* 核心亮点：范围标识器 */}
                <div className="scope-badge-container">
                  {meta.scope === "global" && <span className="scope-tag global"><Globe size={12}/> 全局漫游</span>}
                  {meta.scope === "channel" && <span className="scope-tag restricted"><Layers size={12}/> 局限于 {(meta.channels?.length || 0)} 个渠道面</span>}
                  {meta.scope === "tag" && <span className="scope-tag restricted"><TagIcon size={12}/> 锁定于 {(meta.tags?.length || 0)} 个特辑聚类</span>}
                  {meta.scope === "single" && <span className="scope-tag super-restricted"><Zap size={12}/> 单点直连隧道</span>}
                </div>

                <div className="token-field">
                  <div className="token-value-wrap">
                    <code>{visibleTokens[token.id] ? token.token : "sk-ag-••••••••••••••••••••••••••••"}</code>
                    <button onClick={() => toggleTokenVisibility(token.id)}>
                      {visibleTokens[token.id] ? <EyeOff size={14} /> : <Eye size={14} />}
                    </button>
                  </div>
                </div>

                <div className="metrics-grid">
                  <div className="metric">
                    <span className="metric-icon"><Network size={14}/></span>
                    <span className="metric-val">{token.max_ips === 0 ? '不限网络拓扑' : `风控 ${token.max_ips} IP`}</span>
                  </div>
                  {(token.curfew_start || token.curfew_end) && (
                    <div className="metric warn">
                      <span className="metric-icon"><AlertTriangle size={14}/></span>
                      <span className="metric-val">断电期: {token.curfew_start || '00:00'} - {token.curfew_end || '23:59'}</span>
                    </div>
                  )}
                  <div className="metric" style={{gridColumn: '1 / -1'}}>
                    <span className="metric-icon"><Clock size={14}/></span>
                    <span className="metric-val">熔铸于: {formatTimestamp(token.created_at)}</span>
                  </div>
                </div>

                <div className="usage-stats">
                  <div className="stat-pill">
                    <span className="label">请求冲击波</span>
                    <span className="value">{token.total_requests}</span>
                  </div>
                  <div className="stat-pill highlight">
                    <span className="label">燃油散尽量(Tokens)</span>
                    <span className="value">{token.total_tokens_used.toLocaleString()}</span>
                  </div>
                  <div className="stat-pill">
                    <span className="label">末次心跳脉冲</span>
                    <span className="value">
                      {token.last_used_at ? formatTimestamp(token.last_used_at) : '暂未建连'}
                    </span>
                  </div>
                </div>
              </div>
            </div>
            );
          })}
          {tokens.length === 0 && (
            <div className="empty-state">
              <Zap size={48} className="empty-icon" />
              <p>中转网关静待指令</p>
              <span>目前没有任何对外建立的安全隧道。创建你的第一把私有化钥匙。</span>
            </div>
          )}
        </div>
      )}

      {confirmRevokeId && (
        <div className="sharing-modal-overlay" onClick={() => !revoking && setConfirmRevokeId(null)}>
          <div className="sharing-modal" onClick={(e) => e.stopPropagation()}>
            <h3>收回 Token</h3>
            <p>确定要收回这个 Token 吗？使用该 Token 的设备将立刻失去连接。</p>
            <div className="sharing-modal-actions">
              <button className="secondary-button" onClick={() => setConfirmRevokeId(null)} disabled={revoking}>
                取消
              </button>
              <button className="danger-button" onClick={() => handleDelete(confirmRevokeId)} disabled={revoking}>
                {revoking ? "收回中..." : "确认收回"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
