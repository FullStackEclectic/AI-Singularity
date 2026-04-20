import { useEffect, useMemo, useRef, useState } from "react";
import {
  api,
  type OAuthEnvStatusItem,
  type RuntimeEnvStatusItem,
  type SkillStorageInfo,
  type WebReportStatus,
  type WebSocketStatus,
} from "../../lib/api";
import type { EnvConflict } from "../../types";
import "./ToolDepotPage.css";

type ConflictAppId = "claude" | "openai" | "gemini";

type ConflictBucket = {
  id: ConflictAppId;
  label: string;
  conflicts: EnvConflict[];
};

const CONFLICT_APPS: { id: ConflictAppId; label: string }[] = [
  { id: "claude", label: "Claude" },
  { id: "openai", label: "OpenAI" },
  { id: "gemini", label: "Gemini" },
];

export default function ToolDepotPage() {
  const mountedRef = useRef(true);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [skillStorage, setSkillStorage] = useState<SkillStorageInfo | null>(null);
  const [oauthEnvStatus, setOauthEnvStatus] = useState<OAuthEnvStatusItem[]>([]);
  const [runtimeEnvStatuses, setRuntimeEnvStatuses] = useState<RuntimeEnvStatusItem[]>([]);
  const [websocketStatus, setWebsocketStatus] = useState<WebSocketStatus | null>(null);
  const [webReportStatus, setWebReportStatus] = useState<WebReportStatus | null>(null);
  const [conflicts, setConflicts] = useState<Record<ConflictAppId, EnvConflict[]>>({
    claude: [],
    openai: [],
    gemini: [],
  });
  const [loadError, setLoadError] = useState<string>("");

  useEffect(() => {
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const loadEnvironmentData = async (mode: "initial" | "refresh" = "initial") => {
    if (mode === "refresh") {
      setRefreshing(true);
    } else {
      setLoading(true);
    }
    setLoadError("");

    try {
      const [storageInfo, oauthInfo, runtimeEnvInfo, wsStatus, reportStatus, conflictGroups] =
        await Promise.all([
        api.skills.getStorageInfo(),
        api.oauth.getEnvStatus(),
          api.env.getStatuses(),
          api.websocket.getStatus(),
          api.webReport.getStatus().catch(() => null),
          Promise.all(
            CONFLICT_APPS.map(async (app) => ({
              id: app.id,
              conflicts: await api.env.checkConflicts(app.id),
            }))
          ),
        ]);

      if (!mountedRef.current) {
        return;
      }

      const nextConflicts: Record<ConflictAppId, EnvConflict[]> = {
        claude: [],
        openai: [],
        gemini: [],
      };
      for (const group of conflictGroups) {
        nextConflicts[group.id] = group.conflicts;
      }

      setSkillStorage(storageInfo);
      setOauthEnvStatus(oauthInfo);
      setRuntimeEnvStatuses(runtimeEnvInfo);
      setWebsocketStatus(wsStatus);
      setWebReportStatus(reportStatus);
      setConflicts(nextConflicts);
    } catch (error) {
      console.error("Failed to load environment config:", error);
      if (mountedRef.current) {
        setLoadError("读取环境配置失败，请稍后重试。");
      }
    } finally {
      if (!mountedRef.current) {
        return;
      }
      setLoading(false);
      setRefreshing(false);
    }
  };

  useEffect(() => {
    void loadEnvironmentData();
  }, []);

  const conflictBuckets = useMemo<ConflictBucket[]>(
    () =>
      CONFLICT_APPS.map((app) => ({
        ...app,
        conflicts: conflicts[app.id] || [],
      })),
    [conflicts]
  );

  const totalConflicts = useMemo(
    () => conflictBuckets.reduce((sum, item) => sum + item.conflicts.length, 0),
    [conflictBuckets]
  );

  return (
    <div className="page-container page-tool-depot">
      <div className="page-header">
        <div className="page-title-row">
          <h1>环境配置</h1>
          <p className="page-subtitle">
            把应用当前依赖的本地路径、OAuth 环境变量和系统环境冲突集中展示，方便排查问题。
          </p>
        </div>
        <button
          className="btn btn-secondary"
          onClick={() => void loadEnvironmentData("refresh")}
          disabled={loading || refreshing}
        >
          {refreshing ? "刷新中..." : "刷新"}
        </button>
      </div>

      <div className="env-summary-strip">
        <div className="env-summary-card">
          <span className="env-summary-label">OAuth Client Secret</span>
          <strong>{oauthEnvStatus.length}</strong>
        </div>
        <div className="env-summary-card">
          <span className="env-summary-label">主流 CLI 环境变量</span>
          <strong>{runtimeEnvStatuses.length}</strong>
        </div>
        <div className="env-summary-card">
          <span className="env-summary-label">已发现冲突</span>
          <strong className={totalConflicts > 0 ? "is-danger" : "is-success"}>
            {totalConflicts}
          </strong>
        </div>
        <div className="env-summary-card">
          <span className="env-summary-label">WebSocket</span>
          <strong>{websocketStatus?.running ? "运行中" : "未启动"}</strong>
        </div>
        <div className="env-summary-card">
          <span className="env-summary-label">Web Report</span>
          <strong>{webReportStatus?.running ? "运行中" : "未启动"}</strong>
        </div>
      </div>

      {loadError ? (
        <div className="env-banner env-banner-error">{loadError}</div>
      ) : null}

      {loading ? (
        <div className="env-loading-panel">正在读取环境配置...</div>
      ) : (
        <div className="env-grid">
          <section className="env-card env-card-wide">
            <div className="env-card-header">
              <div>
                <h3>系统环境冲突体检</h3>
                <p>检查会导致 CLI 工具绕过代理或覆盖本应用配置的系统环境变量。</p>
              </div>
              <span className={`env-status-pill ${totalConflicts > 0 ? "danger" : "success"}`}>
                {totalConflicts > 0 ? `发现 ${totalConflicts} 项` : "未发现冲突"}
              </span>
            </div>
            <div className="env-conflict-groups">
              {conflictBuckets.map((bucket) => (
                <div key={bucket.id} className="env-conflict-group">
                  <div className="env-conflict-group-header">
                    <strong>{bucket.label}</strong>
                    <span className={`env-status-pill ${bucket.conflicts.length > 0 ? "danger" : "success"}`}>
                      {bucket.conflicts.length > 0 ? `${bucket.conflicts.length} 项冲突` : "正常"}
                    </span>
                  </div>
                  {bucket.conflicts.length === 0 ? (
                    <div className="env-empty-hint">当前未发现与 {bucket.label} 相关的环境污染。</div>
                  ) : (
                    <div className="env-list">
                      {bucket.conflicts.map((item, index) => (
                        <div
                          key={`${bucket.id}-${item.varName}-${item.sourcePath}-${index}`}
                          className="env-list-item"
                        >
                          <div className="env-list-item-row">
                            <code>{item.varName}</code>
                            <span className="env-inline-meta">{item.sourceType}</span>
                          </div>
                          <div className="env-list-meta">来源: {item.sourcePath}</div>
                          <div className="env-list-meta">当前值: {item.varValue || "(空值)"}</div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </section>

          <section className="env-card">
            <div className="env-card-header">
              <div>
                <h3>OAuth Client Secret 环境项</h3>
                <p>这里只展示 OAuth 额外依赖的 Client Secret，不等同于所有主流工具环境变量。</p>
              </div>
            </div>
            {oauthEnvStatus.length > 0 ? (
              <div className="env-list">
                {oauthEnvStatus.map((item) => (
                  <div key={item.env_name} className="env-list-item">
                    <div className="env-list-item-row">
                      <strong>{item.provider}</strong>
                      <span className={`env-status-pill ${item.configured ? "success" : "danger"}`}>
                        {item.configured ? "已配置" : "缺失"}
                      </span>
                    </div>
                    <code>{item.env_name}</code>
                  </div>
                ))}
              </div>
            ) : (
              <div className="env-empty-hint">当前没有需要展示的 OAuth 环境项。</div>
            )}
          </section>

          <section className="env-card">
            <div className="env-card-header">
              <div>
                <h3>主流 CLI 环境变量</h3>
                <p>集中展示 Claude、Codex/OpenAI、Gemini 常见变量是否已存在，以及它们来自系统环境还是工具配置文件。</p>
              </div>
            </div>
            {runtimeEnvStatuses.length > 0 ? (
              <div className="env-list">
                {runtimeEnvStatuses.map((item) => (
                  <div key={`${item.tool}-${item.env_name}`} className="env-list-item">
                    <div className="env-list-item-row">
                      <strong>{item.label}</strong>
                      <span className={`env-status-pill ${item.configured ? "success" : "muted"}`}>
                        {item.configured ? "已发现" : "未配置"}
                      </span>
                    </div>
                    <code>{item.env_name}</code>
                    {item.sources.length > 0 ? (
                      <div className="env-source-list">
                        {item.sources.map((source) => (
                          <span key={`${item.env_name}-${source}`} className="env-source-chip">
                            {source}
                          </span>
                        ))}
                      </div>
                    ) : (
                      <div className="env-empty-hint">当前未在系统环境或工具配置文件中检测到。</div>
                    )}
                    {item.note ? <div className="env-list-meta">{item.note}</div> : null}
                  </div>
                ))}
              </div>
            ) : (
              <div className="env-empty-hint">当前没有可展示的主流 CLI 环境变量。</div>
            )}
          </section>

          <section className="env-card">
            <div className="env-card-header">
              <div>
                <h3>Skills 存储位置</h3>
                <p>用于确认技能仓当前使用的新目录以及是否仍兼容旧目录。</p>
              </div>
            </div>
            {skillStorage ? (
              <div className="env-list">
                <div className="env-list-item">
                  <div className="env-list-meta">主仓路径</div>
                  <code>{skillStorage.primary_path}</code>
                </div>
                <div className="env-list-item">
                  <div className="env-list-meta">兼容旧目录</div>
                  <code>{skillStorage.legacy_path}</code>
                </div>
                <div className="env-list-item">
                  <span className={`env-status-pill ${skillStorage.legacy_exists ? "warning" : "success"}`}>
                    {skillStorage.legacy_exists ? "检测到旧目录" : "已完全使用新技能仓"}
                  </span>
                </div>
              </div>
            ) : (
              <div className="env-empty-hint">未能读取技能仓信息。</div>
            )}
          </section>

          <section className="env-card">
            <div className="env-card-header">
              <div>
                <h3>本地 WebSocket 广播</h3>
                <p>用于窗口间同步和本地监听的广播服务状态。</p>
              </div>
            </div>
            {websocketStatus ? (
              <div className="env-list">
                <div className="env-list-item">
                  <div className="env-list-item-row">
                    <span>状态</span>
                    <span className={`env-status-pill ${websocketStatus.running ? "success" : "muted"}`}>
                      {websocketStatus.running ? "运行中" : "未启动"}
                    </span>
                  </div>
                </div>
                <div className="env-list-item">
                  <div className="env-list-meta">端口</div>
                  <code>{websocketStatus.port ?? "—"}</code>
                </div>
                <div className="env-list-item">
                  <div className="env-list-meta">客户端数</div>
                  <strong>{websocketStatus.client_count}</strong>
                </div>
              </div>
            ) : (
              <div className="env-empty-hint">未能读取 WebSocket 状态。</div>
            )}
          </section>

          <section className="env-card">
            <div className="env-card-header">
              <div>
                <h3>本地 Web Report 状态页</h3>
                <p>用于外部客户端读取状态页与 JSON 快照的本地服务状态。</p>
              </div>
            </div>
            {webReportStatus ? (
              <div className="env-list">
                <div className="env-list-item">
                  <div className="env-list-item-row">
                    <span>状态</span>
                    <span className={`env-status-pill ${webReportStatus.running ? "success" : "muted"}`}>
                      {webReportStatus.running ? "运行中" : "未启动"}
                    </span>
                  </div>
                </div>
                <div className="env-list-item">
                  <div className="env-list-meta">地址</div>
                  <code>{webReportStatus.local_url || "—"}</code>
                </div>
                <div className="env-list-item">
                  <div className="env-list-meta">健康检查</div>
                  <code>{webReportStatus.health_url || "—"}</code>
                </div>
                <div className="env-list-item">
                  <div className="env-list-meta">JSON 状态</div>
                  <code>{webReportStatus.status_api_url || "—"}</code>
                </div>
                <div className="env-list-item">
                  <div className="env-list-meta">JSON 快照</div>
                  <code>{webReportStatus.snapshot_api_url || "—"}</code>
                </div>
                <div className="env-list-item">
                  <div className="env-list-item-row">
                    <span>接口认证</span>
                    <span className={`env-status-pill ${webReportStatus.auth_enabled ? "warning" : "muted"}`}>
                      {webReportStatus.auth_enabled ? "已启用 token" : "未启用"}
                    </span>
                  </div>
                </div>
              </div>
            ) : (
              <div className="env-empty-hint">未能读取 Web Report 状态。</div>
            )}
          </section>

          <div className="env-banner">
            建议优先处理“系统环境冲突体检”中的项目；如果这里存在硬编码环境变量，CLI 可能会忽略你在应用里设置的代理或 Provider 配置。
          </div>
        </div>
      )}
    </div>
  );
}
