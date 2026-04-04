import { useState, useEffect } from "react";
import { api } from "../../lib/api";
import { listen } from "@tauri-apps/api/event";
import "./ToolDepotPage.css";

interface ToolStatus {
  id: string;
  is_installed: boolean;
  version: string | null;
}

const WEAPONS = [
  {
    id: "claude_code",
    name: "Claude Code",
    desc: "Anthropic 官方控制台智能体 (NPM版)",
    requirements: "Node.js 环境"
  },
  {
    id: "aider",
    name: "Aider AI",
    desc: "全能型本地终端 AI 结对编程助手",
    requirements: "Python 3, pip"
  }
];

export default function ToolDepotPage() {
  const [statuses, setStatuses] = useState<Record<string, ToolStatus>>({});
  const [loadingIds, setLoadingIds] = useState<Set<string>>(new Set());
  const [logs, setLogs] = useState<string[]>([]);
  const [logOpen, setLogOpen] = useState(false);

  useEffect(() => {
    refreshAll();

    const unlisten = listen<string>("provisioner-event", (event) => {
      setLogs((prev) => [...prev, event.payload]);
    });

    return () => {
      unlisten.then(f => f());
    };
  }, []);

  const refreshAll = async () => {
    for (const w of WEAPONS) {
      try {
        const payload = await api.tools.checkStatus(w.id);
        setStatuses(prev => ({ ...prev, [w.id]: payload }));
      } catch (e) {
        console.error("Status error for", w.id, e);
      }
    }
  };

  const handleDeploy = async (id: string) => {
    try {
      setLoadingIds(prev => new Set(prev).add(id));
      setLogOpen(true);
      setLogs(prev => [...prev, `[SYSTEM] 启动 ${id} 空投装载程序...`]);
      await api.tools.deploy(id);
      await refreshAll();
    } catch (e: any) {
      setLogs(prev => [...prev, `[ERROR] ${e.toString()}`]);
    } finally {
      setLoadingIds(prev => {
        const n = new Set(prev);
        n.delete(id);
        return n;
      });
    }
  };

  return (
    <div className="page-container page-tool-depot">
      <div className="page-header">
         <div className="page-title-row">
           <h1>🛠️ 兵工厂装配车间 (Tool Depot)</h1>
           <p className="page-subtitle">独立且纯净的环境隔层。向您的系统静默部署核武级编程工具。</p>
         </div>
      </div>

      <div className="depot-grid">
        {WEAPONS.map(w => {
           const st = statuses[w.id];
           const isDeploying = loadingIds.has(w.id);
           return (
             <div key={w.id} className="depot-card">
               <div className="depot-card-header">
                 <h3>{w.name}</h3>
                 {st?.is_installed ? (
                    <span className="badge badge-success">ACTIVE</span>
                 ) : (
                    <span className="badge badge-muted">未就绪</span>
                 )}
               </div>
               <p className="depot-desc">{w.desc}</p>
               <div className="depot-req">依赖: {w.requirements}</div>
               
               <div className="depot-actions">
                 {st?.is_installed ? (
                   <div className="installed-info">
                     <span className="ver">v{st.version}</span>
                     <button className="btn btn-ghost" onClick={() => handleDeploy(w.id)} disabled={isDeploying}>
                       {isDeploying ? "重装中..." : "🔄 重新装配"}
                     </button>
                   </div>
                 ) : (
                   <button 
                     className="btn btn-primary" 
                     onClick={() => handleDeploy(w.id)}
                     disabled={isDeploying}
                     style={{ width: "100%" }}
                   >
                     {isDeploying ? "正在空投..." : "⏬ 请求部署"}
                   </button>
                 )}
               </div>
             </div>
           );
        })}
      </div>

      {logOpen && (
        <div className="depot-logs-panel">
          <div className="logs-header">
            <span>💻 自动化装载日志监控</span>
            <button className="btn-icon" onClick={() => setLogOpen(false)}>✕</button>
          </div>
          <div className="logs-body">
            {logs.map((L, i) => (
              <div key={i} className="log-line">{L}</div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
