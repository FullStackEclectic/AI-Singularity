import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../lib/api";
import type { ApiKey, Balance, Platform } from "../../types";
import { PLATFORM_LABELS, STATUS_LABELS } from "../../types";
import "./KeysPage.css";

const PLATFORMS: { value: Platform; label: string }[] = [
  { value: "open_ai", label: "OpenAI" },
  { value: "anthropic", label: "Anthropic (Claude)" },
  { value: "gemini", label: "Google Gemini" },
  { value: "deep_seek", label: "DeepSeek" },
  { value: "aliyun", label: "阿里云百炼" },
  { value: "bytedance", label: "字节豆包" },
  { value: "moonshot", label: "Moonshot (Kimi)" },
  { value: "zhipu", label: "智谱 GLM" },
  { value: "custom", label: "自定义接口" },
];

export default function KeysPage() {
  const qc = useQueryClient();
  const [showAdd, setShowAdd] = useState(false);

  const { data: keys = [], isLoading } = useQuery({
    queryKey: ["keys"],
    queryFn: api.keys.list,
  });

  const { data: balances = [] } = useQuery({
    queryKey: ["balances"],
    queryFn: api.balance.listAll,
    staleTime: 1000 * 60 * 5,
  });

  const balanceMap = Object.fromEntries(balances.map((b) => [b.key_id, b]));

  const deleteMut = useMutation({
    mutationFn: api.keys.delete,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["keys"] }),
  });

  const checkMut = useMutation({
    mutationFn: api.keys.check,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["keys"] });
      qc.invalidateQueries({ queryKey: ["dashboard-stats"] });
    },
  });

  const refreshBalanceMut = useMutation({
    mutationFn: (id: string) => api.balance.refreshOne(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["balances"] }),
  });

  const validCount = keys.filter((k) => k.status === "valid").length;
  const invalidCount = keys.filter(
    (k) => k.status === "invalid" || k.status === "banned" || k.status === "expired"
  ).length;

  return (
    <div className="keys-page">
      <div className="page-header">
        <div>
          <h1 className="page-title">API Keys</h1>
          <p className="page-subtitle">
            {keys.length} 个账号
            {validCount > 0 && (
              <> · <span className="text-success">{validCount} 有效</span></>
            )}
            {invalidCount > 0 && (
              <> · <span className="text-danger">{invalidCount} 异常</span></>
            )}
          </p>
        </div>
        <div style={{ display: "flex", gap: "var(--space-3)" }}>
          <button
            className="btn btn-ghost"
            onClick={() => keys.forEach((k) => checkMut.mutate(k.id))}
            disabled={keys.length === 0 || checkMut.isPending}
          >
            ⟳ 全部检测
          </button>
          <button className="btn btn-primary" onClick={() => setShowAdd(true)}>
            ＋ 添加 Key
          </button>
        </div>
      </div>

      <div className="keys-body">
        {isLoading ? (
          <div className="empty-state">
            <div className="animate-spin" style={{ fontSize: 24 }}>⟳</div>
            <span>加载中...</span>
          </div>
        ) : keys.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">🔑</div>
            <h3 style={{ color: "var(--color-text-secondary)" }}>还没有 API Key</h3>
            <p>点击「添加 Key」开始配置</p>
            <button className="btn btn-primary" onClick={() => setShowAdd(true)}>
              ＋ 添加第一个 Key
            </button>
          </div>
        ) : (
          <div className="keys-list">
            {keys.map((key) => (
              <KeyCard
                key={key.id}
                apiKey={key}
                balance={balanceMap[key.id]}
                onDelete={() => deleteMut.mutate(key.id)}
                onCheck={() => checkMut.mutate(key.id)}
                onRefreshBalance={() => refreshBalanceMut.mutate(key.id)}
                isChecking={checkMut.isPending}
                isRefreshingBalance={
                  refreshBalanceMut.isPending &&
                  (refreshBalanceMut.variables as string) === key.id
                }
              />
            ))}
          </div>
        )}
      </div>

      {showAdd && (
        <AddKeyModal
          onClose={() => setShowAdd(false)}
          onSuccess={() => {
            setShowAdd(false);
            qc.invalidateQueries({ queryKey: ["keys"] });
            qc.invalidateQueries({ queryKey: ["dashboard-stats"] });
          }}
        />
      )}
    </div>
  );
}

function KeyCard({
  apiKey,
  balance,
  onDelete,
  onCheck,
  onRefreshBalance,
  isChecking,
  isRefreshingBalance,
}: {
  apiKey: ApiKey;
  balance?: Balance;
  onDelete: () => void;
  onCheck: () => void;
  onRefreshBalance: () => void;
  isChecking: boolean;
  isRefreshingBalance: boolean;
}) {
  const statusClass =
    apiKey.status === "valid" ? "valid" :
    apiKey.status === "banned" ? "banned" :
    apiKey.status === "invalid" || apiKey.status === "expired" ? "invalid" :
    isChecking ? "checking" : "unknown";

  const badgeClass =
    apiKey.status === "valid" ? "badge-success" :
    apiKey.status === "banned" || apiKey.status === "invalid" || apiKey.status === "expired"
      ? "badge-danger"
      : apiKey.status === "rate_limit" ? "badge-warning" : "badge-muted";

  // 格式化余额显示
  const balanceText = (() => {
    if (!balance) return null;
    if (balance.balance_usd != null) return `$${balance.balance_usd.toFixed(2)}`;
    if (balance.balance_cny != null) return `¥${balance.balance_cny.toFixed(2)}`;
    return null;
  })();

  return (
    <div className="key-card card animate-fade-in">
      <div className="key-card-header">
        <div className="key-card-info">
          <span className={`status-dot ${statusClass}`} />
          <div>
            <div className="key-card-name">{apiKey.name}</div>
            <div className="key-card-platform text-muted">
              {PLATFORM_LABELS[apiKey.platform]}
            </div>
          </div>
        </div>

        <div className="key-card-actions">
          {/* 余额显示 */}
          {balanceText && (
            <span className="balance-tag">
              {balanceText}
            </span>
          )}

          <span className={`badge ${badgeClass}`}>
            {isChecking ? "检测中..." : STATUS_LABELS[apiKey.status]}
          </span>

          {/* 刷新余额 */}
          <button
            className="btn btn-ghost btn-sm btn-icon"
            onClick={onRefreshBalance}
            disabled={isRefreshingBalance}
            title="刷新余额"
          >
            {isRefreshingBalance ? <span className="animate-spin">⟳</span> : "💰"}
          </button>

          {/* 检测有效性 */}
          <button
            className="btn btn-ghost btn-sm btn-icon"
            onClick={onCheck}
            disabled={isChecking}
            title="检测有效性"
          >
            {isChecking ? <span className="animate-spin">⟳</span> : "⟳"}
          </button>

          {/* 删除 */}
          <button
            className="btn btn-danger btn-sm btn-icon"
            onClick={onDelete}
            title="删除"
          >
            ✕
          </button>
        </div>
      </div>

      <div className="key-card-preview font-mono text-muted">
        {apiKey.key_preview}
      </div>

      {apiKey.base_url && (
        <div className="key-card-url text-muted">{apiKey.base_url}</div>
      )}

      {/* 备注 */}
      {apiKey.notes && (
        <div className="key-card-notes text-muted">{apiKey.notes}</div>
      )}
    </div>
  );
}

function AddKeyModal({
  onClose,
  onSuccess,
}: {
  onClose: () => void;
  onSuccess: () => void;
}) {
  const [form, setForm] = useState({
    name: "",
    platform: "open_ai" as Platform,
    secret: "",
    base_url: "",
    notes: "",
  });
  const [error, setError] = useState("");

  const addMut = useMutation({
    mutationFn: api.keys.add,
    onSuccess: () => onSuccess(),
    onError: (e: unknown) => setError(String(e)),
  });

  const showBaseUrl = form.platform === "custom";

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!form.name.trim() || !form.secret.trim()) {
      setError("名称和 API Key 不能为空");
      return;
    }
    addMut.mutate({
      name: form.name.trim(),
      platform: form.platform,
      secret: form.secret.trim(),
      base_url: showBaseUrl ? form.base_url.trim() || undefined : undefined,
      notes: form.notes.trim() || undefined,
    });
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>添加 API Key</h2>
          <button className="btn btn-icon" onClick={onClose}>✕</button>
        </div>

        <form className="modal-body" onSubmit={handleSubmit}>
          <div className="form-row">
            <label className="form-label">名称 *</label>
            <input
              className="form-input"
              placeholder="例：我的 OpenAI Key"
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
            />
          </div>

          <div className="form-row">
            <label className="form-label">平台 *</label>
            <select
              className="form-input"
              value={form.platform}
              onChange={(e) => setForm({ ...form, platform: e.target.value as Platform })}
            >
              {PLATFORMS.map((p) => (
                <option key={p.value} value={p.value}>{p.label}</option>
              ))}
            </select>
          </div>

          {showBaseUrl && (
            <div className="form-row">
              <label className="form-label">接口地址</label>
              <input
                className="form-input"
                placeholder="https://api.example.com"
                value={form.base_url}
                onChange={(e) => setForm({ ...form, base_url: e.target.value })}
              />
            </div>
          )}

          <div className="form-row">
            <label className="form-label">API Key *</label>
            <input
              className="form-input font-mono"
              type="password"
              placeholder="sk-..."
              value={form.secret}
              onChange={(e) => setForm({ ...form, secret: e.target.value })}
              autoComplete="off"
            />
            <p className="form-hint">🔒 Key 将加密存储在系统 Keychain 中，不会以明文保存</p>
          </div>

          <div className="form-row">
            <label className="form-label">备注（可选）</label>
            <input
              className="form-input"
              placeholder="用途说明..."
              value={form.notes}
              onChange={(e) => setForm({ ...form, notes: e.target.value })}
            />
          </div>

          {error && <div className="form-error">{error}</div>}

          <div className="modal-footer">
            <button type="button" className="btn btn-ghost" onClick={onClose}>取消</button>
            <button
              type="submit"
              className="btn btn-primary"
              disabled={addMut.isPending}
            >
              {addMut.isPending ? "保存中..." : "保存"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
