import type { ChangeEvent } from "react";
import type {
  CurrentAccountSnapshot,
  FloatingAccountCard,
  OAuthEnvStatusItem,
  SkillStorageInfo,
  WebReportStatus,
  WebSocketStatus,
} from "../../lib/api";

type SettingsRuntimeSectionProps = {
  languageTitle: string;
  languageDescription: string;
  selectedLanguage: string;
  onLanguageChange: (event: ChangeEvent<HTMLSelectElement>) => void;
  runtimeLoading: boolean;
  skillStorage: SkillStorageInfo | null;
  oauthEnvStatus: OAuthEnvStatusItem[];
  websocketStatus: WebSocketStatus | null;
  webReportStatus: WebReportStatus | null;
  currentSnapshots: CurrentAccountSnapshot[];
  floatingCards: FloatingAccountCard[];
  floatingCardMsg: string;
  onCreateGlobalFloatingCard: () => void;
  onToggleFloatingCardVisible: (card: FloatingAccountCard) => void;
  onToggleFloatingCardTop: (card: FloatingAccountCard) => void;
  onDeleteFloatingCard: (card: FloatingAccountCard) => void;
};

export function SettingsRuntimeSection({
  languageTitle,
  languageDescription,
  selectedLanguage,
  onLanguageChange,
  runtimeLoading,
  skillStorage,
  oauthEnvStatus,
  websocketStatus,
  webReportStatus,
  currentSnapshots,
  floatingCards,
  floatingCardMsg,
  onCreateGlobalFloatingCard,
  onToggleFloatingCardVisible,
  onToggleFloatingCardTop,
  onDeleteFloatingCard,
}: SettingsRuntimeSectionProps) {
  return (
    <>
      <h3 style={{ marginBottom: "var(--space-2)" }}>{languageTitle}</h3>
      <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>
        {languageDescription}
      </p>
      <div style={{ marginBottom: "var(--space-6)" }}>
        <select
          className="form-input"
          style={{ width: "200px" }}
          value={selectedLanguage}
          onChange={onLanguageChange}
        >
          <option value="zh">简体中文</option>
          <option value="en">English</option>
        </select>
      </div>

      <h3 style={{ marginBottom: "var(--space-2)" }}>运行时状态</h3>
      <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>
        把当前应用真正依赖的本地路径和 OAuth 环境配置显式展示出来，方便排查问题。
      </p>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: "var(--space-4)",
          marginBottom: "var(--space-6)",
        }}
      >
        <div
          style={{
            background: "var(--surface-sunken)",
            padding: "var(--space-4)",
            borderRadius: "var(--radius-md)",
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: "var(--space-2)" }}>Skills 存储位置</div>
          {runtimeLoading ? (
            <div className="text-muted" style={{ fontSize: 13 }}>
              加载中...
            </div>
          ) : skillStorage ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
              <div>
                <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>
                  主仓路径
                </div>
                <code style={{ fontSize: 12, wordBreak: "break-all" }}>
                  {skillStorage.primary_path}
                </code>
              </div>
              <div>
                <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>
                  兼容旧目录
                </div>
                <code style={{ fontSize: 12, wordBreak: "break-all" }}>
                  {skillStorage.legacy_path}
                </code>
              </div>
              <div
                style={{
                  fontSize: 12,
                  color: skillStorage.legacy_exists
                    ? "var(--color-warning)"
                    : "var(--color-success)",
                }}
              >
                {skillStorage.legacy_exists
                  ? "检测到旧目录，应用会继续兼容读取。"
                  : "未检测到旧目录，当前已完全使用新技能仓。"}
              </div>
            </div>
          ) : (
            <div className="text-muted" style={{ fontSize: 13 }}>
              未能读取技能仓信息
            </div>
          )}
        </div>

        <div
          style={{
            background: "var(--surface-sunken)",
            padding: "var(--space-4)",
            borderRadius: "var(--radius-md)",
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: "var(--space-2)" }}>OAuth 环境配置</div>
          {runtimeLoading ? (
            <div className="text-muted" style={{ fontSize: 13 }}>
              加载中...
            </div>
          ) : oauthEnvStatus.length > 0 ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
              {oauthEnvStatus.map((item) => (
                <div
                  key={item.env_name}
                  style={{
                    paddingBottom: "var(--space-2)",
                    borderBottom: "1px solid var(--color-border)",
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      marginBottom: 4,
                    }}
                  >
                    <span style={{ fontWeight: 500, fontSize: 13 }}>{item.provider}</span>
                    <span
                      style={{
                        fontSize: 12,
                        color: item.configured
                          ? "var(--color-success)"
                          : "var(--color-danger)",
                      }}
                    >
                      {item.configured ? "已配置" : "缺失"}
                    </span>
                  </div>
                  <code style={{ fontSize: 12, wordBreak: "break-all" }}>{item.env_name}</code>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-muted" style={{ fontSize: 13 }}>
              当前没有需要展示的 OAuth 环境项
            </div>
          )}
        </div>

        <div
          style={{
            background: "var(--surface-sunken)",
            padding: "var(--space-4)",
            borderRadius: "var(--radius-md)",
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: "var(--space-2)" }}>本地 WebSocket 广播</div>
          {runtimeLoading ? (
            <div className="text-muted" style={{ fontSize: 13 }}>
              加载中...
            </div>
          ) : websocketStatus ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
              <div style={{ fontSize: 13 }}>
                状态：
                <strong style={{ marginLeft: 6 }}>
                  {websocketStatus.running ? "运行中" : "未启动"}
                </strong>
              </div>
              <div style={{ fontSize: 13 }}>
                端口：
                <code style={{ marginLeft: 6 }}>{websocketStatus.port ?? "—"}</code>
              </div>
              <div style={{ fontSize: 13 }}>
                客户端数：
                <strong style={{ marginLeft: 6 }}>{websocketStatus.client_count}</strong>
              </div>
            </div>
          ) : (
            <div className="text-muted" style={{ fontSize: 13 }}>
              未能读取 WebSocket 状态
            </div>
          )}
        </div>

        <div
          style={{
            background: "var(--surface-sunken)",
            padding: "var(--space-4)",
            borderRadius: "var(--radius-md)",
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: "var(--space-2)" }}>本地 Web Report 状态页</div>
          {runtimeLoading ? (
            <div className="text-muted" style={{ fontSize: 13 }}>
              加载中...
            </div>
          ) : webReportStatus ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
              <div style={{ fontSize: 13 }}>
                地址：
                <code style={{ marginLeft: 6 }}>{webReportStatus.local_url || "—"}</code>
              </div>
              <div style={{ fontSize: 13 }}>
                健康检查：
                <code style={{ marginLeft: 6 }}>{webReportStatus.health_url || "—"}</code>
              </div>
              <div style={{ fontSize: 13 }}>
                JSON 状态：
                <code style={{ marginLeft: 6 }}>{webReportStatus.status_api_url || "—"}</code>
              </div>
              <div style={{ fontSize: 13 }}>
                JSON 快照：
                <code style={{ marginLeft: 6 }}>{webReportStatus.snapshot_api_url || "—"}</code>
              </div>
              <div style={{ fontSize: 13 }}>
                JSON 认证：
                <strong style={{ marginLeft: 6 }}>
                  {webReportStatus.auth_enabled ? "已启用（需携带 token）" : "未启用"}
                </strong>
              </div>
              <div className="text-muted" style={{ fontSize: 12 }}>
                现在除了 HTML 状态页，也能给外部客户端直接消费 `status/snapshot` JSON；如配置环境变量 `AIS_WEB_REPORT_TOKEN`，JSON 接口会要求认证。
              </div>
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
              <div className="text-muted" style={{ fontSize: 13 }}>
                未能读取 Web Report 状态
              </div>
            </div>
          )}
        </div>

        <div
          style={{
            background: "var(--surface-sunken)",
            padding: "var(--space-4)",
            borderRadius: "var(--radius-md)",
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: "var(--space-2)" }}>当前账号快照</div>
          {runtimeLoading ? (
            <div className="text-muted" style={{ fontSize: 13 }}>
              加载中...
            </div>
          ) : currentSnapshots.length > 0 ? (
            <div style={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 12 }}>
              {currentSnapshots.map((item) => (
                <div
                  key={item.platform}
                  style={{
                    padding: "10px 12px",
                    borderRadius: "var(--radius-sm)",
                    border: "1px solid var(--color-border)",
                    background: "rgba(255,255,255,0.04)",
                  }}
                >
                  <div style={{ fontSize: 12, fontWeight: 700, marginBottom: 4 }}>
                    {item.platform}
                  </div>
                  <div style={{ fontSize: 13 }}>{item.label || "未解析到当前账号"}</div>
                  <div className="text-muted" style={{ fontSize: 11, marginTop: 4 }}>
                    {item.email || "—"} · {item.status || "unknown"}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-muted" style={{ fontSize: 13 }}>
              当前没有可展示的账号快照
            </div>
          )}
        </div>

        <div
          style={{
            background: "var(--surface-sunken)",
            padding: "var(--space-4)",
            borderRadius: "var(--radius-md)",
          }}
        >
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              gap: 8,
              alignItems: "center",
              flexWrap: "wrap",
            }}
          >
            <div style={{ fontWeight: 600 }}>浮动账号卡片</div>
            <button className="btn btn-secondary" onClick={onCreateGlobalFloatingCard}>
              新建全局浮窗
            </button>
          </div>
          <div className="text-muted" style={{ fontSize: 12, marginTop: 6 }}>
            浮窗支持实例绑定、拖拽定位记忆和跨窗口同步，账号切换会自动广播到所有窗口。
          </div>
          <div style={{ marginTop: 10, display: "flex", flexDirection: "column", gap: 8 }}>
            {floatingCards.length === 0 ? (
              <div className="text-muted" style={{ fontSize: 13 }}>
                当前还没有浮动账号卡片
              </div>
            ) : (
              floatingCards.map((card) => (
                <div
                  key={card.id}
                  style={{
                    padding: "10px 12px",
                    borderRadius: "var(--radius-sm)",
                    border: "1px solid var(--color-border)",
                    background: "rgba(255,255,255,0.04)",
                    display: "flex",
                    justifyContent: "space-between",
                    gap: 8,
                    alignItems: "center",
                    flexWrap: "wrap",
                  }}
                >
                  <div>
                    <div style={{ fontSize: 13, fontWeight: 600 }}>{card.title}</div>
                    <div className="text-muted" style={{ fontSize: 11, marginTop: 4 }}>
                      {card.scope === "instance"
                        ? `实例绑定: ${card.instance_id || "未知实例"}`
                        : "全局浮窗"}
                      {" · "}
                      {`平台 ${card.bound_platforms?.join(", ") || "codex, gemini"}`}
                      {" · "}
                      {card.visible ? "可见" : "已隐藏"}
                      {" · "}
                      {card.always_on_top ? "置顶" : "普通"}
                    </div>
                  </div>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                    <button
                      className="btn btn-secondary"
                      onClick={() => onToggleFloatingCardVisible(card)}
                    >
                      {card.visible ? "隐藏" : "显示"}
                    </button>
                    <button
                      className="btn btn-secondary"
                      onClick={() => onToggleFloatingCardTop(card)}
                    >
                      {card.always_on_top ? "取消置顶" : "设为置顶"}
                    </button>
                    <button className="btn btn-danger" onClick={() => onDeleteFloatingCard(card)}>
                      删除
                    </button>
                  </div>
                </div>
              ))
            )}
          </div>
          {floatingCardMsg ? (
            <div className="text-muted" style={{ fontSize: 12, marginTop: 8 }}>
              {floatingCardMsg}
            </div>
          ) : null}
        </div>
      </div>
    </>
  );
}
