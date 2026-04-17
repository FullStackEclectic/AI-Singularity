import type { ChatMessage } from "./sessionTypes";

type ExpandedMessageModalProps = {
  message: ChatMessage | null;
  onClose: () => void;
  onOpenSource: (message: ChatMessage) => void;
  onCopyFull: (message: ChatMessage) => void;
  formatMessageTime: (ts?: number) => string;
};

export function ExpandedMessageModal({
  message,
  onClose,
  onOpenSource,
  onCopyFull,
  formatMessageTime,
}: ExpandedMessageModalProps) {
  if (!message) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal session-message-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>消息全文</h2>
          <button className="btn btn-icon" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div className="session-instance-meta">
            <span>角色：{message.role}</span>
            {message.timestamp ? <span>时间：{formatMessageTime(message.timestamp)}</span> : null}
            {message.source_path ? <span>来源：{message.source_path}</span> : null}
          </div>
          <pre className="session-message-fulltext">{message.full_content || message.content}</pre>
        </div>
        <div className="modal-footer">
          {message.source_path ? (
            <button className="btn btn-secondary" onClick={() => onOpenSource(message)}>
              打开源文件
            </button>
          ) : null}
          <button
            className="btn btn-secondary"
            onClick={() =>
              onCopyFull({
                ...message,
                content: message.full_content || message.content,
              })
            }
          >
            复制全文
          </button>
          <button className="btn btn-primary" onClick={onClose}>
            关闭
          </button>
        </div>
      </div>
    </div>
  );
}
