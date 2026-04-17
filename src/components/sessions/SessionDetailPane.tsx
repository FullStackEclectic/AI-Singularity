import { Activity, ChevronDown, ChevronRight, Copy, Folder, Terminal } from "lucide-react";
import type { ChatMessage, ChatSession } from "./sessionTypes";

type SessionDetailPaneProps = {
  selectedSession: ChatSession | null;
  visibleMessages: ChatMessage[];
  messagesLoading: boolean;
  messageViewMode: "all" | "tool";
  relatedGeminiTranscripts: ChatSession[];
  selectedSessionToolOutputDir: string | null;
  collapsedToolKeys: string[];
  onSetMessageViewMode: (mode: "all" | "tool") => void;
  onShowCodexInstances: () => void;
  onJumpToRelatedGeminiTranscript: () => void;
  onShowRelatedGeminiDialog: () => void;
  onCopyDir: (session: ChatSession) => void;
  onCopyCmd: (session: ChatSession) => void;
  onCopyVisibleMessages: (onlyTool?: boolean) => void;
  onOpenToolOutputDir: () => void;
  onLaunchTerminal: (session: ChatSession) => void;
  onToggleToolMessageCollapsed: (key: string) => void;
  onCopyMessageBlock: (message: ChatMessage) => void;
  onExpandMessage: (message: ChatMessage) => void;
  formatMessageTime: (ts?: number) => string;
  getSessionCwd: (session: ChatSession) => string;
  isToolRelatedMessage: (message: ChatMessage) => boolean;
};

export function SessionDetailPane({
  selectedSession,
  visibleMessages,
  messagesLoading,
  messageViewMode,
  relatedGeminiTranscripts,
  selectedSessionToolOutputDir,
  collapsedToolKeys,
  onSetMessageViewMode,
  onShowCodexInstances,
  onJumpToRelatedGeminiTranscript,
  onShowRelatedGeminiDialog,
  onCopyDir,
  onCopyCmd,
  onCopyVisibleMessages,
  onOpenToolOutputDir,
  onLaunchTerminal,
  onToggleToolMessageCollapsed,
  onCopyMessageBlock,
  onExpandMessage,
  formatMessageTime,
  getSessionCwd,
  isToolRelatedMessage,
}: SessionDetailPaneProps) {
  if (!selectedSession) {
    return (
      <div className="empty-chat cyber-placeholder">
        <Activity size={48} className="placeholder-icon pulse-icon" />
        <div>SYSTEM_STANDBY</div>
        <div className="text-muted" style={{ fontSize: 12, marginTop: 8 }}>请选择左侧内存残片进行逆向还原</div>
      </div>
    );
  }

  return (
    <>
      <div className="session-content-header cyber-header-box" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div>
          <h3 className="glow-text">{selectedSession.title}</h3>
          <div className="session-path-text">{selectedSession.filepath}</div>
          <div className="session-location-row">
            <span className="session-location-chip">工作区：{getSessionCwd(selectedSession) || "未识别"}</span>
            <span className="session-location-chip">工具：{selectedSession.tool_type || "Unknown"}</span>
            <span className="session-location-chip">消息：{selectedSession.messages_count}</span>
            <span className="session-location-chip">当前显示：{visibleMessages.length}</span>
            {selectedSession.has_tool_calls ? <span className="session-location-chip">含工具调用</span> : null}
            {selectedSession.has_log_events ? <span className="session-location-chip">含日志事件</span> : null}
            {selectedSession.source_kind === "workspace_history" ? (
              <span className="session-location-chip warning">来源：工作区历史索引</span>
            ) : null}
          </div>
          {selectedSession.tool_type === "GeminiCLI" && selectedSession.messages_count === 0 ? (
            <div className="session-source-note">
              当前这条 Gemini 记录来自工作区历史索引；完整聊天转录会优先从 `~/.gemini/tmp/*/chats/session-*.json` 读取。
              {relatedGeminiTranscripts.length > 0 ? (
                <>
                  {" "}当前同工作区已找到 {relatedGeminiTranscripts.length} 条真实转录，可直接跳转。
                </>
              ) : null}
            </div>
          ) : null}
          {selectedSession.tool_type === "Codex" ? (
            <button className="session-instance-chip clickable" onClick={onShowCodexInstances}>
              实例：{selectedSession.instance_name || "默认实例"}
            </button>
          ) : null}
          {selectedSession.source_kind === "workspace_history" && relatedGeminiTranscripts.length > 0 ? (
            <div className="session-related-actions">
              <button className="session-instance-chip clickable" onClick={onJumpToRelatedGeminiTranscript}>
                跳到最新转录 · {relatedGeminiTranscripts[0]?.title}
              </button>
              <button className="session-instance-chip clickable" onClick={onShowRelatedGeminiDialog}>
                查看全部关联转录 · {relatedGeminiTranscripts.length}
              </button>
            </div>
          ) : null}
        </div>
        <div className="session-actions" style={{ display: "flex", gap: "8px" }}>
          <button className="btn btn-ghost btn-sm" onClick={() => onCopyDir(selectedSession)} title="复制会话对应的工作目录">
            <Folder size={14} style={{ marginRight: 4 }} /> 目录
          </button>
          <button className="btn btn-ghost btn-sm" onClick={() => onCopyCmd(selectedSession)} title="生成恢复脚本并复制剪贴板">
            <Copy size={14} style={{ marginRight: 4 }} /> 复制指令
          </button>
          <button className="btn btn-ghost btn-sm" onClick={() => onCopyVisibleMessages(false)} title="复制当前可见消息">
            <Copy size={14} style={{ marginRight: 4 }} /> 复制消息
          </button>
          <button className="btn btn-ghost btn-sm" onClick={() => onCopyVisibleMessages(true)} title="复制当前工具调用摘要">
            <Copy size={14} style={{ marginRight: 4 }} /> 复制工具摘要
          </button>
          {selectedSessionToolOutputDir ? (
            <button className="btn btn-ghost btn-sm" onClick={onOpenToolOutputDir} title="打开 Gemini 工具输出目录">
              <Folder size={14} style={{ marginRight: 4 }} /> 工具输出
            </button>
          ) : null}
          <button className="btn btn-primary btn-sm" onClick={() => onLaunchTerminal(selectedSession)} title="在新终端以当前目录直接拉起">
            <Terminal size={14} style={{ marginRight: 4 }} /> 外置终端拉起
          </button>
        </div>
      </div>

      <div className="session-detail-toolbar">
        <div className="sessions-tool-filters">
          <button
            className={`sessions-tool-chip ${messageViewMode === "all" ? "active" : ""}`}
            onClick={() => onSetMessageViewMode("all")}
          >
            全部消息
          </button>
          <button
            className={`sessions-tool-chip ${messageViewMode === "tool" ? "active" : ""}`}
            onClick={() => onSetMessageViewMode("tool")}
          >
            只看工具调用
          </button>
        </div>
        {messageViewMode === "tool" ? (
          <div className="session-detail-hint">
            当前视图会优先保留 Gemini 的工具调用摘要、logs 事件和 tool-outputs 上下文。
          </div>
        ) : null}
      </div>

      <div className="chat-messages cyber-chat-box">
        {messagesLoading ? (
          <div className="empty-chat glitch-text">解析内存残片中... DECRYPTING...</div>
        ) : visibleMessages.length === 0 ? (
          <div className="empty-chat text-muted">残片已清空或无法还原</div>
        ) : (
          visibleMessages.map((message, idx) => {
            const isUser = message.role === "user";
            const isSys = message.role === "system";
            const isTool = message.role === "tool" || isToolRelatedMessage(message);
            const collapseKey = `${selectedSession.filepath}:${idx}`;
            const shouldAutoCollapse = isTool && (message.content.length > 280 || !!message.full_content);
            const collapsed = shouldAutoCollapse && !collapsedToolKeys.includes(collapseKey);
            const visibleContent = collapsed ? `${message.content.slice(0, 280)}...` : message.content;
            return (
              <div key={idx} className={`chat-bubble ${isSys ? "system" : isUser ? "user" : isTool ? "tool" : "assistant"}`}>
                <div className="chat-bubble-head">
                  {message.role !== "system" ? (
                    <div className="chat-bubble-role">{message.role.toUpperCase()}</div>
                  ) : (
                    <div className="chat-bubble-role muted">SYSTEM</div>
                  )}
                  <div className="chat-bubble-head-actions">
                    {message.timestamp ? <div className="chat-bubble-time">{formatMessageTime(message.timestamp)}</div> : null}
                    <button
                      className="chat-bubble-copy"
                      onClick={() => onCopyMessageBlock(message)}
                      title="复制当前消息块"
                    >
                      <Copy size={12} />
                    </button>
                    {shouldAutoCollapse ? (
                      <button
                        className="chat-bubble-copy"
                        onClick={() => onToggleToolMessageCollapsed(collapseKey)}
                        title={collapsed ? "展开当前工具块" : "收起当前工具块"}
                      >
                        {collapsed ? <ChevronRight size={12} /> : <ChevronDown size={12} />}
                      </button>
                    ) : null}
                    {message.full_content || message.source_path ? (
                      <button
                        className="chat-bubble-copy"
                        onClick={() => onExpandMessage(message)}
                        title="查看完整内容"
                      >
                        <Folder size={12} />
                      </button>
                    ) : null}
                  </div>
                </div>
                <div className="chat-bubble-text">{visibleContent}</div>
              </div>
            );
          })
        )}
      </div>
    </>
  );
}
