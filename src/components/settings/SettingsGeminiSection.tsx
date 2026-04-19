import type { GeminiInstanceRecord } from "../../lib/api";
import type {
  GeminiInstanceWarning,
  GeminiQuickUpdatePatch,
} from "./settingsTypes";

type GeminiWarningListProps = {
  warnings: GeminiInstanceWarning[];
  warningKeyPrefix: string;
};

function GeminiWarningList({
  warnings,
  warningKeyPrefix,
}: GeminiWarningListProps) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 10 }}>
      {warnings.map((item, index) => (
        <div
          key={`${warningKeyPrefix}-${index}`}
          style={{
            padding: "8px 10px",
            borderRadius: "var(--radius-sm)",
            fontSize: 12,
            lineHeight: 1.5,
            background:
              item.tone === "warning"
                ? "rgba(245,158,11,0.12)"
                : item.tone === "success"
                  ? "rgba(16,185,129,0.12)"
                  : "rgba(59,130,246,0.12)",
            color:
              item.tone === "warning"
                ? "var(--color-warning)"
                : item.tone === "success"
                  ? "var(--color-success)"
                  : "var(--color-primary)",
          }}
        >
          {item.text}
        </div>
      ))}
    </div>
  );
}

type GeminiStatsCardProps = {
  label: string;
  value: number;
  background: string;
};

function GeminiStatsCard({ label, value, background }: GeminiStatsCardProps) {
  return (
    <div
      style={{
        padding: "10px 12px",
        borderRadius: "var(--radius-sm)",
        background,
      }}
    >
      <div className="text-muted" style={{ fontSize: 12, marginBottom: 4 }}>
        {label}
      </div>
      <div style={{ fontWeight: 700, fontSize: 20 }}>{value}</div>
    </div>
  );
}

type GeminiInstanceCardProps = {
  instance: GeminiInstanceRecord;
  currentGeminiAccountId: string | null;
  getGeminiAccountLabel: (accountId?: string | null) => string;
  getEffectiveGeminiAccountId: (
    instance: GeminiInstanceRecord
  ) => string | null | undefined;
  getEffectiveGeminiProjectId: (instance: GeminiInstanceRecord) => string | null;
  getGeminiAccountProjectLabel: (accountId?: string | null) => string | null;
  isCurrentLocalGeminiAccount: (accountId?: string | null) => boolean;
  formatGeminiLaunchTime: (value?: string | null) => string;
  getGeminiInstanceWarnings: (
    instance: GeminiInstanceRecord
  ) => GeminiInstanceWarning[];
  onQuickUpdate: (
    instance: GeminiInstanceRecord,
    patch: GeminiQuickUpdatePatch,
    successMessage: string
  ) => void;
  onOpenSettings: (instance: GeminiInstanceRecord) => void;
  onCopyLaunchCommand: (id: string) => void;
  onLaunch: (id: string) => void;
  onConfirmDelete?: (id: string) => void;
};

function GeminiInstanceCard({
  instance,
  currentGeminiAccountId,
  getGeminiAccountLabel,
  getEffectiveGeminiAccountId,
  getEffectiveGeminiProjectId,
  getGeminiAccountProjectLabel,
  isCurrentLocalGeminiAccount,
  formatGeminiLaunchTime,
  getGeminiInstanceWarnings,
  onQuickUpdate,
  onOpenSettings,
  onCopyLaunchCommand,
  onLaunch,
  onConfirmDelete,
}: GeminiInstanceCardProps) {
  return (
    <div
      style={{
        padding: "var(--space-3)",
        border: "1px solid var(--color-border)",
        borderRadius: "var(--radius-sm)",
        background: "rgba(255,255,255,0.02)",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", gap: "var(--space-3)" }}>
        <div>
          <div style={{ fontWeight: 600 }}>
            {instance.is_default ? "默认实例" : instance.name}
          </div>
          <div className="text-muted" style={{ fontSize: 12, wordBreak: "break-all" }}>
            {instance.user_data_dir}
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 8 }}>
            {instance.is_default ? (
              <span
                style={{
                  fontSize: 11,
                  padding: "2px 8px",
                  borderRadius: 999,
                  background: "rgba(59,130,246,0.12)",
                  color: "var(--color-primary)",
                }}
              >
                当前本地账号：{getGeminiAccountLabel(currentGeminiAccountId)}
              </span>
            ) : null}
            {instance.bind_account_id ? (
              <span
                style={{
                  fontSize: 11,
                  padding: "2px 8px",
                  borderRadius: 999,
                  background: isCurrentLocalGeminiAccount(instance.bind_account_id)
                    ? "rgba(16,185,129,0.12)"
                    : "rgba(255,255,255,0.08)",
                  color: isCurrentLocalGeminiAccount(instance.bind_account_id)
                    ? "var(--color-success)"
                    : "var(--color-text-secondary)",
                }}
              >
                绑定账号：{getGeminiAccountLabel(instance.bind_account_id)}
                {isCurrentLocalGeminiAccount(instance.bind_account_id) ? " · 当前本地" : ""}
              </span>
            ) : !instance.is_default ? (
              <span
                style={{
                  fontSize: 11,
                  padding: "2px 8px",
                  borderRadius: 999,
                  background: "rgba(255,255,255,0.08)",
                  color: "var(--color-text-secondary)",
                }}
              >
                未绑定账号
              </span>
            ) : null}
            <span
              style={{
                fontSize: 11,
                padding: "2px 8px",
                borderRadius: 999,
                background: "rgba(255,255,255,0.08)",
                color: "var(--color-text-secondary)",
              }}
            >
              实际生效账号：{getGeminiAccountLabel(getEffectiveGeminiAccountId(instance))}
            </span>
            <span
              style={{
                fontSize: 11,
                padding: "2px 8px",
                borderRadius: 999,
                background: "rgba(255,255,255,0.08)",
                color: "var(--color-text-secondary)",
              }}
            >
              实际生效项目：
              {getEffectiveGeminiProjectId(instance) || "沿用本地默认行为"}
            </span>
            {instance.is_default ? (
              <span
                style={{
                  fontSize: 11,
                  padding: "2px 8px",
                  borderRadius: 999,
                  background: instance.follow_local_account
                    ? "rgba(16,185,129,0.12)"
                    : "rgba(148,163,184,0.16)",
                  color: instance.follow_local_account
                    ? "var(--color-success)"
                    : "var(--color-text-secondary)",
                }}
              >
                {instance.follow_local_account ? "跟随当前本地账号" : "固定绑定模式"}
              </span>
            ) : (
              <span
                style={{
                  fontSize: 11,
                  padding: "2px 8px",
                  borderRadius: 999,
                  background: instance.initialized
                    ? "rgba(16,185,129,0.12)"
                    : "rgba(245,158,11,0.12)",
                  color: instance.initialized
                    ? "var(--color-success)"
                    : "var(--color-warning)",
                }}
              >
                {instance.initialized ? "已初始化" : "未初始化"}
              </span>
            )}
          </div>
          <div className="text-muted" style={{ fontSize: 12, marginTop: 6 }}>
            {instance.is_default
              ? instance.follow_local_account
                ? `跟随当前本地账号 (${getGeminiAccountLabel(currentGeminiAccountId)})`
                : `绑定账号 ${getGeminiAccountLabel(instance.bind_account_id)}`
              : instance.bind_account_id
                ? `绑定账号 ${getGeminiAccountLabel(instance.bind_account_id)}`
                : "未绑定账号"}
            {" · "}
            {instance.is_default
              ? instance.follow_local_account
                ? "跟随当前本地账号"
                : "固定绑定模式"
              : instance.initialized
                ? "已初始化"
                : "未初始化"}
            {" · "}
            {instance.project_id ? `项目 ${instance.project_id}` : "无项目覆盖"}
            {" · "}
            {instance.extra_args ? `参数 ${instance.extra_args}` : "无额外参数"}
            {" · "}
            {`最近启动 ${formatGeminiLaunchTime(instance.last_launched_at)}`}
          </div>
          <GeminiWarningList
            warnings={getGeminiInstanceWarnings(instance)}
            warningKeyPrefix={instance.is_default ? "default-warning" : `${instance.id}-warning`}
          />
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 10 }}>
            {instance.is_default ? (
              <>
                {!instance.follow_local_account && currentGeminiAccountId ? (
                  <button
                    className="btn btn-secondary"
                    onClick={() =>
                      onQuickUpdate(
                        instance,
                        { bindAccountId: null, followLocalAccount: true },
                        "默认 Gemini 实例已改为跟随当前本地账号"
                      )
                    }
                  >
                    跟随当前本地账号
                  </button>
                ) : null}
                {!instance.follow_local_account &&
                currentGeminiAccountId &&
                instance.bind_account_id !== currentGeminiAccountId ? (
                  <button
                    className="btn btn-secondary"
                    onClick={() =>
                      onQuickUpdate(
                        instance,
                        {
                          bindAccountId: currentGeminiAccountId,
                          followLocalAccount: false,
                        },
                        "默认 Gemini 实例已绑定当前本地账号"
                      )
                    }
                  >
                    绑定当前本地账号
                  </button>
                ) : null}
                {instance.project_id ? (
                  <button
                    className="btn btn-secondary"
                    onClick={() =>
                      onQuickUpdate(
                        instance,
                        { projectId: null },
                        "默认 Gemini 实例已清除项目覆盖"
                      )
                    }
                  >
                    清除项目覆盖
                  </button>
                ) : null}
              </>
            ) : (
              <>
                {currentGeminiAccountId &&
                instance.bind_account_id !== currentGeminiAccountId ? (
                  <button
                    className="btn btn-secondary"
                    onClick={() =>
                      onQuickUpdate(
                        instance,
                        { bindAccountId: currentGeminiAccountId },
                        `${instance.name} 已绑定当前本地账号`
                      )
                    }
                  >
                    绑定当前本地账号
                  </button>
                ) : null}
                {instance.project_id ? (
                  <button
                    className="btn btn-secondary"
                    onClick={() =>
                      onQuickUpdate(
                        instance,
                        { projectId: null },
                        `${instance.name} 已清除项目覆盖`
                      )
                    }
                  >
                    清除项目覆盖
                  </button>
                ) : null}
                {!instance.project_id &&
                getGeminiAccountProjectLabel(instance.bind_account_id) ? (
                  <button
                    className="btn btn-secondary"
                    onClick={() =>
                      onQuickUpdate(
                        instance,
                        {
                          projectId: getGeminiAccountProjectLabel(
                            instance.bind_account_id
                          ),
                        },
                        `${instance.name} 已固定为账号默认项目`
                      )
                    }
                  >
                    固定为账号默认项目
                  </button>
                ) : null}
              </>
            )}
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", justifyContent: "flex-end" }}>
          <button className="btn btn-secondary" onClick={() => onOpenSettings(instance)}>
            设置
          </button>
          <button
            className="btn btn-secondary"
            onClick={() => onCopyLaunchCommand(instance.id)}
          >
            复制命令
          </button>
          <button className="btn btn-primary" onClick={() => onLaunch(instance.id)}>
            在终端启动
          </button>
          {onConfirmDelete ? (
            <button
              className="btn btn-danger"
              onClick={() => onConfirmDelete(instance.id)}
            >
              删除
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}

type SettingsGeminiSectionProps = {
  allVisibleGeminiInstances: GeminiInstanceRecord[];
  geminiUninitializedCount: number;
  geminiConflictCount: number;
  geminiProjectOverrideCount: number;
  geminiUnboundCount: number;
  geminiRefreshLoading: boolean;
  defaultGeminiInstance: GeminiInstanceRecord | null;
  currentGeminiAccountId: string | null;
  geminiInstanceName: string;
  geminiInstanceDir: string;
  geminiInstanceMsg: string;
  geminiInstanceLoading: boolean;
  geminiInstances: GeminiInstanceRecord[];
  sortedGeminiInstances: GeminiInstanceRecord[];
  getGeminiAccountLabel: (accountId?: string | null) => string;
  getEffectiveGeminiAccountId: (
    instance: GeminiInstanceRecord
  ) => string | null | undefined;
  getEffectiveGeminiProjectId: (instance: GeminiInstanceRecord) => string | null;
  isCurrentLocalGeminiAccount: (accountId?: string | null) => boolean;
  formatGeminiLaunchTime: (value?: string | null) => string;
  getGeminiInstanceWarnings: (
    instance: GeminiInstanceRecord
  ) => GeminiInstanceWarning[];
  getGeminiAccountProjectLabel: (accountId?: string | null) => string | null;
  onRefreshGeminiRuntime: () => void;
  onInstanceNameChange: (value: string) => void;
  onInstanceDirChange: (value: string) => void;
  onPickGeminiDir: () => void;
  onAddGeminiInstance: () => void;
  onQuickUpdateGeminiInstance: (
    instance: GeminiInstanceRecord,
    patch: GeminiQuickUpdatePatch,
    successMessage: string
  ) => void;
  onOpenSettings: (instance: GeminiInstanceRecord) => void;
  onCopyGeminiLaunchCommand: (id: string) => void;
  onLaunchGeminiInstance: (id: string) => void;
  onConfirmDeleteGeminiInstance: (id: string) => void;
};

export function SettingsGeminiSection({
  allVisibleGeminiInstances,
  geminiUninitializedCount,
  geminiConflictCount,
  geminiProjectOverrideCount,
  geminiUnboundCount,
  geminiRefreshLoading,
  defaultGeminiInstance,
  currentGeminiAccountId,
  geminiInstanceName,
  geminiInstanceDir,
  geminiInstanceMsg,
  geminiInstanceLoading,
  geminiInstances,
  sortedGeminiInstances,
  getGeminiAccountLabel,
  getEffectiveGeminiAccountId,
  getEffectiveGeminiProjectId,
  isCurrentLocalGeminiAccount,
  formatGeminiLaunchTime,
  getGeminiInstanceWarnings,
  getGeminiAccountProjectLabel,
  onRefreshGeminiRuntime,
  onInstanceNameChange,
  onInstanceDirChange,
  onPickGeminiDir,
  onAddGeminiInstance,
  onQuickUpdateGeminiInstance,
  onOpenSettings,
  onCopyGeminiLaunchCommand,
  onLaunchGeminiInstance,
  onConfirmDeleteGeminiInstance,
}: SettingsGeminiSectionProps) {
  return (
    <>
      <h3 style={{ marginBottom: "var(--space-2)" }}>Gemini 实例</h3>
      <p className="text-muted" style={{ fontSize: "12px", marginBottom: "var(--space-4)" }}>
        管理 Gemini CLI 的默认实例与额外实例目录，支持实例级绑定账号、项目 ID 和启动参数。
      </p>
      <div
        style={{
          background: "var(--surface-sunken)",
          padding: "var(--space-4)",
          borderRadius: "var(--radius-md)",
          marginBottom: "var(--space-6)",
          display: "flex",
          flexDirection: "column",
          gap: "var(--space-3)",
        }}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            gap: 12,
            alignItems: "flex-start",
            flexWrap: "wrap",
          }}
        >
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(5, minmax(120px, 1fr))",
              gap: "var(--space-3)",
              flex: "1 1 760px",
            }}
          >
            <GeminiStatsCard
              label="实例总数"
              value={allVisibleGeminiInstances.length}
              background="rgba(255,255,255,0.04)"
            />
            <GeminiStatsCard
              label="已初始化"
              value={allVisibleGeminiInstances.length - geminiUninitializedCount}
              background="rgba(16,185,129,0.10)"
            />
            <GeminiStatsCard
              label="账号冲突"
              value={geminiConflictCount}
              background="rgba(245,158,11,0.10)"
            />
            <GeminiStatsCard
              label="项目覆盖"
              value={geminiProjectOverrideCount}
              background="rgba(59,130,246,0.10)"
            />
            <GeminiStatsCard
              label="无有效账号"
              value={geminiUnboundCount}
              background="rgba(239,68,68,0.10)"
            />
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", justifyContent: "flex-end" }}>
            <button
              className="btn btn-secondary"
              onClick={onRefreshGeminiRuntime}
              disabled={geminiRefreshLoading}
            >
              {geminiRefreshLoading ? "刷新中..." : "刷新 Gemini 账号状态"}
            </button>
          </div>
        </div>

        {defaultGeminiInstance ? (
          <GeminiInstanceCard
            instance={defaultGeminiInstance}
            currentGeminiAccountId={currentGeminiAccountId}
            getGeminiAccountLabel={getGeminiAccountLabel}
            getEffectiveGeminiAccountId={getEffectiveGeminiAccountId}
            getEffectiveGeminiProjectId={getEffectiveGeminiProjectId}
            getGeminiAccountProjectLabel={getGeminiAccountProjectLabel}
            isCurrentLocalGeminiAccount={isCurrentLocalGeminiAccount}
            formatGeminiLaunchTime={formatGeminiLaunchTime}
            getGeminiInstanceWarnings={getGeminiInstanceWarnings}
            onQuickUpdate={onQuickUpdateGeminiInstance}
            onOpenSettings={onOpenSettings}
            onCopyLaunchCommand={onCopyGeminiLaunchCommand}
            onLaunch={onLaunchGeminiInstance}
          />
        ) : null}

        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 1fr auto",
            gap: "var(--space-3)",
            alignItems: "end",
          }}
        >
          <div>
            <label style={{ display: "block", marginBottom: 4, fontSize: 13 }}>实例名称</label>
            <input
              type="text"
              className="form-input"
              value={geminiInstanceName}
              onChange={(e) => onInstanceNameChange(e.target.value)}
              placeholder="例如：工作区实例 / 沙盒实例"
            />
          </div>
          <div>
            <label style={{ display: "block", marginBottom: 4, fontSize: 13 }}>实例目录</label>
            <input
              type="text"
              className="form-input"
              value={geminiInstanceDir}
              onChange={(e) => onInstanceDirChange(e.target.value)}
              placeholder="选择或粘贴实例目录"
            />
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <button className="btn btn-secondary" onClick={onPickGeminiDir}>
              浏览
            </button>
            <button
              className="btn btn-primary"
              onClick={onAddGeminiInstance}
              disabled={geminiInstanceLoading}
            >
              {geminiInstanceLoading ? "添加中..." : "添加实例"}
            </button>
          </div>
        </div>

        {geminiInstanceMsg ? (
          <div
            style={{
              padding: "var(--space-2)",
              background: "rgba(0,0,0,0.2)",
              borderRadius: "var(--radius-sm)",
              fontSize: 13,
            }}
          >
            {geminiInstanceMsg}
          </div>
        ) : null}

        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
          {geminiInstances.length === 0 ? (
            <div className="text-muted" style={{ fontSize: 13 }}>
              当前还没有额外 Gemini 实例
            </div>
          ) : (
            sortedGeminiInstances.map((instance) => (
              <GeminiInstanceCard
                key={instance.id}
                instance={instance}
                currentGeminiAccountId={currentGeminiAccountId}
                getGeminiAccountLabel={getGeminiAccountLabel}
                getEffectiveGeminiAccountId={getEffectiveGeminiAccountId}
                getEffectiveGeminiProjectId={getEffectiveGeminiProjectId}
                getGeminiAccountProjectLabel={getGeminiAccountProjectLabel}
                isCurrentLocalGeminiAccount={isCurrentLocalGeminiAccount}
                formatGeminiLaunchTime={formatGeminiLaunchTime}
                getGeminiInstanceWarnings={getGeminiInstanceWarnings}
                onQuickUpdate={onQuickUpdateGeminiInstance}
                onOpenSettings={onOpenSettings}
                onCopyLaunchCommand={onCopyGeminiLaunchCommand}
                onLaunch={onLaunchGeminiInstance}
                onConfirmDelete={onConfirmDeleteGeminiInstance}
              />
            ))
          )}
        </div>
      </div>
    </>
  );
}
