import type { Platform, ToolTarget } from "../../types";

export const ALL_TOOLS: ToolTarget[] = [
  "claude_code",
  "codex",
  "gemini_cli",
  "open_code",
  "open_claw",
  "aider",
];

export const TOOL_ICONS: Record<ToolTarget, string> = {
  claude_code: "🤖",
  codex: "🧠",
  gemini_cli: "✨",
  open_code: "🌐",
  open_claw: "🦞",
  aider: "💻",
};

export const getProviderPlatformIcon = (platform: Platform) => {
  if (platform === "anthropic") return "🟠";
  if (platform === "open_ai") return "🟢";
  if (platform === "gemini") return "🔵";
  if (platform === "deep_seek") return "🔷";
  if (platform === "open_router") return "🟣";
  return "⚙️";
};
