import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import type { NotifyConfig } from "../../lib/api/notify";

const DEFAULT_CONFIG: NotifyConfig = {
  feishuEnabled: false,
  feishuWebhook: "",
  dingtalkEnabled: false,
  dingtalkWebhook: "",
  dingtalkSecret: "",
  wecomEnabled: false,
  wecomWebhook: "",
  emailEnabled: false,
  emailSmtpHost: "",
  emailSmtpPort: 465,
  emailUsername: "",
  emailPassword: "",
  emailTo: "",
};

type TestState = "idle" | "loading" | "ok" | "error";

interface ChannelTestState {
  feishu: TestState;
  dingtalk: TestState;
  wecom: TestState;
  email: TestState;
}

interface ChannelTestMsg {
  feishu: string;
  dingtalk: string;
  wecom: string;
  email: string;
}

export function SettingsNotifySection() {
  const [config, setConfig] = useState<NotifyConfig>(DEFAULT_CONFIG);
  const [saveMsg, setSaveMsg] = useState("");
  const [saving, setSaving] = useState(false);
  const [testState, setTestState] = useState<ChannelTestState>({
    feishu: "idle",
    dingtalk: "idle",
    wecom: "idle",
    email: "idle",
  });
  const [testMsg, setTestMsg] = useState<ChannelTestMsg>({
    feishu: "",
    dingtalk: "",
    wecom: "",
    email: "",
  });

  useEffect(() => {
    void api.notify.getConfig().then((cfg) => setConfig(cfg)).catch(() => {
      // backend not yet implemented — keep defaults
    });
  }, []);

  function patch(partial: Partial<NotifyConfig>) {
    setConfig((prev) => ({ ...prev, ...partial }));
  }

  async function handleSave() {
    setSaving(true);
    setSaveMsg("");
    try {
      await api.notify.saveConfig(config);
      setSaveMsg("配置已保存");
    } catch (e) {
      setSaveMsg(`保存失败：${String(e)}`);
    } finally {
      setSaving(false);
    }
  }

  async function handleTest(channel: keyof ChannelTestState) {
    setTestState((prev) => ({ ...prev, [channel]: "loading" }));
    setTestMsg((prev) => ({ ...prev, [channel]: "" }));
    try {
      await api.notify.testChannel(channel, "AI Singularity 测试通知", "这是一条来自 AI Singularity 的测试消息，如果您看到此内容说明通知渠道配置正确。");
      setTestState((prev) => ({ ...prev, [channel]: "ok" }));
      setTestMsg((prev) => ({ ...prev, [channel]: "发送成功 ✓" }));
    } catch (e) {
      setTestState((prev) => ({ ...prev, [channel]: "error" }));
      setTestMsg((prev) => ({ ...prev, [channel]: `发送失败：${String(e)}` }));
    }
  }

  const cardStyle: React.CSSProperties = {
    background: "rgba(255, 255, 255, 0.4)",
    backdropFilter: "blur(16px)",
    WebkitBackdropFilter: "blur(16px)",
    border: "1px solid rgba(15, 23, 42, 0.08)",
    borderRadius: "var(--radius-md)",
    padding: "var(--space-4)",
    boxShadow: "0 20px 40px -10px rgba(15, 23, 42, 0.08), 0 0 0 1px rgba(15, 23, 42, 0.03)",
    display: "flex",
    flexDirection: "column",
    gap: "var(--space-3)",
  };

  const toggleRowStyle: React.CSSProperties = {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    marginBottom: "var(--space-1)",
  };

  const labelStyle: React.CSSProperties = {
    display: "block",
    fontSize: "13px",
    marginBottom: "4px",
    color: "var(--color-text-secondary)",
  };

  return (
    <>
      <h3 style={{ marginBottom: "var(--space-2)" }}>告警通知渠道</h3>
      <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>
        配置告警推送渠道，当账号异常、余额不足或任务失败时，系统将通过已启用的渠道发送通知。
      </p>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))",
          gap: "var(--space-4)",
          marginBottom: "var(--space-6)",
        }}
      >
        {/* 飞书 */}
        <div style={cardStyle}>
          <div style={toggleRowStyle}>
            <div>
              <div style={{ fontWeight: 600, fontSize: "14px" }}>飞书 Webhook</div>
              <div className="text-muted" style={{ fontSize: "12px" }}>通过飞书机器人发送群消息</div>
            </div>
            <Toggle
              checked={config.feishuEnabled}
              onChange={(v) => patch({ feishuEnabled: v })}
            />
          </div>
          <div style={{ opacity: config.feishuEnabled ? 1 : 0.45, transition: "opacity 0.2s" }}>
            <label style={labelStyle}>Webhook URL</label>
            <input
              type="text"
              className="form-input"
              placeholder="https://open.feishu.cn/open-apis/bot/v2/hook/..."
              value={config.feishuWebhook}
              disabled={!config.feishuEnabled}
              onChange={(e) => patch({ feishuWebhook: e.target.value })}
            />
          </div>
          <TestRow
            channel="feishu"
            enabled={config.feishuEnabled && !!config.feishuWebhook}
            state={testState.feishu}
            msg={testMsg.feishu}
            onTest={() => void handleTest("feishu")}
          />
        </div>

        {/* 钉钉 */}
        <div style={cardStyle}>
          <div style={toggleRowStyle}>
            <div>
              <div style={{ fontWeight: 600, fontSize: "14px" }}>钉钉 Webhook</div>
              <div className="text-muted" style={{ fontSize: "12px" }}>通过钉钉自定义机器人推送</div>
            </div>
            <Toggle
              checked={config.dingtalkEnabled}
              onChange={(v) => patch({ dingtalkEnabled: v })}
            />
          </div>
          <div style={{ opacity: config.dingtalkEnabled ? 1 : 0.45, transition: "opacity 0.2s" }}>
            <label style={labelStyle}>Webhook URL</label>
            <input
              type="text"
              className="form-input"
              placeholder="https://oapi.dingtalk.com/robot/send?access_token=..."
              value={config.dingtalkWebhook}
              disabled={!config.dingtalkEnabled}
              onChange={(e) => patch({ dingtalkWebhook: e.target.value })}
              style={{ marginBottom: "var(--space-2)" }}
            />
            <label style={labelStyle}>加签密钥（可选）</label>
            <input
              type="text"
              className="form-input"
              placeholder="SEC..."
              value={config.dingtalkSecret}
              disabled={!config.dingtalkEnabled}
              onChange={(e) => patch({ dingtalkSecret: e.target.value })}
            />
          </div>
          <TestRow
            channel="dingtalk"
            enabled={config.dingtalkEnabled && !!config.dingtalkWebhook}
            state={testState.dingtalk}
            msg={testMsg.dingtalk}
            onTest={() => void handleTest("dingtalk")}
          />
        </div>

        {/* 企业微信 */}
        <div style={cardStyle}>
          <div style={toggleRowStyle}>
            <div>
              <div style={{ fontWeight: 600, fontSize: "14px" }}>企业微信 Webhook</div>
              <div className="text-muted" style={{ fontSize: "12px" }}>通过企业微信群机器人推送</div>
            </div>
            <Toggle
              checked={config.wecomEnabled}
              onChange={(v) => patch({ wecomEnabled: v })}
            />
          </div>
          <div style={{ opacity: config.wecomEnabled ? 1 : 0.45, transition: "opacity 0.2s" }}>
            <label style={labelStyle}>Webhook URL</label>
            <input
              type="text"
              className="form-input"
              placeholder="https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=..."
              value={config.wecomWebhook}
              disabled={!config.wecomEnabled}
              onChange={(e) => patch({ wecomWebhook: e.target.value })}
            />
          </div>
          <TestRow
            channel="wecom"
            enabled={config.wecomEnabled && !!config.wecomWebhook}
            state={testState.wecom}
            msg={testMsg.wecom}
            onTest={() => void handleTest("wecom")}
          />
        </div>

        {/* 邮件 */}
        <div style={cardStyle}>
          <div style={toggleRowStyle}>
            <div>
              <div style={{ fontWeight: 600, fontSize: "14px" }}>邮件通知</div>
              <div className="text-muted" style={{ fontSize: "12px" }}>通过 SMTP 发送告警邮件</div>
            </div>
            <Toggle
              checked={config.emailEnabled}
              onChange={(v) => patch({ emailEnabled: v })}
            />
          </div>
          <div
            style={{
              opacity: config.emailEnabled ? 1 : 0.45,
              transition: "opacity 0.2s",
              display: "flex",
              flexDirection: "column",
              gap: "var(--space-2)",
            }}
          >
            <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: "var(--space-2)" }}>
              <div>
                <label style={labelStyle}>SMTP 服务器</label>
                <input
                  type="text"
                  className="form-input"
                  placeholder="smtp.example.com"
                  value={config.emailSmtpHost}
                  disabled={!config.emailEnabled}
                  onChange={(e) => patch({ emailSmtpHost: e.target.value })}
                />
              </div>
              <div style={{ minWidth: "80px" }}>
                <label style={labelStyle}>端口</label>
                <input
                  type="number"
                  className="form-input"
                  placeholder="465"
                  value={config.emailSmtpPort}
                  disabled={!config.emailEnabled}
                  onChange={(e) => patch({ emailSmtpPort: Number(e.target.value) })}
                />
              </div>
            </div>
            <div>
              <label style={labelStyle}>发件人用户名</label>
              <input
                type="text"
                className="form-input"
                placeholder="notify@example.com"
                value={config.emailUsername}
                disabled={!config.emailEnabled}
                onChange={(e) => patch({ emailUsername: e.target.value })}
              />
            </div>
            <div>
              <label style={labelStyle}>授权密码 / App Password</label>
              <input
                type="password"
                className="form-input"
                placeholder="••••••••"
                value={config.emailPassword}
                disabled={!config.emailEnabled}
                onChange={(e) => patch({ emailPassword: e.target.value })}
              />
            </div>
            <div>
              <label style={labelStyle}>收件人地址</label>
              <input
                type="text"
                className="form-input"
                placeholder="you@example.com"
                value={config.emailTo}
                disabled={!config.emailEnabled}
                onChange={(e) => patch({ emailTo: e.target.value })}
              />
            </div>
          </div>
          <TestRow
            channel="email"
            enabled={
              config.emailEnabled &&
              !!config.emailSmtpHost &&
              !!config.emailUsername &&
              !!config.emailTo
            }
            state={testState.email}
            msg={testMsg.email}
            onTest={() => void handleTest("email")}
          />
        </div>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)" }}>
        <button className="btn btn-primary" onClick={() => void handleSave()} disabled={saving}>
          {saving ? "保存中..." : "保存配置"}
        </button>
        {saveMsg ? (
          <span
            className="text-muted"
            style={{
              fontSize: "13px",
              color: saveMsg.startsWith("保存失败") ? "var(--color-danger, #ef4444)" : undefined,
            }}
          >
            {saveMsg}
          </span>
        ) : null}
      </div>
    </>
  );
}

// ── Toggle switch ────────────────────────────────────────────────────────────

interface ToggleProps {
  checked: boolean;
  onChange: (value: boolean) => void;
}

function Toggle({ checked, onChange }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      style={{
        flexShrink: 0,
        width: "40px",
        height: "22px",
        borderRadius: "999px",
        border: "none",
        cursor: "pointer",
        padding: "2px",
        background: checked ? "var(--color-accent, #2563EB)" : "rgba(15, 23, 42, 0.15)",
        transition: "background 0.2s cubic-bezier(0.16, 1, 0.3, 1)",
        display: "flex",
        alignItems: "center",
        justifyContent: checked ? "flex-end" : "flex-start",
      }}
    >
      <span
        style={{
          display: "block",
          width: "18px",
          height: "18px",
          borderRadius: "50%",
          background: "#fff",
          boxShadow: "0 1px 4px rgba(15, 23, 42, 0.18)",
          transition: "transform 0.2s cubic-bezier(0.16, 1, 0.3, 1)",
        }}
      />
    </button>
  );
}

// ── Test row ─────────────────────────────────────────────────────────────────

interface TestRowProps {
  channel: string;
  enabled: boolean;
  state: TestState;
  msg: string;
  onTest: () => void;
}

function TestRow({ enabled, state, msg, onTest }: TestRowProps) {
  const msgColor =
    state === "ok"
      ? "#10B981"
      : state === "error"
        ? "var(--color-danger, #ef4444)"
        : "var(--color-text-secondary)";

  return (
    <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginTop: "var(--space-1)" }}>
      <button
        className="btn btn-secondary"
        disabled={!enabled || state === "loading"}
        onClick={onTest}
        style={{ fontSize: "12px", padding: "4px 12px" }}
      >
        {state === "loading" ? "发送中..." : "测试"}
      </button>
      {msg ? (
        <span style={{ fontSize: "12px", color: msgColor }}>{msg}</span>
      ) : null}
    </div>
  );
}
