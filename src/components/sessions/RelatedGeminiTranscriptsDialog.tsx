import { Search, X } from "lucide-react";
import type { ChatSession } from "./sessionTypes";

type RelatedGeminiTranscriptsDialogProps = {
  open: boolean;
  selectedSession: ChatSession | null;
  relatedGeminiTranscripts: ChatSession[];
  filteredRelatedGeminiTranscripts: ChatSession[];
  relatedSearchQuery: string;
  relatedStatusFilter: "all" | "success" | "failed" | "none";
  onClose: () => void;
  onQueryChange: (value: string) => void;
  onStatusFilterChange: (value: "all" | "success" | "failed" | "none") => void;
  onSelectTranscript: (session: ChatSession) => Promise<void> | void;
  getSessionCwd: (session: ChatSession) => string;
  formatDate: (ts: number) => string;
};

export function RelatedGeminiTranscriptsDialog({
  open,
  selectedSession,
  relatedGeminiTranscripts,
  filteredRelatedGeminiTranscripts,
  relatedSearchQuery,
  relatedStatusFilter,
  onClose,
  onQueryChange,
  onStatusFilterChange,
  onSelectTranscript,
  getSessionCwd,
  formatDate,
}: RelatedGeminiTranscriptsDialogProps) {
  if (!open || !selectedSession) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal session-message-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>关联 Gemini 转录</h2>
          <button className="btn btn-icon" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div className="session-instance-meta">
            <span>工作区：{getSessionCwd(selectedSession) || "未识别"}</span>
            <span>关联转录：{relatedGeminiTranscripts.length}</span>
          </div>
          <div className="session-related-panel">
            <div className="sessions-search-box">
              <Search size={14} />
              <input
                value={relatedSearchQuery}
                onChange={(e) => onQueryChange(e.target.value)}
                placeholder="搜索标题、路径、最近工具..."
              />
              {relatedSearchQuery && (
                <button className="sessions-search-clear" onClick={() => onQueryChange("")}>
                  <X size={14} />
                </button>
              )}
            </div>
            <div className="sessions-tool-filters">
              <button
                className={`sessions-tool-chip ${relatedStatusFilter === "all" ? "active" : ""}`}
                onClick={() => onStatusFilterChange("all")}
              >
                全部
              </button>
              <button
                className={`sessions-tool-chip ${relatedStatusFilter === "success" ? "active" : ""}`}
                onClick={() => onStatusFilterChange("success")}
              >
                最近成功
              </button>
              <button
                className={`sessions-tool-chip ${relatedStatusFilter === "failed" ? "active" : ""}`}
                onClick={() => onStatusFilterChange("failed")}
              >
                最近失败
              </button>
              <button
                className={`sessions-tool-chip ${relatedStatusFilter === "none" ? "active" : ""}`}
                onClick={() => onStatusFilterChange("none")}
              >
                无工具调用
              </button>
            </div>
          </div>
          <div className="session-instance-list">
            {filteredRelatedGeminiTranscripts.length === 0 ? (
              <div className="empty-text">当前筛选条件下没有匹配的关联转录</div>
            ) : (
              filteredRelatedGeminiTranscripts.map((item) => (
                <button
                  key={item.filepath}
                  className="instance-overview-item"
                  onClick={() => onSelectTranscript(item)}
                  title={item.filepath}
                >
                  <div className="instance-overview-item-main">
                    <span className="instance-overview-item-name">{item.title}</span>
                    <span className={`instance-overview-state ${item.latest_tool_status === "success" ? "running" : item.latest_tool_status ? "stopped" : "running"}`}>
                      {item.latest_tool_name
                        ? `${item.latest_tool_name} · ${item.latest_tool_status || "unknown"}`
                        : "无工具调用"}
                    </span>
                  </div>
                  <div className="instance-overview-item-meta">
                    <span>{formatDate(item.updated_at)}</span>
                    <span>{item.messages_count} 条消息</span>
                  </div>
                </button>
              ))
            )}
          </div>
        </div>
        <div className="modal-footer">
          <button className="btn btn-primary" onClick={onClose}>
            关闭
          </button>
        </div>
      </div>
    </div>
  );
}
