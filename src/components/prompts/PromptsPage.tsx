import { useEffect, useState } from "react";
import { usePromptStore, type PromptConfig } from "../../stores/promptStore";
import "./PromptsPage.css";

export default function PromptsPage() {
  const { prompts, isLoading, fetch, deletePrompt, syncPrompt } = usePromptStore();
  const [showAdd, setShowAdd] = useState(false);
  const [editingPrompt, setEditingPrompt] = useState<PromptConfig | null>(null);
  const [syncingId, setSyncingId] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const [syncDialog, setSyncDialog] = useState<{ prompt: PromptConfig; dir: string } | null>(null);
  const [confirmDeletePrompt, setConfirmDeletePrompt] = useState<PromptConfig | null>(null);

  useEffect(() => {
    fetch();
  }, [fetch]);

  const executeSync = async (prompt: PromptConfig, dir: string) => {
    setSyncingId(prompt.id);
    try {
      await syncPrompt(prompt.id, dir);
      setMessage(`成功同步到 ${dir}\\${prompt.target_file}`);
    } catch (e) {
      setMessage(`同步失败: ${e}`);
    } finally {
      setSyncingId(null);
    }
  };

  return (
    <div className="prompts-page">
      <div className="page-header">
        <div>
          <h1 className="page-title">System Prompts</h1>
          <p className="page-subtitle">
            统一维护你的 AI Prompt / 设定档，并一键推送到开发工程目录
          </p>
        </div>
        <div style={{ display: "flex", gap: "var(--space-3)" }}>
          <button className="btn btn-ghost" onClick={() => fetch()} disabled={isLoading}>
            ⟳ 刷新
          </button>
          <button className="btn btn-primary" onClick={() => {
            setEditingPrompt(null);
            setShowAdd(true);
          }}>
            ＋ 新建 Prompt
          </button>
        </div>
      </div>

      <div className="prompts-body">
        {message && (
          <div className="alert alert-info" style={{ marginBottom: "var(--space-4)" }}>
            {message}
          </div>
        )}
        {isLoading && prompts.length === 0 ? (
          <div className="empty-state">
             <div className="animate-spin" style={{ fontSize: 24 }}>⟳</div>
             <span>加载中...</span>
          </div>
        ) : prompts.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">📝</div>
            <h3 style={{ color: "var(--color-text-secondary)" }}>暂无系统提示词</h3>
            <p>管理全局规范、代码风格，一键下发到各自项目空间中</p>
            <button className="btn btn-primary" onClick={() => setShowAdd(true)}>
              ＋ 创建第一个 Prompt
            </button>
          </div>
        ) : (
          <div className="prompts-grid">
            {prompts.map((p) => (
              <div key={p.id} className="prompt-card card animate-fade-in">
                <div className="prompt-header">
                   <div>
                      <div className="prompt-name">{p.name}</div>
                      <div className="prompt-file font-mono text-muted">{p.target_file}</div>
                   </div>
                   <div className="prompt-actions">
                      <button 
                         className="btn btn-ghost btn-sm"
                         onClick={() => {
                           setEditingPrompt(p);
                           setShowAdd(true);
                         }}
                      >编辑</button>
                      <button 
                         className="btn btn-primary btn-sm"
                         onClick={() => setSyncDialog({ prompt: p, dir: "" })}
                         disabled={syncingId === p.id}
                      >
                        {syncingId === p.id ? "同步中..." : "推送至项目"}
                      </button>
                      <button 
                         className="btn btn-danger btn-sm btn-icon"
                         onClick={() => setConfirmDeletePrompt(p)}
                      >✕</button>
                   </div>
                </div>
                <div className="prompt-preview">
                   {p.content.slice(0, 150)}{p.content.length > 150 ? "..." : ""}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {showAdd && (
        <PromptModal
          existing={editingPrompt}
          onClose={() => {
            setShowAdd(false);
            setEditingPrompt(null);
          }}
          onSuccess={() => {
            setShowAdd(false);
            setEditingPrompt(null);
          }}
        />
      )}

      {syncDialog && (
        <div className="modal-overlay" onClick={() => setSyncDialog(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>推送至项目</h2>
              <button className="btn btn-icon" onClick={() => setSyncDialog(null)}>✕</button>
            </div>
            <div className="modal-body">
              <div className="form-row">
                <label className="form-label">目标工程目录</label>
                <input
                  className="form-input font-mono"
                  placeholder="例如：C:\\Code\\my-project"
                  value={syncDialog.dir}
                  onChange={(e) => setSyncDialog({ ...syncDialog, dir: e.target.value })}
                />
              </div>
              <div className="modal-footer">
                <button type="button" className="btn btn-ghost" onClick={() => setSyncDialog(null)}>取消</button>
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={!syncDialog.dir.trim() || syncingId === syncDialog.prompt.id}
                  onClick={async () => {
                    const current = syncDialog;
                    setSyncDialog(null);
                    await executeSync(current.prompt, current.dir.trim());
                  }}
                >
                  {syncingId === syncDialog.prompt.id ? "同步中..." : "开始推送"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {confirmDeletePrompt && (
        <div className="modal-overlay" onClick={() => setConfirmDeletePrompt(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>删除 Prompt</h2>
              <button className="btn btn-icon" onClick={() => setConfirmDeletePrompt(null)}>✕</button>
            </div>
            <div className="modal-body">
              <p>确认删除 Prompt “{confirmDeletePrompt.name}” 吗？</p>
              <div className="modal-footer">
                <button type="button" className="btn btn-ghost" onClick={() => setConfirmDeletePrompt(null)}>取消</button>
                <button
                  type="button"
                  className="btn btn-danger"
                  onClick={async () => {
                    await deletePrompt(confirmDeletePrompt.id);
                    setMessage(`已删除 Prompt：${confirmDeletePrompt.name}`);
                    setConfirmDeletePrompt(null);
                  }}
                >
                  删除
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function PromptModal({ existing, onClose, onSuccess }: { existing: PromptConfig | null, onClose: () => void, onSuccess: () => void }) {
  const { save } = usePromptStore();
  const [form, setForm] = useState({
    name: existing?.name || "",
    target_file: existing?.target_file || "CLAUDE.md",
    content: existing?.content || "",
    is_active: existing ? existing.is_active : true
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!form.name.trim() || !form.target_file.trim() || !form.content.trim()) {
      setError("所有字段都是必填项");
      return;
    }
    
    setIsSubmitting(true);
    setError("");
    try {
      await save({
        id: existing?.id || "",
        name: form.name.trim(),
        target_file: form.target_file.trim(),
        content: form.content.trim(),
        is_active: form.is_active,
        created_at: existing?.created_at || "",
        updated_at: ""
      });
      onSuccess();
    } catch (err) {
      setError(String(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-lg" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>{existing ? "编辑" : "新建"} Prompt</h2>
          <button className="btn btn-icon" onClick={onClose}>✕</button>
        </div>

        <form className="modal-body" onSubmit={handleSubmit}>
          <div className="form-row flex-row-2">
            <div>
              <label className="form-label">标识名称 *</label>
              <input
                className="form-input"
                placeholder="例如：前端 React 规范"
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
              />
            </div>
            <div>
              <label className="form-label">生成文件名 *</label>
              <input
                className="form-input font-mono"
                placeholder="例如：CLAUDE.md"
                value={form.target_file}
                onChange={(e) => setForm({ ...form, target_file: e.target.value })}
              />
            </div>
          </div>

          <div className="form-row" style={{ flex: 1 }}>
            <label className="form-label">内容 (Markdown) *</label>
            <textarea
              className="form-input font-mono prompt-textarea"
              placeholder="在这里编写针对 AI 的架构及风格指导规范..."
              value={form.content}
              onChange={(e) => setForm({ ...form, content: e.target.value })}
            />
          </div>

          {error && <div className="form-error">{error}</div>}

          <div className="modal-footer">
            <button type="button" className="btn btn-ghost" onClick={onClose}>取消</button>
            <button
              type="submit"
              className="btn btn-primary"
              disabled={isSubmitting}
            >
              {isSubmitting ? "保存中..." : "保存记录"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
