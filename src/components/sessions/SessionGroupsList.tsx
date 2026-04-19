import { ChevronDown, ChevronRight, Folder, MessageSquare } from "lucide-react";
import type { ChatSession, SessionGroup } from "./sessionTypes";
import "./SessionGroupsList.css";

type SessionGroupsListProps = {
  sessions: ChatSession[];
  loading: boolean;
  sessionGroups: SessionGroup[];
  expandedGroups: string[];
  selectedFilepaths: string[];
  selectedSession: ChatSession | null;
  sourceFilter: "all" | "transcript" | "workspace_history" | "no_transcript";
  sessionSignalFilter: "all" | "tool" | "log" | "failed_tool";
  searchQuery: string;
  toolFilter: string;
  onClearFilters: () => void;
  onToggleGroupExpanded: (cwd: string) => void;
  onToggleGroupSelected: (group: SessionGroup) => void;
  onToggleSessionSelected: (filepath: string) => void;
  onLoadSession: (session: ChatSession) => void;
  getSessionFlags: (session: ChatSession) => { label: string; tone?: "warning" | "info" | "danger" }[];
  formatCwdLabel: (cwd: string) => string;
  formatDate: (ts: number) => string;
  getSessionCwd: (session: ChatSession) => string;
};

export function SessionGroupsList({
  sessions,
  loading,
  sessionGroups,
  expandedGroups,
  selectedFilepaths,
  selectedSession,
  sourceFilter,
  sessionSignalFilter,
  searchQuery,
  toolFilter,
  onClearFilters,
  onToggleGroupExpanded,
  onToggleGroupSelected,
  onToggleSessionSelected,
  onLoadSession,
  getSessionFlags,
  formatCwdLabel,
  formatDate,
  getSessionCwd,
}: SessionGroupsListProps) {
  return (
    <>
      <div className="section-divider mt-4">
        <span>[ 沉睡数据缓冲 ] OFFLINE_CACHE</span>
      </div>

      {sessions.length === 0 && !loading && (
        <div className="empty-text">未发现沉睡的历史数据</div>
      )}

      {sessions.length > 0 && sessionGroups.length === 0 && !loading && (
        <div className="empty-text">
          {sourceFilter === "workspace_history"
            ? "当前没有匹配的工作区历史会话"
            : sourceFilter === "no_transcript"
              ? "当前没有匹配的无转录会话"
              : sourceFilter === "transcript"
                ? "当前没有匹配的有转录会话"
                : "当前没有匹配的会话"}
          {(sourceFilter !== "all" || sessionSignalFilter !== "all" || searchQuery.trim() || toolFilter !== "all") && (
            <>
              <br />
              <button className="btn btn-ghost btn-xs" onClick={onClearFilters}>
                清除当前视图筛选
              </button>
            </>
          )}
        </div>
      )}

      {sessionGroups.map((group) => {
        const isExpanded = expandedGroups.includes(group.cwd);
        const allSelected =
          group.sessions.length > 0 &&
          group.sessions.every((session) => selectedFilepaths.includes(session.filepath));
        return (
          <div key={group.cwd} className="session-group">
            <button
              className="session-group-header"
              onClick={() => onToggleGroupExpanded(group.cwd)}
              title={group.cwd}
            >
              <div className="session-group-left">
                <input
                  type="checkbox"
                  checked={allSelected}
                  onChange={(e) => {
                    e.stopPropagation();
                    onToggleGroupSelected(group);
                  }}
                  onClick={(e) => e.stopPropagation()}
                />
                {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                <Folder size={14} />
                <div className="session-group-texts">
                  <span className="session-group-label">{group.label}</span>
                  <span className="session-group-path">{formatCwdLabel(group.cwd)}</span>
                </div>
                <span className="session-group-count">
                  {group.sessions.length}
                  {group.sessions.some((session) => selectedFilepaths.includes(session.filepath))
                    ? ` / 已选 ${group.sessions.filter((session) => selectedFilepaths.includes(session.filepath)).length}`
                    : ""}
                </span>
              </div>
              <span className="session-group-time">{formatDate(group.updated_at)}</span>
            </button>
            {isExpanded && (
              <div className="session-group-children">
                {group.sessions.map((session) => (
                  <div
                    key={session.filepath}
                    className={`session-item ${selectedSession?.filepath === session.filepath ? "active" : ""}`}
                    onClick={() => onLoadSession(session)}
                  >
                    <div className="session-item-select">
                      <input
                        type="checkbox"
                        checked={selectedFilepaths.includes(session.filepath)}
                        onChange={(e) => {
                          e.stopPropagation();
                          onToggleSessionSelected(session.filepath);
                        }}
                        onClick={(e) => e.stopPropagation()}
                      />
                    </div>
                    <div className="session-item-title">{session.title || "Unnamed Chat"}</div>
                    {getSessionFlags(session).length > 0 && (
                      <div className="session-item-flags">
                        {getSessionFlags(session).map((flag) => (
                          <span key={flag.label} className={`session-flag ${flag.tone || "info"}`}>
                            {flag.label}
                          </span>
                        ))}
                      </div>
                    )}
                    <div className="session-item-meta">
                      <span>
                        {session.tool_type || "Unknown"}
                        {session.instance_name ? ` / ${session.instance_name}` : ""}
                      </span>
                      <span className="text-accent"><MessageSquare size={10} /> {session.messages_count} logs</span>
                      <span>L: {formatDate(session.updated_at)}</span>
                    </div>
                    <div className="session-item-path" title={getSessionCwd(session) || session.filepath}>
                      {getSessionCwd(session) || session.filepath}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </>
  );
}
