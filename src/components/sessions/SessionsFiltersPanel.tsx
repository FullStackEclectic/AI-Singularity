import { Search, X } from "lucide-react";
import type { SessionGroup } from "./sessionTypes";
import type { AccountGroup } from "../../types";
import "./SessionsFiltersPanel.css";

type SessionsFiltersPanelProps = {
  sessionOverview: {
    total: number;
    visible: number;
    workspaceHistory: number;
    noTranscript: number;
    visibleProblem: number;
  };
  currentSessionViewLabel: string;
  selectedFilepathsCount: number;
  searchQuery: string;
  toolFilter: string;
  sourceFilter: "all" | "transcript" | "workspace_history" | "no_transcript";
  sessionSignalFilter: "all" | "tool" | "log" | "failed_tool";
  problemAccountGroupFilter: string;
  problemAccountGroupOptions: AccountGroup[];
  hasUngroupedProblemSessions: boolean;
  toolFilterOptions: { tool: string; count: number }[];
  sessionGroups: SessionGroup[];
  problemGroupCount: number;
  visibleSessionFilepaths: string[];
  visibleProblemFilepaths: string[];
  workspaceHistoryFilepaths: string[];
  noTranscriptFilepaths: string[];
  visibleProblemDirs: string[];
  visibleProblemResumeCommands: string[];
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
};

export function SessionsFiltersPanel({
  sessionOverview,
  currentSessionViewLabel,
  selectedFilepathsCount,
  searchQuery,
  toolFilter,
  sourceFilter,
  sessionSignalFilter,
  problemAccountGroupFilter,
  problemAccountGroupOptions,
  hasUngroupedProblemSessions,
  toolFilterOptions,
  sessionGroups,
  problemGroupCount,
  visibleSessionFilepaths,
  visibleProblemFilepaths,
  workspaceHistoryFilepaths,
  noTranscriptFilepaths,
  visibleProblemDirs,
  visibleProblemResumeCommands,
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
}: SessionsFiltersPanelProps) {
  return (
    <div className="sessions-filter-bar">
      <div className="sessions-overview-stats">
        <div className="sessions-overview-stat">
          <span className="sessions-overview-value">{sessionOverview.total}</span>
          <span className="sessions-overview-label">总会话</span>
        </div>
        <div className="sessions-overview-stat">
          <span className="sessions-overview-value">{sessionOverview.visible}</span>
          <span className="sessions-overview-label">当前可见</span>
        </div>
        <div className="sessions-overview-stat">
          <span className="sessions-overview-value">{sessionOverview.workspaceHistory}</span>
          <span className="sessions-overview-label">工作区历史</span>
        </div>
        <div className="sessions-overview-stat">
          <span className="sessions-overview-value">{sessionOverview.noTranscript}</span>
          <span className="sessions-overview-label">无转录</span>
        </div>
      </div>
      <div className="sessions-view-bar">
        <span className="sessions-view-chip">当前视图：{currentSessionViewLabel}</span>
        <span className="sessions-view-chip warning">可见问题会话：{sessionOverview.visibleProblem}</span>
        <span className="sessions-view-chip">已选：{selectedFilepathsCount}</span>
      </div>
      <div className="sessions-search-box">
        <Search size={14} />
        <input
          value={searchQuery}
          onChange={(e) => onSearchQueryChange(e.target.value)}
          placeholder="搜索标题、目录、实例名..."
        />
        {searchQuery && (
          <button className="sessions-search-clear" onClick={() => onSearchQueryChange("")}>
            <X size={14} />
          </button>
        )}
      </div>
      <div className="sessions-tool-filters">
        <button
          className={`sessions-tool-chip ${toolFilter === "all" ? "active" : ""}`}
          onClick={() => onToolFilterChange("all")}
        >
          全部 · {sessionOverview.total}
        </button>
        {toolFilterOptions.map((item) => (
          <button
            key={item.tool}
            className={`sessions-tool-chip ${toolFilter === item.tool ? "active" : ""}`}
            onClick={() => onToolFilterChange(item.tool)}
          >
            {item.tool} · {item.count}
          </button>
        ))}
      </div>
      <div className="sessions-tool-filters">
        <button
          className={`sessions-tool-chip ${sourceFilter === "all" ? "active" : ""}`}
          onClick={() => onSourceFilterChange("all")}
        >
          全部来源
        </button>
        <button
          className={`sessions-tool-chip ${sourceFilter === "transcript" ? "active" : ""}`}
          onClick={() => onSourceFilterChange("transcript")}
        >
          有转录
        </button>
        <button
          className={`sessions-tool-chip ${sourceFilter === "workspace_history" ? "active" : ""}`}
          onClick={() => onSourceFilterChange("workspace_history")}
        >
          工作区历史
        </button>
        <button
          className={`sessions-tool-chip ${sourceFilter === "no_transcript" ? "active" : ""}`}
          onClick={() => onSourceFilterChange("no_transcript")}
        >
          无转录
        </button>
      </div>
      <div className="sessions-tool-filters">
        <button
          className={`sessions-tool-chip ${sessionSignalFilter === "all" ? "active" : ""}`}
          onClick={() => onSessionSignalFilterChange("all")}
        >
          全部信号
        </button>
        <button
          className={`sessions-tool-chip ${sessionSignalFilter === "tool" ? "active" : ""}`}
          onClick={() => onSessionSignalFilterChange("tool")}
        >
          含工具调用
        </button>
        <button
          className={`sessions-tool-chip ${sessionSignalFilter === "log" ? "active" : ""}`}
          onClick={() => onSessionSignalFilterChange("log")}
        >
          含日志事件
        </button>
        <button
          className={`sessions-tool-chip ${sessionSignalFilter === "failed_tool" ? "active" : ""}`}
          onClick={() => onSessionSignalFilterChange("failed_tool")}
        >
          最近工具失败
        </button>
      </div>
      <div className="sessions-tool-filters">
        <button
          className={`sessions-tool-chip ${problemAccountGroupFilter === "all" ? "active" : ""}`}
          onClick={() => onProblemAccountGroupFilterChange("all")}
        >
          问题账号分组 · 全部
        </button>
        {hasUngroupedProblemSessions && (
          <button
            className={`sessions-tool-chip ${problemAccountGroupFilter === "__ungrouped__" ? "active" : ""}`}
            onClick={() => onProblemAccountGroupFilterChange("__ungrouped__")}
          >
            未分组
          </button>
        )}
        {problemAccountGroupOptions.map((group) => (
          <button
            key={group.id}
            className={`sessions-tool-chip ${problemAccountGroupFilter === group.id ? "active" : ""}`}
            onClick={() => onProblemAccountGroupFilterChange(group.id)}
          >
            {group.name}
          </button>
        ))}
      </div>
      <div className="sessions-filter-actions">
        <button
          className="sessions-tool-chip"
          onClick={onExpandVisibleGroups}
          disabled={sessionGroups.length === 0}
        >
          展开当前可见工作区
        </button>
        <button
          className="sessions-tool-chip"
          onClick={onExpandProblemGroups}
          disabled={problemGroupCount === 0}
        >
          只展开问题工作区
        </button>
        <button
          className="sessions-tool-chip"
          onClick={onCollapseAllGroups}
          disabled={sessionGroups.length === 0}
        >
          收起全部工作区
        </button>
        <button
          className="sessions-tool-chip"
          onClick={onSelectVisibleSessions}
          disabled={visibleSessionFilepaths.length === 0}
        >
          只选当前可见
        </button>
        <button
          className="sessions-tool-chip"
          onClick={onSelectProblemSessions}
          disabled={visibleProblemFilepaths.length === 0}
        >
          只选问题会话
        </button>
        <button
          className="sessions-tool-chip"
          onClick={onSelectWorkspaceHistory}
          disabled={workspaceHistoryFilepaths.length === 0}
        >
          只选工作区历史
        </button>
        <button
          className="sessions-tool-chip"
          onClick={onSelectNoTranscript}
          disabled={noTranscriptFilepaths.length === 0}
        >
          只选无转录
        </button>
        <button
          className="sessions-tool-chip danger"
          onClick={onMoveProblemSessionsToTrash}
          disabled={visibleProblemFilepaths.length === 0}
        >
          当前问题会话移到废纸篓
        </button>
        <button
          className="sessions-tool-chip"
          onClick={onCopyProblemDirs}
          disabled={visibleProblemDirs.length === 0}
        >
          复制问题会话目录
        </button>
        <button
          className="sessions-tool-chip"
          onClick={onCopyProblemCommands}
          disabled={visibleProblemResumeCommands.length === 0}
        >
          复制问题会话命令
        </button>
        <button
          className="sessions-tool-chip"
          onClick={onLaunchProblemSessions}
          disabled={visibleProblemFilepaths.length === 0}
        >
          拉起问题会话终端(前3)
        </button>
        {(sourceFilter !== "all" ||
          sessionSignalFilter !== "all" ||
          problemAccountGroupFilter !== "all" ||
          searchQuery.trim() ||
          toolFilter !== "all") && (
          <button className="sessions-tool-chip" onClick={onClearFilters}>
            清除视图筛选
          </button>
        )}
      </div>
      <div className="sessions-filter-hint">
        {sourceFilter === "workspace_history" && "当前视图更适合批量排查工作区索引来源，不代表完整聊天转录。"}
        {sourceFilter === "no_transcript" && "当前视图适合优先检查无转录会话，再决定是否移到废纸篓或保留索引。"}
        {sourceFilter === "transcript" && "当前视图只保留有实际转录内容的会话。"}
        {sourceFilter === "all" && "可以先按来源或工具筛一轮，再用批量栏统一处理。"}
        {visibleProblemFilepaths.length > 3 && " 批量拉起终端默认限制前 3 条，避免一次打开过多窗口。"}
      </div>
    </div>
  );
}
