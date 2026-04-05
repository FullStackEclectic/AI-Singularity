import { useState, useEffect } from "react";
import { api } from "../../lib/api";
import { Shield, Key, Network, Users, Clock, AlertTriangle, Plus, Trash2, Power, Eye, EyeOff, Save, X } from "lucide-react";
import "./SharingPage.css";

export default function SharingPage() {
  const [tokens, setTokens] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  
  // Form State
  const [username, setUsername] = useState("");
  const [description, setDescription] = useState("");
  const [expiresType, setExpiresType] = useState("never");
  const [expiresDays, setExpiresDays] = useState("7");
  const [maxIps, setMaxIps] = useState("0");
  const [curfewStart, setCurfewStart] = useState("");
  const [curfewEnd, setCurfewEnd] = useState("");

  const [visibleTokens, setVisibleTokens] = useState<Record<string, boolean>>({});

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
      const req = {
        username,
        description: description || null,
        expires_type: expiresType,
        expires_at: expAt,
        max_ips: parseInt(maxIps, 10) || 0,
        curfew_start: curfewStart || null,
        curfew_end: curfewEnd || null,
      };
      await api.userTokens.create(req);
      setShowForm(false);
      setUsername("");
      setDescription("");
      fetchTokens();
    } catch (e) {
      console.error("Failed to create user token", e);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("确定要收回这个 Token 吗？使用该 Token 的设备将立刻失去连接。")) return;
    try {
      await api.userTokens.delete(id);
      fetchTokens();
    } catch (e) {
      console.error("Delete failed", e);
    }
  };

  const handleToggleEnabled = async (token: any) => {
    try {
      await api.userTokens.update({
        id: token.id,
        enabled: !token.enabled
      });
      fetchTokens();
    } catch (e) {
      console.error("Toggle enabled failed", e);
    }
  };

  const formatTimestamp = (ts: number | null | undefined) => {
    if (!ts) return "-";
    return new Date(ts * 1000).toLocaleString();
  };

  return (
    <div className="sharing-page">
      <header className="page-header">
        <div className="header-left">
          <Shield className="header-icon" />
          <h1>分享与下发</h1>
        </div>
        <button className="primary-button" onClick={() => setShowForm(!showForm)}>
          <Plus size={16} /> 创建分享 Token
        </button>
      </header>

      {showForm && (
        <div className="creation-panel">
          <h3><Key size={16}/> 颁发新访问权</h3>
          <div className="form-grid">
            <div className="form-group">
              <label>使用者名称</label>
              <input value={username} onChange={e => setUsername(e.target.value)} placeholder="如：我的朋友小明" />
            </div>
            <div className="form-group">
              <label>备注描述</label>
              <input value={description} onChange={e => setDescription(e.target.value)} placeholder="用途说明..." />
            </div>
            <div className="form-group">
              <label>过期机制</label>
              <select value={expiresType} onChange={e => setExpiresType(e.target.value)}>
                <option value="never">永久有效</option>
                <option value="relative">按创建后相对天数</option>
              </select>
            </div>
            {expiresType === "relative" && (
              <div className="form-group">
                <label>有效期 (天)</label>
                <input type="number" value={expiresDays} onChange={e => setExpiresDays(e.target.value)} />
              </div>
            )}
            <div className="form-group">
              <label>最大绑定IP (0为不限制)</label>
              <input type="number" value={maxIps} onChange={e => setMaxIps(e.target.value)} />
            </div>
            <div className="form-group row-group">
              <div className="flex-1">
                <label>不可用开始时间 (HH:MM)</label>
                <input type="time" value={curfewStart} onChange={e => setCurfewStart(e.target.value)} />
              </div>
              <div className="flex-1">
                <label>不可用结束时间 (HH:MM)</label>
                <input type="time" value={curfewEnd} onChange={e => setCurfewEnd(e.target.value)} />
              </div>
            </div>
          </div>
          <div className="form-actions">
            <button className="secondary-button" onClick={() => setShowForm(false)}>
              <X size={14} /> 取消
            </button>
            <button className="primary-button" onClick={handleCreate}>
              <Save size={14} /> 创建颁发
            </button>
          </div>
        </div>
      )}

      {loading ? (
        <div className="loading-state">
          <div className="spinner"></div>
          <span>加载数据中...</span>
        </div>
      ) : (
        <div className="token-list">
          {tokens.map((token) => (
            <div key={token.id} className={`token-card ${token.enabled ? 'active' : 'suspended'}`}>
              <div className="card-header">
                <div className="user-info">
                  <Users size={18} className="user-icon" />
                  <span className="username">{token.username}</span>
                  {token.description && <span className="description">- {token.description}</span>}
                </div>
                <div className="card-actions">
                  <button onClick={() => handleToggleEnabled(token)} className={`status-toggle ${token.enabled ? 'on' : 'off'}`} title={token.enabled ? "冻结" : "解冻"}>
                    <Power size={18} />
                  </button>
                  <button onClick={() => handleDelete(token.id)} className="delete-btn" title="回收撤销">
                    <Trash2 size={18} />
                  </button>
                </div>
              </div>
              
              <div className="card-body">
                <div className="token-field">
                  <span className="label">下发 Token:</span>
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
                    <span className="metric-val">{token.max_ips === 0 ? '不限 IP' : `限制 ${token.max_ips} IP`}</span>
                  </div>
                  {(token.curfew_start || token.curfew_end) && (
                    <div className="metric warn">
                      <span className="metric-icon"><AlertTriangle size={14}/></span>
                      <span className="metric-val">此时不可用: {token.curfew_start || '00:00'} - {token.curfew_end || '23:59'}</span>
                    </div>
                  )}
                  <div className="metric">
                    <span className="metric-icon"><Clock size={14}/></span>
                    <span className="metric-val">创建于: {formatTimestamp(token.created_at)}</span>
                  </div>
                </div>

                <div className="usage-stats">
                  <div className="stat-pill">
                    <span className="label">请求次数</span>
                    <span className="value">{token.total_requests}</span>
                  </div>
                  <div className="stat-pill highlight">
                    <span className="label">Token 消耗量</span>
                    <span className="value">{token.total_tokens_used.toLocaleString()}</span>
                  </div>
                  <div className="stat-pill">
                    <span className="label">近期活跃时间</span>
                    <span className="value">
                      {token.last_used_at ? formatTimestamp(token.last_used_at) : '暂未连接'}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          ))}
          {tokens.length === 0 && (
            <div className="empty-state">
              <Network size={48} className="empty-icon" />
              <p>暂无下线分享记录</p>
              <span>通过下发受控 Token，您可以轻松把算力分享给其他程序或朋友。</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
