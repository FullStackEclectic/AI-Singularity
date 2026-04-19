import { Activity, Cpu, Folder, RefreshCw, Skull } from "lucide-react";
import { SessionsActionStack } from "./SessionsActionStack";
import { SessionsFiltersPanel } from "./SessionsFiltersPanel";
import { SessionGroupsList } from "./SessionGroupsList";
import "./SessionsSidebar.css";
import type {
  CodexInstanceCardRecord,
  ChatSession,
  SessionActionMessage,
  SessionGroup,
  ZombieProcess,
} from "./sessionTypes";
import type { AccountGroup } from "../../types";
import type { SessionOverview } from "./useSessionsDerivedState";

type SessionsSidebarProps = {
  loading: boolean;
  zombies: ZombieProcess[];
  codexInstanceCount: number;
  selectedFilepaths: string[];
  selectedSessionsCount: number;
  selectedGroupsCount: number;
  sessionsCount: number;
  sessions: ChatSession[];
  codexSessionCount: number;
  sessionGroups: SessionGroup[];
  expandedGroups: string[];
  selectedSession: ChatSession | null;
  sourceFilter: "all" | "transcript" | "workspace_history" | "no_transcript";
  sessionSignalFilter: "all" | "tool" | "log" | "failed_tool";
  searchQuery: string;
  toolFilter: string;
  visibleSessionFilepaths: string[];
  visibleProblemFilepaths: string[];
  workspaceHistoryFilepaths: string[];
  noTranscriptFilepaths: string[];
  visibleProblemDirs: string[];
  visibleProblemResumeCommands: string[];
  problemGroupCount: number;
  problemAccountGroupFilter: string;
  problemAccountGroupOptions: AccountGroup[];
  hasUngroupedProblemSessions: boolean;
  toolFilterOptions: { tool: string; count: number }[];
  sessionOverview: SessionOverview;
  currentSessionViewLabel: string;
  actionMessages: SessionActionMessage[];
  codexInstanceCards: CodexInstanceCardRecord[];
  runningCodexInstances: number;
  onRepairCodexIndex: () => void;
  onSyncCodexThreads: () => void;
  onShowCodexInstances: () => void;
  onMoveToTrash: () => void;
  onRefresh: () => void;
  onClearActionMessages: () => void;
  onPushActionMessage: (message: {
    text: string;
    tone: "info" | "success" | "error";
  }) => void;
  onSearchQueryChange: (value: string) => void;
  onToolFilterChange: (value: string) => void;
  onSourceFilterChange: (value: "all" | "transcript" | "workspace_history" | "no_transcript") => void;
  onSessionSignalFilterChange: (value: "all" | "tool" | "log" | "failed_tool") => void;
  onProblemAccountGroupFilterChange: (value: string) => void;
  onExpandVisibleGroups: () => void;
  onExpandProblemGroups: () => void;
  onCollapseAllGroups: () => void;
  onSelectVisibleSessions: () => void;
  onSelectProblemSessions: () => void;
  onSelectWorkspaceHistory: () => void;
  onSelectNoTranscript: () => void;
  onMoveProblemSessionsToTrash: () => void;
  onCopyProblemDirs: () => void;
  onCopyProblemCommands: () => void;
  onLaunchProblemSessions: () => void;
  onClearFilters: () => void;
  onToggleAllSessionsSelected: () => void;
  onClearSelectedFilepaths: () => void;
  onToggleGroupExpanded: (cwd: string) => void;
  onToggleGroupSelected: (group: SessionGroup) => void;
  onToggleSessionSelected: (filepath: string) => void;
  onLoadSession: (session: ChatSession) => void;
  getSessionFlags: (session: ChatSession) => { label: string; tone?: "warning" | "info" | "danger" }[];
  formatCwdLabel: (cwd: string) => string;
  formatDate: (ts: number) => string;
  formatUptime: (seconds: number) => string;
  getSessionCwd: (session: ChatSession) => string;
};

export function SessionsSidebar(props: SessionsSidebarProps) {
  const {
    loading,
    zombies,
    codexInstanceCount,
    selectedFilepaths,
    selectedSessionsCount,
    selectedGroupsCount,
    sessionsCount,
    sessionGroups,
    expandedGroups,
    selectedSession,
    sourceFilter,
    sessionSignalFilter,
    searchQuery,
    toolFilter,
    visibleSessionFilepaths,
    visibleProblemFilepaths,
    workspaceHistoryFilepaths,
    noTranscriptFilepaths,
    visibleProblemDirs,
    visibleProblemResumeCommands,
    problemGroupCount,
    problemAccountGroupFilter,
    problemAccountGroupOptions,
    hasUngroupedProblemSessions,
    toolFilterOptions,
    sessionOverview,
    currentSessionViewLabel,
    actionMessages,
    codexInstanceCards,
    runningCodexInstances,
    onRepairCodexIndex,
    onSyncCodexThreads,
    onShowCodexInstances,
    onMoveToTrash,
    onRefresh,
    onClearActionMessages,
    onPushActionMessage,
    onSearchQueryChange,
    onToolFilterChange,
    onSourceFilterChange,
    onSessionSignalFilterChange,
    onProblemAccountGroupFilterChange,
    onExpandVisibleGroups,
    onExpandProblemGroups,
    onCollapseAllGroups,
    onSelectVisibleSessions,
    onSelectProblemSessions,
    onSelectWorkspaceHistory,
    onSelectNoTranscript,
    onMoveProblemSessionsToTrash,
    onCopyProblemDirs,
    onCopyProblemCommands,
    onLaunchProblemSessions,
    onClearFilters,
    onToggleAllSessionsSelected,
    onClearSelectedFilepaths,
    onToggleGroupExpanded,
    onToggleGroupSelected,
    onToggleSessionSelected,
    onLoadSession,
    getSessionFlags,
    formatCwdLabel,
    formatDate,
    formatUptime,
    getSessionCwd,
  } = props;

  return (
    <div className="sessions-sidebar cyber-sidebar">
      <div className="sessions-header">
        <h2 className="cyber-title-sm">
          <Activity size={16} className="pulse-icon text-accent" /> ZOMBIE_RADAR // 全域劫持雷达
        </h2>
        <div className="sessions-header-actions">
          <button
            className="cyber-icon-btn"
            onClick={onRepairCodexIndex}
            title="修复 Codex 会话索引"
          >
            <Folder size={14} />
          </button>
          <button
            className="cyber-icon-btn"
            onClick={onSyncCodexThreads}
            title="同步 Codex 缺失线程"
            disabled={codexInstanceCount < 2}
          >
            <RefreshCw size={14} />
          </button>
          <button
            className="cyber-icon-btn"
            onClick={onShowCodexInstances}
            title="管理 Codex 实例目录"
          >
            <Cpu size={14} />
          </button>
          <button
            className="cyber-icon-btn danger"
            onClick={onMoveToTrash}
            disabled={selectedFilepaths.length === 0}
            title="将选中的会话移到废纸篓"
          >
            <Skull size={14} />
          </button>
          <button
            className="cyber-icon-btn"
            onClick={onRefresh}
            disabled={loading}
            title="刷新系统探针"
          >
            <RefreshCw size={14} className={loading ? "spin" : ""} />
          </button>
        </div>
      </div>

      <SessionsFiltersPanel
        sessionOverview={sessionOverview}
        currentSessionViewLabel={currentSessionViewLabel}
        selectedFilepathsCount={selectedFilepaths.length}
        searchQuery={searchQuery}
        toolFilter={toolFilter}
        sourceFilter={sourceFilter}
        sessionSignalFilter={sessionSignalFilter}
        problemAccountGroupFilter={problemAccountGroupFilter}
        problemAccountGroupOptions={problemAccountGroupOptions}
        hasUngroupedProblemSessions={hasUngroupedProblemSessions}
        toolFilterOptions={toolFilterOptions}
        sessionGroups={sessionGroups}
        problemGroupCount={problemGroupCount}
        visibleSessionFilepaths={visibleSessionFilepaths}
        visibleProblemFilepaths={visibleProblemFilepaths}
        workspaceHistoryFilepaths={workspaceHistoryFilepaths}
        noTranscriptFilepaths={noTranscriptFilepaths}
        visibleProblemDirs={visibleProblemDirs}
        visibleProblemResumeCommands={visibleProblemResumeCommands}
        onSearchQueryChange={onSearchQueryChange}
        onToolFilterChange={onToolFilterChange}
        onSourceFilterChange={onSourceFilterChange}
        onSessionSignalFilterChange={onSessionSignalFilterChange}
        onProblemAccountGroupFilterChange={onProblemAccountGroupFilterChange}
        onExpandVisibleGroups={onExpandVisibleGroups}
        onExpandProblemGroups={onExpandProblemGroups}
        onCollapseAllGroups={onCollapseAllGroups}
        onSelectVisibleSessions={onSelectVisibleSessions}
        onSelectProblemSessions={onSelectProblemSessions}
        onSelectWorkspaceHistory={onSelectWorkspaceHistory}
        onSelectNoTranscript={onSelectNoTranscript}
        onMoveProblemSessionsToTrash={onMoveProblemSessionsToTrash}
        onCopyProblemDirs={onCopyProblemDirs}
        onCopyProblemCommands={onCopyProblemCommands}
        onLaunchProblemSessions={onLaunchProblemSessions}
        onClearFilters={onClearFilters}
      />

      {selectedFilepaths.length > 0 && (
        <div className="session-batch-bar">
          <div className="session-batch-title">批量处理队列</div>
          <div className="session-batch-meta">
            已选 {selectedSessionsCount} 条会话，覆盖 {selectedGroupsCount} 个工作区
          </div>
          <div className="session-batch-actions">
            <button className="btn btn-ghost btn-xs" onClick={onToggleAllSessionsSelected}>
              {selectedFilepaths.length === sessionsCount ? "取消全选" : "全选全部"}
            </button>
            <button className="btn btn-ghost btn-xs" onClick={onClearSelectedFilepaths}>
              清空选择
            </button>
            <button className="btn btn-danger-ghost btn-xs" onClick={onMoveToTrash}>
              移到废纸篓
            </button>
          </div>
        </div>
      )}

      <SessionsActionStack actionMessages={actionMessages} onClear={onClearActionMessages} />

      <div className="sessions-list">
        <div className="section-divider">
          <span>[ 活跃宿主进程 ] ACTIVE_ZOMBIES</span>
        </div>

        {zombies.length === 0 && !loading && (
          <div className="empty-text">未探测到活动的受体进程</div>
        )}

        {zombies.map((z) => (
          <div key={z.pid} className="zombie-item">
            <div className="zombie-header">
              <span className="zombie-name">
                <Cpu size={12} /> {z.tool_type}
              </span>
              <span className="zombie-pid">PID: {z.pid}</span>
            </div>
            <div className="zombie-meta">
              <span title={z.cwd}>
                CWD: {z.cwd.length > 20 ? "..." + z.cwd.slice(-20) : z.cwd}
              </span>
              <span>UP: {formatUptime(z.active_time_sec)}</span>
            </div>
            <div className="zombie-actions">
              <button
                className="cyber-btn-mini toxic"
                onClick={() =>
                  onPushActionMessage({
                    text: `功能研发中：自动修改 ${z.tool_type} 的路由并热重启进程`,
                    tone: "info",
                  })
                }
              >
                <Skull size={10} /> 注入毒素代理
              </button>
            </div>
          </div>
        ))}

        <div className="section-divider mt-4">
          <span>[ CODEX 多实例 ] INSTANCE_MATRIX</span>
        </div>

        <div className="instance-overview-card">
          <div className="instance-overview-stats">
            <div className="instance-overview-stat">
              <span className="instance-overview-value">{codexInstanceCards.length}</span>
              <span className="instance-overview-label">实例</span>
            </div>
            <div className="instance-overview-stat">
              <span className="instance-overview-value">{runningCodexInstances}</span>
              <span className="instance-overview-label">运行中</span>
            </div>
            <div className="instance-overview-stat">
              <span className="instance-overview-value">
                {props.codexSessionCount}
              </span>
              <span className="instance-overview-label">Codex 会话</span>
            </div>
          </div>
          <div className="instance-overview-list">
            {codexInstanceCards.map((item) => (
              <button
                key={item.id}
                className="instance-overview-item"
                onClick={onShowCodexInstances}
                title={item.user_data_dir}
              >
                <div className="instance-overview-item-main">
                  <span className="instance-overview-item-name">
                    {item.name}
                    {item.is_default ? " · 默认" : ""}
                  </span>
                  <span
                    className={`instance-overview-state ${
                      item.running ? "running" : "stopped"
                    }`}
                  >
                    {item.running ? "运行中" : "未运行"}
                  </span>
                </div>
                <div className="instance-overview-item-meta">
                  <span>{item.sessionCount} 条会话</span>
                  <span>{formatCwdLabel(item.user_data_dir)}</span>
                </div>
              </button>
            ))}
          </div>
          <div className="instance-overview-actions">
            <button className="btn btn-ghost btn-xs" onClick={onShowCodexInstances}>
              打开实例面板
            </button>
          </div>
        </div>

        <SessionGroupsList
          sessions={props.sessions}
          loading={loading}
          sessionGroups={sessionGroups}
          expandedGroups={expandedGroups}
          selectedFilepaths={selectedFilepaths}
          selectedSession={selectedSession}
          sourceFilter={sourceFilter}
          sessionSignalFilter={sessionSignalFilter}
          searchQuery={searchQuery}
          toolFilter={toolFilter}
          onClearFilters={onClearFilters}
          onToggleGroupExpanded={onToggleGroupExpanded}
          onToggleGroupSelected={onToggleGroupSelected}
          onToggleSessionSelected={onToggleSessionSelected}
          onLoadSession={onLoadSession}
          getSessionFlags={getSessionFlags}
          formatCwdLabel={formatCwdLabel}
          formatDate={formatDate}
          getSessionCwd={getSessionCwd}
        />
      </div>
    </div>
  );
}
