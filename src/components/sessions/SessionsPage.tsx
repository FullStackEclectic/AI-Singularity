import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MessageSquare, RefreshCw } from "lucide-react";
import "./SessionsPage.css";

interface ChatSession {
  id: string;
  title: string;
  created_at: number;
  updated_at: number;
  messages_count: number;
  filepath: string;
}

interface ChatMessage {
  role: string;
  content: string;
  timestamp?: number;
}

export default function SessionsPage() {
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedSession, setSelectedSession] = useState<ChatSession | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [messagesLoading, setMessagesLoading] = useState(false);

  const fetchSessions = async () => {
    setLoading(true);
    try {
      const data = await invoke<ChatSession[]>("list_sessions");
      setSessions(data);
    } catch (e) {
      console.error("Failed to load sessions:", e);
    } finally {
      setLoading(false);
    }
  };

  const loadSession = async (session: ChatSession) => {
    setSelectedSession(session);
    setMessagesLoading(true);
    try {
      const msgData = await invoke<ChatMessage[]>("get_session_details", { filepath: session.filepath });
      setMessages(msgData);
    } catch (e) {
      console.error("Failed to load session details:", e);
      setMessages([]);
    } finally {
      setMessagesLoading(false);
    }
  };

  useEffect(() => {
    fetchSessions();
  }, []);

  const formatDate = (ts: number) => {
    if (ts === 0) return "Unknown";
    const d = new Date(ts * 1000);
    return `${d.getMonth() + 1}/${d.getDate()} ${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}`;
  };

  return (
    <div className="sessions-page">
      {/* 侧边栏结构 */}
      <div className="sessions-sidebar">
        <div className="sessions-header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}">
          <h2 style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <MessageSquare size={16} /> 离线会话缓冲
          </h2>
          <button className="btn-icon" onClick={fetchSessions} disabled={loading} title="刷新缓存列表">
            <RefreshCw size={14} className={loading ? "spin" : ""} />
          </button>
        </div>
        
        <div className="sessions-list">
          {sessions.length === 0 && !loading && (
            <div style={{ textAlign: "center", color: "var(--color-text-secondary)", marginTop: 20, fontSize: 13 }}>
              未在系统找到 .claude 缓存
            </div>
          )}
          {sessions.map((s) => (
            <div 
              key={s.filepath} 
              className={`session-item ${selectedSession?.filepath === s.filepath ? 'active' : ''}`}
              onClick={() => loadSession(s)}
            >
              <div className="session-item-title">{s.title || "Unnamed Chat"}</div>
              <div className="session-item-meta">
                <span>💬 {s.messages_count} msgs</span>
                <span>{formatDate(s.updated_at)}</span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* 详情主视口 */}
      <div className="session-content">
        {selectedSession ? (
          <>
            <div className="session-content-header">
              <h3>{selectedSession.title}</h3>
              <div style={{ fontSize: 12, color: 'var(--color-text-secondary)', marginTop: 4 }}>
                {selectedSession.filepath}
              </div>
            </div>
            
            <div className="chat-messages">
              {messagesLoading ? (
                <div className="empty-chat">解析会话中...</div>
              ) : messages.length === 0 ? (
                <div className="empty-chat">暂无聊天记录</div>
              ) : (
                messages.map((m, idx) => {
                  const isUser = m.role === 'user';
                  const isSys = m.role === 'system';
                  return (
                    <div key={idx} className={`chat-bubble ${isSys ? 'system' : isUser ? 'user' : 'assistant'}`}>
                      {m.role !== 'system' && (
                        <div className="chat-bubble-role">{m.role}</div>
                      )}
                      {m.content}
                    </div>
                  );
                })
              )}
            </div>
          </>
        ) : (
          <div className="empty-chat">请从左侧选择一个会话查看日志</div>
        )}
      </div>
    </div>
  );
}
