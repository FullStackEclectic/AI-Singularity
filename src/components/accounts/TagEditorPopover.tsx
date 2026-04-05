import { useState, useRef, useEffect, KeyboardEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useQueryClient } from "@tanstack/react-query";
import "./TagEditorPopover.css";

interface TagEditorPopoverProps {
  accountId: string;
  accountType: "ide" | "api";
  currentTags: string[];
  anchorEl: HTMLElement | null;
  onClose: () => void;
}

// 预设标签颜色（按 tag 哈希取色）
const TAG_COLORS = [
  "#6366f1", "#8b5cf6", "#ec4899", "#f43f5e",
  "#f97316", "#eab308", "#22c55e", "#14b8a6",
  "#3b82f6", "#06b6d4",
];

function getTagColor(tag: string): string {
  let hash = 0;
  for (let i = 0; i < tag.length; i++) {
    hash = tag.charCodeAt(i) + ((hash << 5) - hash);
  }
  return TAG_COLORS[Math.abs(hash) % TAG_COLORS.length];
}

export default function TagEditorPopover({
  accountId,
  accountType,
  currentTags,
  anchorEl,
  onClose,
}: TagEditorPopoverProps) {
  const qc = useQueryClient();
  const [tags, setTags] = useState<string[]>(currentTags);
  const [input, setInput] = useState("");
  const [saving, setSaving] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);

  // 点击外部关闭
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node) &&
        anchorEl &&
        !anchorEl.contains(e.target as Node)
      ) {
        handleSave();
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [tags]);

  // 聚焦输入框
  useEffect(() => {
    setTimeout(() => inputRef.current?.focus(), 50);
  }, []);

  const addTag = (tag: string) => {
    const trimmed = tag.trim();
    if (!trimmed || tags.includes(trimmed)) return;
    setTags([...tags, trimmed]);
    setInput("");
  };

  const removeTag = (tag: string) => {
    setTags(tags.filter((t) => t !== tag));
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" || e.key === "," || e.key === "Tab") {
      e.preventDefault();
      addTag(input);
    } else if (e.key === "Backspace" && !input && tags.length > 0) {
      removeTag(tags[tags.length - 1]);
    } else if (e.key === "Escape") {
      handleSave();
    }
  };

  const handleSave = async () => {
    if (saving) return;
    setSaving(true);
    try {
      const cmd = accountType === "ide" ? "update_ide_account_tags" : "update_api_key_tags";
      await invoke(cmd, { id: accountId, tags });
      qc.invalidateQueries({ queryKey: ["ideAccounts"] });
      qc.invalidateQueries({ queryKey: ["keys"] });
    } catch (e) {
      console.error("[TagEditor] 保存标签失败", e);
    } finally {
      setSaving(false);
      onClose();
    }
  };

  if (!anchorEl) return null;

  return (
    <div className="tag-editor-popover" ref={popoverRef}>
      <div className="tag-editor-header">
        <span>编辑标签</span>
        <button className="tag-editor-close" onClick={handleSave}>✕</button>
      </div>

      <div className="tag-editor-tags">
        {tags.map((tag) => (
          <span
            key={tag}
            className="tag-chip"
            style={{ backgroundColor: getTagColor(tag) + "33", borderColor: getTagColor(tag) + "88", color: getTagColor(tag) }}
          >
            {tag}
            <button className="tag-remove-btn" onClick={() => removeTag(tag)}>×</button>
          </span>
        ))}
        <input
          ref={inputRef}
          className="tag-input"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={tags.length === 0 ? "输入标签，回车确认..." : "继续添加..."}
        />
      </div>

      <div className="tag-editor-hint">
        回车 / 逗号 添加 · Backspace 删除 · Esc 保存关闭
      </div>

      <div className="tag-editor-footer">
        <button className="tag-editor-save" onClick={handleSave} disabled={saving}>
          {saving ? "保存中..." : "保存"}
        </button>
      </div>
    </div>
  );
}

/** 内联标签展示组件（点击打开编辑器） */
export function TagBadgeList({
  tags,
  accountId,
  accountType,
  editable = true,
}: {
  tags?: string[];
  accountId: string;
  accountType: "ide" | "api";
  editable?: boolean;
}) {
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);

  const handleClick = (e: React.MouseEvent<HTMLElement>) => {
    if (!editable) return;
    e.stopPropagation();
    setAnchorEl(anchorEl ? null : e.currentTarget);
  };

  return (
    <div className="tag-badge-container" onClick={handleClick}>
      {(tags ?? []).map((tag) => (
        <span
          key={tag}
          className="tag-badge"
          style={{ backgroundColor: getTagColor(tag) + "22", color: getTagColor(tag) }}
        >
          {tag}
        </span>
      ))}
      {editable && (
        <span className="tag-add-btn">
          {(tags?.length ?? 0) === 0 ? "+ 标签" : "+"}
        </span>
      )}

      {anchorEl && (
        <TagEditorPopover
          accountId={accountId}
          accountType={accountType}
          currentTags={tags ?? []}
          anchorEl={anchorEl}
          onClose={() => setAnchorEl(null)}
        />
      )}
    </div>
  );
}
