import { useEffect, useMemo, useState } from "react";
import {
  AlertCircle,
  Fingerprint as FingerprintIcon,
  History,
  Plug,
  RefreshCw,
  ShieldAlert,
  X,
  Zap,
} from "lucide-react";
import {
  accountHealth,
  accountRefresh,
  autoSwitch,
  deviceFingerprints,
  extensionImport,
  quotaAlert,
  type AutoSwitchGroupDefinition,
  type AutoSwitchSettings,
  type DeviceFingerprintRecord,
  type ExtensionScanResult,
  type QuotaAlertSettings,
  type RefreshStats,
} from "../../lib/api/integration";
import "./AccountCockpitDrawer.css";

type TabKey = "health" | "auto" | "alert" | "fingerprint" | "extension";

const TABS: { key: TabKey; label: string; icon: typeof Zap }[] = [
  { key: "health", label: "账号状态", icon: ShieldAlert },
  { key: "auto", label: "自动切换", icon: Zap },
  { key: "alert", label: "用量告警", icon: AlertCircle },
  { key: "fingerprint", label: "设备标识", icon: FingerprintIcon },
  { key: "extension", label: "插件导入", icon: Plug },
];

interface AccountCockpitDrawerProps {
  open: boolean;
  onClose: () => void;
  accounts: Array<{ id: string; email: string; origin_platform: string }>;
}

export function AccountCockpitDrawer({
  open,
  onClose,
  accounts,
}: AccountCockpitDrawerProps) {
  const [tab, setTab] = useState<TabKey>("health");

  if (!open) return null;
  return (
    <div className="cockpit-drawer-overlay" onClick={onClose}>
      <div
        className="cockpit-drawer"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
      >
        <header className="cockpit-drawer-header">
          <h2>账号管理中心</h2>
          <button onClick={onClose} aria-label="关闭" className="cockpit-close-btn">
            <X size={18} />
          </button>
        </header>
        <nav className="cockpit-tabs">
          {TABS.map((t) => {
            const Icon = t.icon;
            return (
              <button
                key={t.key}
                className={`cockpit-tab ${tab === t.key ? "active" : ""}`}
                onClick={() => setTab(t.key)}
              >
                <Icon size={14} />
                {t.label}
              </button>
            );
          })}
        </nav>
        <main className="cockpit-content">
          {tab === "health" && <HealthPanel />}
          {tab === "auto" && <AutoSwitchPanel accounts={accounts} />}
          {tab === "alert" && <QuotaAlertPanel />}
          {tab === "fingerprint" && (
            <FingerprintPanel accounts={accounts} />
          )}
          {tab === "extension" && <ExtensionImportPanel />}
        </main>
      </div>
    </div>
  );
}

// ---------------- Health Panel ----------------

function HealthPanel() {
  const [disabled, setDisabled] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [refreshStats, setRefreshStats] = useState<RefreshStats | null>(null);

  async function load() {
    setLoading(true);
    try {
      const list = await accountHealth.listDisabled();
      setDisabled(list);
    } finally {
      setLoading(false);
    }
  }
  useEffect(() => {
    void load();
  }, []);

  async function handleClear(id: string) {
    await accountHealth.clearDisabled(id);
    await load();
  }

  async function handleBatchRefresh() {
    setRefreshing(true);
    try {
      const stats = await accountRefresh.refreshAll("manual_batch");
      setRefreshStats(stats);
    } finally {
      setRefreshing(false);
      await load();
    }
  }

  return (
    <section className="cockpit-section">
      <header className="cockpit-section-header">
        <h3>账号状态</h3>
        <div className="cockpit-actions">
          <button onClick={() => void load()} disabled={loading}>
            <RefreshCw size={14} /> 刷新列表
          </button>
          <button
            onClick={() => void handleBatchRefresh()}
            disabled={refreshing}
            className="primary"
          >
            <RefreshCw size={14} /> {refreshing ? "刷新中..." : "立即批量刷新"}
          </button>
        </div>
      </header>
      {refreshStats && (
        <div className="cockpit-stats">
          总计 {refreshStats.total} · 成功 {refreshStats.success} · 失败{" "}
          {refreshStats.failed}
        </div>
      )}
      {disabled.length === 0 ? (
        <p className="cockpit-empty">当前没有禁用账号</p>
      ) : (
        <table className="cockpit-table">
          <thead>
            <tr>
              <th>邮箱</th>
              <th>平台</th>
              <th>禁用原因</th>
              <th>禁用时间</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            {disabled.map((a: any) => (
              <tr key={a.id}>
                <td>{a.email}</td>
                <td>{a.origin_platform}</td>
                <td className="cockpit-reason">{a.disabled_reason || "未知"}</td>
                <td>{a.disabled_at ? new Date(a.disabled_at).toLocaleString() : "—"}</td>
                <td>
                  <button onClick={() => void handleClear(a.id)}>恢复启用</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

// ---------------- Auto Switch Panel ----------------

function AutoSwitchPanel({
  accounts,
}: {
  accounts: Array<{ id: string; email: string; origin_platform: string }>;
}) {
  const [settings, setSettings] = useState<AutoSwitchSettings | null>(null);
  const [groups, setGroups] = useState<AutoSwitchGroupDefinition[]>([]);
  const [history, setHistory] = useState<any[]>([]);
  const [running, setRunning] = useState(false);
  const [saving, setSaving] = useState(false);
  const [outcome, setOutcome] = useState<string | null>(null);

  async function load() {
    const [s, g, h] = await Promise.all([
      autoSwitch.getSettings(),
      autoSwitch.listGroups(),
      autoSwitch.listHistory(20),
    ]);
    setSettings(s);
    setGroups(g);
    setHistory(h);
  }
  useEffect(() => {
    void load();
  }, []);

  async function save() {
    if (!settings) return;
    setSaving(true);
    try {
      const updated = await autoSwitch.setSettings(settings);
      setSettings(updated);
    } finally {
      setSaving(false);
    }
  }

  async function runNow() {
    setRunning(true);
    try {
      const res = await autoSwitch.runNow();
      setOutcome(
        res.triggered
          ? `已切换 ${res.fromAccountId ?? "?"} → ${res.toAccountId} (rule=${res.rule})`
          : `未触发: ${res.reason ?? "no_trigger"}`,
      );
      const h = await autoSwitch.listHistory(20);
      setHistory(h);
    } finally {
      setRunning(false);
    }
  }

  if (!settings) return <p className="cockpit-empty">加载中...</p>;

  return (
    <section className="cockpit-section">
      <header className="cockpit-section-header">
        <h3>自动切换账号</h3>
        <div className="cockpit-actions">
          <button onClick={() => void runNow()} disabled={running} className="primary">
            <Zap size={14} /> {running ? "运行中..." : "立即执行一次"}
          </button>
        </div>
      </header>
      {outcome && <div className="cockpit-stats">{outcome}</div>}

      <div className="cockpit-form">
        <label className="cockpit-row">
          <input
            type="checkbox"
            checked={settings.enabled}
            onChange={(e) =>
              setSettings({ ...settings, enabled: e.target.checked })
            }
          />
          启用自动切换账号
        </label>
        <label className="cockpit-row">
          阈值（≤ 该百分比触发）
          <input
            type="number"
            min={0}
            max={100}
            value={settings.threshold}
            onChange={(e) =>
              setSettings({
                ...settings,
                threshold: Number(e.target.value) || 0,
              })
            }
          />
          %
        </label>
        <label className="cockpit-row">
          <input
            type="checkbox"
            checked={settings.hardSwitchEnabled}
            onChange={(e) =>
              setSettings({
                ...settings,
                hardSwitchEnabled: e.target.checked,
              })
            }
          />
          强制切换（关闭并重启）
        </label>
        <fieldset className="cockpit-fieldset">
          <legend>监控分组（不选 = 全部）</legend>
          {groups.map((g) => {
            const checked = settings.selectedGroupIds.includes(g.id);
            return (
              <label key={g.id} className="cockpit-row">
                <input
                  type="checkbox"
                  checked={checked}
                  onChange={(e) => {
                    const next = new Set(settings.selectedGroupIds);
                    if (e.target.checked) next.add(g.id);
                    else next.delete(g.id);
                    setSettings({
                      ...settings,
                      selectedGroupIds: Array.from(next),
                      scopeMode:
                        next.size === 0 ? "any_group" : "selected_groups",
                    });
                  }}
                />
                {g.name} <span className="cockpit-mute">({g.models.length} 模型)</span>
              </label>
            );
          })}
        </fieldset>
        <fieldset className="cockpit-fieldset">
          <legend>监控账号（不选 = 全部）</legend>
          {accounts
            .filter((a) =>
              ["antigravity", "gemini", "codex"].includes(
                a.origin_platform.toLowerCase(),
              ),
            )
            .map((a) => {
              const checked = settings.selectedAccountIds.includes(a.id);
              return (
                <label key={a.id} className="cockpit-row">
                  <input
                    type="checkbox"
                    checked={checked}
                    onChange={(e) => {
                      const next = new Set(settings.selectedAccountIds);
                      if (e.target.checked) next.add(a.id);
                      else next.delete(a.id);
                      setSettings({
                        ...settings,
                        selectedAccountIds: Array.from(next),
                        accountScopeMode:
                          next.size === 0 ? "all_accounts" : "selected_accounts",
                      });
                    }}
                  />
                  {a.email} <span className="cockpit-mute">[{a.origin_platform}]</span>
                </label>
              );
            })}
        </fieldset>
        <button onClick={() => void save()} disabled={saving} className="primary">
          {saving ? "保存中..." : "保存设置"}
        </button>
      </div>
      <h4 className="cockpit-subhead">
        <History size={14} /> 切换历史 (最近 20)
      </h4>
      {history.length === 0 ? (
        <p className="cockpit-empty">暂无切换历史</p>
      ) : (
        <table className="cockpit-table">
          <thead>
            <tr>
              <th>时间</th>
              <th>触发</th>
              <th>规则</th>
              <th>从</th>
              <th>到</th>
            </tr>
          </thead>
          <tbody>
            {history.map((h) => (
              <tr key={h.id}>
                <td>{new Date(h.ts).toLocaleString()}</td>
                <td>{h.trigger}</td>
                <td>{h.rule || "—"}</td>
                <td>{h.from_email || "—"}</td>
                <td>{h.to_email}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

// ---------------- Quota Alert Panel ----------------

function QuotaAlertPanel() {
  const [settings, setSettings] = useState<QuotaAlertSettings | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [preview, setPreview] = useState<any[]>([]);

  useEffect(() => {
    void quotaAlert.getSettings().then(setSettings);
  }, []);

  async function save() {
    if (!settings) return;
    const updated = await quotaAlert.setSettings(settings);
    setSettings(updated);
  }

  async function runPreview() {
    setPreviewing(true);
    try {
      const list = await quotaAlert.preview();
      setPreview(list);
    } finally {
      setPreviewing(false);
    }
  }

  if (!settings) return <p className="cockpit-empty">加载中...</p>;
  return (
    <section className="cockpit-section">
      <header className="cockpit-section-header">
        <h3>用量告警</h3>
        <div className="cockpit-actions">
          <button onClick={() => void runPreview()} disabled={previewing}>
            <AlertCircle size={14} /> {previewing ? "预览中..." : "立即检查"}
          </button>
        </div>
      </header>
      <div className="cockpit-form">
        <label className="cockpit-row">
          <input
            type="checkbox"
            checked={settings.enabled}
            onChange={(e) =>
              setSettings({ ...settings, enabled: e.target.checked })
            }
          />
          启用用量告警
        </label>
        <label className="cockpit-row">
          阈值
          <input
            type="number"
            min={0}
            max={100}
            value={settings.threshold}
            onChange={(e) =>
              setSettings({
                ...settings,
                threshold: Number(e.target.value) || 0,
              })
            }
          />
          %
        </label>
        <label className="cockpit-row">
          告警冷却时间
          <input
            type="number"
            min={30}
            max={86400}
            value={settings.cooldownSeconds}
            onChange={(e) =>
              setSettings({
                ...settings,
                cooldownSeconds: Number(e.target.value) || 300,
              })
            }
          />
          秒
        </label>
        <button onClick={() => void save()} className="primary">
          保存
        </button>
      </div>
      {preview.length > 0 && (
        <table className="cockpit-table">
          <thead>
            <tr>
              <th>邮箱</th>
              <th>最低</th>
              <th>低位模型</th>
            </tr>
          </thead>
          <tbody>
            {preview.map((p, idx) => (
              <tr key={`${p.account_id}-${idx}`}>
                <td>{p.email}</td>
                <td>{p.lowest_percentage}%</td>
                <td>{p.low_models.join(", ")}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

// ---------------- Fingerprint Panel ----------------

function FingerprintPanel({
  accounts,
}: {
  accounts: Array<{ id: string; email: string; origin_platform: string }>;
}) {
  const [list, setList] = useState<DeviceFingerprintRecord[]>([]);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [bindingFor, setBindingFor] = useState<string | null>(null);

  async function load() {
    setList(await deviceFingerprints.list());
  }
  useEffect(() => {
    void load();
  }, []);

  async function handleCreate() {
    if (!newName.trim()) return;
    setCreating(true);
    try {
      await deviceFingerprints.create(newName.trim(), null);
      setNewName("");
      await load();
    } finally {
      setCreating(false);
    }
  }

  async function handleDelete(id: string) {
    if (id === "original") return;
    await deviceFingerprints.delete(id);
    await load();
  }

  async function handleApply(accountId: string, fpId: string | null) {
    await deviceFingerprints.applyToAccount(accountId, fpId);
    setBindingFor(null);
  }

  return (
    <section className="cockpit-section">
      <header className="cockpit-section-header">
        <h3>设备标识</h3>
        <div className="cockpit-actions">
          <input
            placeholder="标识名称"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
          <button onClick={() => void handleCreate()} disabled={creating} className="primary">
            随机生成
          </button>
        </div>
      </header>
      <table className="cockpit-table">
        <thead>
          <tr>
            <th>名称</th>
            <th>machineId</th>
            <th>创建时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {list.map((fp) => (
            <tr key={fp.id}>
              <td>{fp.name}</td>
              <td className="cockpit-mono">{fp.machineId.slice(0, 16)}...</td>
              <td>{new Date(fp.createdAt).toLocaleString()}</td>
              <td>
                {fp.id !== "original" && (
                  <button onClick={() => void handleDelete(fp.id)}>删除</button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <h4 className="cockpit-subhead">绑定到账号</h4>
      <table className="cockpit-table">
        <thead>
          <tr>
            <th>账号</th>
            <th>平台</th>
            <th>当前指纹</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {accounts
            .filter((a) => a.origin_platform.toLowerCase() === "antigravity")
            .map((a) => (
              <tr key={a.id}>
                <td>{a.email}</td>
                <td>{a.origin_platform}</td>
                <td>—</td>
                <td>
                  {bindingFor === a.id ? (
                    <select
                      onChange={(e) =>
                        void handleApply(a.id, e.target.value || null)
                      }
                      autoFocus
                    >
                      <option value="">—</option>
                      {list.map((fp) => (
                        <option key={fp.id} value={fp.id}>
                          {fp.name}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <button onClick={() => setBindingFor(a.id)}>绑定</button>
                  )}
                </td>
              </tr>
            ))}
        </tbody>
      </table>
    </section>
  );
}

// ---------------- Extension Import Panel ----------------

function ExtensionImportPanel() {
  const [scan, setScan] = useState<ExtensionScanResult[]>([]);
  const [scanning, setScanning] = useState(false);
  const [importing, setImporting] = useState(false);
  const [stats, setStats] = useState<any | null>(null);

  async function runScan() {
    setScanning(true);
    try {
      setScan(await extensionImport.scan());
    } finally {
      setScanning(false);
    }
  }

  async function runImport() {
    setImporting(true);
    try {
      const s = await extensionImport.importAll();
      setStats(s);
      await runScan();
    } finally {
      setImporting(false);
    }
  }

  const summary = useMemo(() => {
    if (!scan.length) return null;
    return `扫描到 ${scan.length} 个候选，其中 ${
      scan.filter((c) => c.hasRefreshToken).length
    } 个含 refresh_token`;
  }, [scan]);

  return (
    <section className="cockpit-section">
      <header className="cockpit-section-header">
        <h3>VS Code 插件导入</h3>
        <div className="cockpit-actions">
          <button onClick={() => void runScan()} disabled={scanning}>
            <Plug size={14} /> {scanning ? "扫描中..." : "扫描插件凭据"}
          </button>
          <button
            onClick={() => void runImport()}
            disabled={importing || scan.length === 0}
            className="primary"
          >
            {importing ? "导入中..." : "全部导入"}
          </button>
        </div>
      </header>
      {summary && <div className="cockpit-stats">{summary}</div>}
      {stats && (
        <div className="cockpit-stats">
          导入 {stats.imported} · 跳过 {stats.skipped} · 失败 {stats.failed}
        </div>
      )}
      {scan.length === 0 ? (
        <p className="cockpit-empty">尚未扫描或未找到凭据</p>
      ) : (
        <table className="cockpit-table">
          <thead>
            <tr>
              <th>邮箱</th>
              <th>插件 ID</th>
              <th>项目 ID</th>
              <th>refresh_token</th>
            </tr>
          </thead>
          <tbody>
            {scan.map((s, idx) => (
              <tr key={`${s.extensionId}-${s.email}-${idx}`}>
                <td>{s.email}</td>
                <td className="cockpit-mono">{s.extensionId}</td>
                <td>{s.projectId || "—"}</td>
                <td>{s.hasRefreshToken ? "✓" : "✗"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
