import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MessageSquare, RefreshCw, Cpu, Activity, Skull } from "lucide-react";
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

interface ZombieProcess {
  pid: number;
  name: string;
  command: string;
  active_time_sec: number;
  tool_type: string;
  cwd: string;
}

export default function SessionsPage() {
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [zombies, setZombies] = useState<ZombieProcess[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedSession, setSelectedSession] = useState<ChatSession | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [messagesLoading, setMessagesLoading] = useState(false);

  const fetchAll = async () => {
    setLoading(true);
    try {
      const [sessData, zombieData] = await Promise.all([
        invoke<ChatSession[]>("list_sessions"),
        invoke<ZombieProcess[]>("scan_zombies")
      ]);
      setSessions(sessData);
      setZombies(zombieData);
    } catch (e) {
      console.error("Failed to load sessions/zombies:", e);
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
    fetchAll();
    const interval = setInterval(fetchAll, 10000);
    return () => clearInterval(interval);
  }, []);

  const formatDate = (ts: number) => {
    if (ts === 0) return "Unknown";
    const d = new Date(ts * 1000);
    return `${d.getMonth() + 1}/${d.getDate()} ${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}`;
  };

  const formatUptime = (secs: number) => {
    if (secs < 60) return `${secs}s`;
    const mins = Math.floor(secs / 60);
    if (mins < 60) return `${mins}m`;
    return `${(mins / 60).toFixed(1)}h`;
  };

  return (
    <div className="sessions-page cyberpunk-theme">
      {/* 侧边栏结构 */}
      <div className="sessions-sidebar cyber-sidebar">
        <div className="sessions-header">
          <h2 className="cyber-title-sm">
            <Activity size={16} className="pulse-icon text-accent" /> ZOMBIE_RADAR // 全域劫持雷达
          </h2>
          <button className="cyber-icon-btn" onClick={fetchAll} disabled={loading} title="刷新系统探针">
            <RefreshCw size={14} className={loading ? "spin" : ""} />
          </button>
        </div>
        
        <div className="sessions-list">
          {/* Zombies Section */}
          <div className="section-divider">
            <span>[ 活跃宿主进程 ] ACTIVE_ZOMBIES</span>
          </div>
          
          {zombies.length === 0 && !loading && (
            <div className="empty-text">未探测到活动的受体进程</div>
          )}
          
          {zombies.map((z) => (
            <div key={z.pid} className="zombie-item">
              <div className="zombie-header">
                <span className="zombie-name"><Cpu size={12}/> {z.tool_type}</span>
                <span className="zombie-pid">PID: {z.pid}</span>
              </div>
              <div className="zombie-meta">
                <span title={z.cwd}>CWD: {z.cwd.length > 20 ? '...'+z.cwd.slice(-20) : z.cwd}</span>
                <span>UP: {formatUptime(z.active_time_sec)}</span>
              </div>
              <div className="zombie-actions">
                <button className="cyber-btn-mini toxic" onClick={() => alert("功能研发中：自动修改 " + z.tool_type + " 的路由并热重启进程")}>
                  <Skull size={10}/> 注入毒素代理
                </button>
              </div>
            </div>
          ))}

          {/* Sessions Section */}
          <div className="section-divider mt-4">
            <span>[ 沉睡数据缓冲 ] OFFLINE_CACHE</span>
          </div>

          {sessions.length === 0 && !loading && (
            <div className="empty-text">未发现沉睡的历史数据</div>
          )}

          {sessions.map((s) => (
            <div 
              key={s.filepath} 
              className={`session-item ${selectedSession?.filepath === s.filepath ? 'active' : ''}`}
              onClick={() => loadSession(s)}
            >
              <div className="session-item-title">{s.title || "Unnamed Chat"}</div>
              <div className="session-item-meta">
                <span className="text-accent"><MessageSquare size={10}/> {s.messages_count} logs</span>
                <span>L: {formatDate(s.updated_at)}</span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* 详情主视口 */}
      <div className="session-content cyber-main">
        {selectedSession ? (
          <>
            <div className="session-content-header cyber-header-box">
              <h3 className="glow-text">{selectedSession.title}</h3>
              <div className="session-path-text">
                {selectedSession.filepath}
              </div>
            </div>
            
            <div className="chat-messages cyber-chat-box">
              {messagesLoading ? (
                <div className="empty-chat glitch-text">解析内存残片中... DECRYPTING...</div>
              ) : messages.length === 0 ? (
                <div className="empty-chat text-muted">残片已清空或无法还原</div>
              ) : (
                messages.map((m, idx) => {
                  const isUser = m.role === 'user';
                  const isSys = m.role === 'system';
                  return (
                    <div key={idx} className={`chat-bubble ${isSys ? 'system' : isUser ? 'user' : 'assistant'}`}>
                      {m.role !== 'system' && (
                        <div className="chat-bubble-role">{m.role.toUpperCase()}</div>
                      )}
                      <div className="chat-bubble-text">{m.content}</div>
                    </div>
                  );
                })
              )}
            </div>
          </>
        ) : (
          <div className="empty-chat cyber-placeholder">
            <Activity size={48} className="placeholder-icon pulse-icon" />
            <div>SYSTEM_STANDBY</div>
            <div className="text-muted" style={{fontSize: 12, marginTop: 8}}>请选择左侧内存残片进行逆向还原</div>
          </div>
        )}
      </div>
    </div>
  );
}
