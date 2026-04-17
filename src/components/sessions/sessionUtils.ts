import type { ChatMessage, ChatSession } from "./sessionTypes";

export function formatDate(ts: number) {
  if (ts === 0) return "Unknown";
  const d = new Date(ts * 1000);
  return `${d.getMonth() + 1}/${d.getDate()} ${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}`;
}

export function formatMessageTime(ts?: number) {
  if (!ts) return "";
  const d = new Date(ts * 1000);
  return d.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function formatUptime(secs: number) {
  if (secs < 60) return `${secs}s`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m`;
  return `${(mins / 60).toFixed(1)}h`;
}

export function getSessionCwd(session: ChatSession) {
  if (session.cwd) return session.cwd;
  if (session.id.startsWith("aider-") || session.title.includes("Aider")) {
    const sep = session.filepath.includes("\\") ? "\\" : "/";
    return session.filepath.substring(0, session.filepath.lastIndexOf(sep));
  }
  return "";
}

export function isToolRelatedMessage(message: ChatMessage) {
  return (
    message.role === "tool" ||
    message.content.includes("[工具调用]") ||
    message.content.includes("工具调用：") ||
    message.content.includes("Gemini logs.json 已记录") ||
    message.content.includes("当前会话的工具输出目录")
  );
}

export function getGeminiToolOutputDir(session: ChatSession | null) {
  if (!session || session.tool_type !== "GeminiCLI") return null;
  if (!session.filepath.toLowerCase().endsWith(".json")) return null;
  const normalized = session.filepath.replace(/\\/g, "/");
  const chatsMarker = "/chats/";
  const chatsIndex = normalized.lastIndexOf(chatsMarker);
  if (chatsIndex === -1) return null;
  const workspaceDir = normalized.slice(0, chatsIndex);
  const fileName = normalized.slice(normalized.lastIndexOf("/") + 1);
  const sessionIdMatch = fileName.match(/session-(.+)\.json$/i);
  if (!sessionIdMatch?.[1]) return null;
  return `${workspaceDir}/tool-outputs/session-${sessionIdMatch[1]}`;
}

export function getResumeCommand(session: ChatSession) {
  if (session.tool_type === "Aider" || session.id.startsWith("aider-") || session.title.includes("Aider")) return "aider";
  if (session.tool_type === "Codex" || session.title.includes("Codex")) return "codex";
  if (session.tool_type === "ClaudeCode" || session.title.includes("Claude")) return "claude";
  if (session.tool_type === "GeminiCLI" || session.title.includes("Gemini")) return "gemini";
  return "aider";
}

export function buildResumeCommandWithCwd(session: ChatSession) {
  const cwd = getSessionCwd(session);
  const cmd = getResumeCommand(session);
  return cwd ? `cd /d "${cwd}" && ${cmd}` : cmd;
}

export function getSessionFlags(session: ChatSession) {
  const flags: { label: string; tone?: "warning" | "info" | "danger" }[] = [];
  if (session.source_kind === "workspace_history") {
    flags.push({ label: "工作区历史", tone: "info" });
  }
  if (session.has_tool_calls) {
    flags.push({ label: "含工具调用", tone: "info" });
  }
  if (session.has_log_events) {
    flags.push({ label: "含日志事件", tone: "info" });
  }
  if (session.latest_tool_name && session.latest_tool_status) {
    flags.push({
      label: `最近工具 ${session.latest_tool_name} · ${session.latest_tool_status}`,
      tone: session.latest_tool_status === "success" ? "info" : "danger",
    });
  }
  if (session.tool_type === "GeminiCLI" && session.messages_count === 0) {
    flags.push({ label: "无消息转录", tone: "warning" });
  }
  return flags;
}

export function isWorkspaceHistorySession(session: ChatSession) {
  return session.source_kind === "workspace_history";
}

export function isNoTranscriptSession(session: ChatSession) {
  return session.messages_count === 0;
}

export function isProblemSession(session: ChatSession) {
  return isWorkspaceHistorySession(session) || isNoTranscriptSession(session);
}

export function formatCwdLabel(cwd: string) {
  if (!cwd) return "未识别工作目录";
  const normalized = cwd.replace(/\\/g, "/");
  return normalized.length > 46 ? `...${normalized.slice(-46)}` : normalized;
}
