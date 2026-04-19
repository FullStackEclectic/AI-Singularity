import type { SessionActionMessage } from "./sessionTypes";
import "./SessionsActionStack.css";

type SessionsActionStackProps = {
  actionMessages: SessionActionMessage[];
  onClear: () => void;
};

export function SessionsActionStack({
  actionMessages,
  onClear,
}: SessionsActionStackProps) {
  if (actionMessages.length === 0) {
    return null;
  }

  return (
    <div className="session-action-stack">
      <div className="session-action-stack-header">
        <span>操作结果</span>
        <button className="btn btn-ghost btn-xs" onClick={onClear}>
          清空
        </button>
      </div>
      {actionMessages.map((item) => (
        <div key={item.id} className={`session-action-bar ${item.tone ?? "info"}`}>
          <div className="session-action-text">{item.text}</div>
          <div className="session-action-time">
            {new Date(item.createdAt).toLocaleTimeString("zh-CN", {
              hour: "2-digit",
              minute: "2-digit",
              second: "2-digit",
            })}
          </div>
        </div>
      ))}
    </div>
  );
}
