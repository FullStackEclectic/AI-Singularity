import { useEffect, useState } from "react";
import { useSkillStore, type SkillInfo } from "../../stores/skillStore";
import "./SkillsPage.css";

export default function SkillsPage() {
  const { skills, isLoading, error, fetch, install, update, uninstall } = useSkillStore();
  const [showAdd, setShowAdd] = useState(false);
  const [installUrl, setInstallUrl] = useState("");
  const [isInstalling, setIsInstalling] = useState(false);
  const [installError, setInstallError] = useState("");

  useEffect(() => {
    fetch();
  }, [fetch]);

  const handleInstall = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!installUrl.trim()) return;
    
    setIsInstalling(true);
    setInstallError("");
    try {
      await install(installUrl.trim());
      setShowAdd(false);
      setInstallUrl("");
    } catch (err: any) {
      setInstallError(String(err));
    } finally {
      setIsInstalling(false);
    }
  };

  const handleUpdate = async (id: string) => {
    try {
      await update(id);
      alert("更新成功");
    } catch (err: any) {
      alert("更新失败: " + String(err));
    }
  };

  const handleUninstall = async (id: string) => {
    if (!confirm(`确定卸载技能库 ${id} 吗？将彻底删除本地文件。`)) return;
    try {
      await uninstall(id);
    } catch (err: any) {
      alert("卸载失败: " + String(err));
    }
  };

  return (
    <div className="skills-page animate-fade-in">
      <div className="page-header">
        <div>
          <h1 className="page-title">Skills 技能插件</h1>
          <p className="page-subtitle">
            专门管理 Claude Code 的扩展命令行逻辑，支持通过 Git 仓库地址一键下发和本地拉取隔离能力。
          </p>
        </div>
        <div style={{ display: "flex", gap: "var(--space-3)" }}>
          <button className="btn btn-ghost" onClick={() => fetch()} disabled={isLoading}>
            ⟳ 刷新
          </button>
          <button className="btn btn-primary" onClick={() => setShowAdd(true)}>
            ＋ 安装 Github 插件
          </button>
        </div>
      </div>

      {error && <div className="form-error" style={{ marginBottom: 16 }}>⚠ 获取技能列表失败：{error}</div>}

      <div className="skills-body">
        {isLoading && skills.length === 0 ? (
           <div className="empty-state">
             <div className="animate-spin" style={{ fontSize: 24 }}>⟳</div>
             <span>扫描本地环境...</span>
           </div>
        ) : skills.length === 0 ? (
           <div className="empty-state">
             <div className="empty-state-icon">🧩</div>
             <h3 style={{ color: "var(--color-text-secondary)" }}>您还没有安装任何 Skill</h3>
             <p>通过提供 Github 仓库 URL 自动在 Claude Code 目录下生成挂载镜像。</p>
             <button className="btn btn-primary" onClick={() => setShowAdd(true)}>
               ＋ 输入 Github Repository URL
             </button>
           </div>
        ) : (
           <div className="skills-list">
             {skills.map((s) => (
               <SkillCard 
                 key={s.id} 
                 skill={s} 
                 onUpdate={() => handleUpdate(s.id)}
                 onUninstall={() => handleUninstall(s.id)}
               />
             ))}
           </div>
        )}
      </div>

      {/* Add Modal */}
      {showAdd && (
        <div className="modal-overlay" onClick={() => !isInstalling && setShowAdd(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>从 Git 安装扩展技能</h2>
              <button className="btn btn-icon" onClick={() => setShowAdd(false)} disabled={isInstalling}>✕</button>
            </div>
            <form className="modal-body" onSubmit={handleInstall}>
              <p className="text-muted" style={{ fontSize: 13, marginBottom: 16 }}>
                程序将在后台将代码库安全克隆至 <code>~/.claude/commands/</code> 以供跨端工具执行。
              </p>
              
              <div className="form-row">
                <label className="form-label">Git Repository URL</label>
                <input 
                   className="form-input font-mono"
                   placeholder="https://github.com/user/my-claude-skill.git"
                   value={installUrl}
                   onChange={e => setInstallUrl(e.target.value)}
                   disabled={isInstalling}
                   autoFocus
                />
              </div>

              {installError && <div className="form-error">{installError}</div>}
              {isInstalling && (
                <div className="alert alert-info">
                  <div className="animate-spin" style={{ display: "inline-block", marginRight: 8 }}>⟳</div>
                  正在拉取代码和运行脚本，由于网络差异，可能需要几秒到一分钟...
                </div>
              )}

              <div className="modal-footer">
                <button type="button" className="btn btn-ghost" onClick={() => setShowAdd(false)} disabled={isInstalling}>取消</button>
                <button type="submit" className="btn btn-primary" disabled={isInstalling || !installUrl.trim()}>
                  {isInstalling ? "安装中..." : "立刻导入"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}

function SkillCard({ skill, onUpdate, onUninstall }: { skill: SkillInfo; onUpdate: () => void; onUninstall: () => void }) {
  return (
    <div className="card skill-card">
      <div className="skill-icon">🛠️</div>
      <div className="skill-info">
        <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
          <h3 style={{ margin: 0, fontSize: 16 }}>{skill.name}</h3>
          <span className="badge badge-success">Local</span>
        </div>
        {skill.source_url && (
          <div className="text-muted font-mono" style={{ fontSize: 12, marginTop: 4 }}>
            Remotes: {skill.source_url}
          </div>
        )}
        <div className="text-muted font-mono" style={{ fontSize: 11, marginTop: 2 }}>
          路径: {skill.local_path}
        </div>
      </div>
      <div className="skill-actions">
         <button className="btn btn-ghost btn-xs" onClick={onUpdate} title="执行 git pull">🔄 更新 pull</button>
         <button className="btn btn-danger-ghost btn-xs" onClick={onUninstall}>删除</button>
      </div>
    </div>
  );
}
