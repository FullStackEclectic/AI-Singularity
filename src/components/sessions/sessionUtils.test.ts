import { describe, expect, it } from "vitest";
import type { ChatMessage, ChatSession } from "./sessionTypes";
import {
  buildResumeCommandWithCwd,
  formatCwdLabel,
  formatUptime,
  getGeminiToolOutputDir,
  getResumeCommand,
  getSessionCwd,
  getSessionFlags,
  isNoTranscriptSession,
  isProblemSession,
  isToolRelatedMessage,
  isWorkspaceHistorySession,
} from "./sessionUtils";

function makeSession(overrides: Partial<ChatSession> = {}): ChatSession {
  return {
    id: "session-1",
    title: "Gemini // demo",
    created_at: 1,
    updated_at: 2,
    messages_count: 3,
    filepath: "C:/Users/test/.gemini/tmp/demo/chats/session-abc.json",
    tool_type: "GeminiCLI",
    cwd: "D:/Code/demo",
    instance_name: "聊天转录",
    source_kind: "transcript",
    has_tool_calls: false,
    has_log_events: false,
    latest_tool_name: null,
    latest_tool_status: null,
    ...overrides,
  };
}

function makeMessage(overrides: Partial<ChatMessage> = {}): ChatMessage {
  return {
    role: "assistant",
    content: "普通内容",
    timestamp: 123,
    full_content: null,
    source_path: null,
    ...overrides,
  };
}

describe("sessionUtils", () => {
  it("prefers explicit cwd and can infer aider cwd from filepath", () => {
    expect(getSessionCwd(makeSession({ cwd: "D:/Code/demo" }))).toBe("D:/Code/demo");

    expect(
      getSessionCwd(
        makeSession({
          id: "aider-22",
          title: "Aider // demo",
          cwd: undefined,
          filepath: "D:/Code/demo/.aider.chat.history.md",
        })
      )
    ).toBe("D:/Code/demo");
  });

  it("resolves resume commands by tool type", () => {
    expect(getResumeCommand(makeSession({ tool_type: "GeminiCLI" }))).toBe("gemini");
    expect(getResumeCommand(makeSession({ tool_type: "Codex", title: "Codex // demo" }))).toBe("codex");
    expect(getResumeCommand(makeSession({ tool_type: "ClaudeCode", title: "Claude // demo" }))).toBe("claude");
    expect(getResumeCommand(makeSession({ tool_type: "Aider", title: "Aider // demo" }))).toBe("aider");
  });

  it("builds resume command with cwd", () => {
    expect(buildResumeCommandWithCwd(makeSession({ tool_type: "GeminiCLI", cwd: "D:/Code/demo" }))).toBe(
      'cd /d "D:/Code/demo" && gemini'
    );
  });

  it("extracts Gemini tool output directory from transcript path", () => {
    expect(
      getGeminiToolOutputDir(
        makeSession({
          filepath: "C:/Users/test/.gemini/tmp/demo/chats/session-2026-04-16T01-23-abc123.json",
        })
      )
    ).toBe("C:/Users/test/.gemini/tmp/demo/tool-outputs/session-2026-04-16T01-23-abc123");

    expect(getGeminiToolOutputDir(makeSession({ tool_type: "Codex" }))).toBeNull();
  });

  it("marks tool-related messages", () => {
    expect(isToolRelatedMessage(makeMessage({ role: "tool" }))).toBe(true);
    expect(isToolRelatedMessage(makeMessage({ content: "[工具调用]\nread_file [success]" }))).toBe(true);
    expect(isToolRelatedMessage(makeMessage({ content: "普通内容" }))).toBe(false);
  });

  it("builds session flags including failed latest tool status", () => {
    const flags = getSessionFlags(
      makeSession({
        source_kind: "workspace_history",
        has_tool_calls: true,
        has_log_events: true,
        latest_tool_name: "read_file",
        latest_tool_status: "error",
        messages_count: 0,
      })
    );

    expect(flags.map((item) => item.label)).toEqual([
      "工作区历史",
      "含工具调用",
      "含日志事件",
      "最近工具 read_file · error",
      "无消息转录",
    ]);
    expect(flags.find((item) => item.label.includes("最近工具"))?.tone).toBe("danger");
  });

  it("detects workspace history and problem sessions", () => {
    const historySession = makeSession({ source_kind: "workspace_history", messages_count: 0 });
    expect(isWorkspaceHistorySession(historySession)).toBe(true);
    expect(isNoTranscriptSession(historySession)).toBe(true);
    expect(isProblemSession(historySession)).toBe(true);

    const transcriptSession = makeSession({ source_kind: "transcript", messages_count: 5 });
    expect(isWorkspaceHistorySession(transcriptSession)).toBe(false);
    expect(isNoTranscriptSession(transcriptSession)).toBe(false);
    expect(isProblemSession(transcriptSession)).toBe(false);
  });

  it("formats uptime and cwd labels", () => {
    expect(formatUptime(45)).toBe("45s");
    expect(formatUptime(120)).toBe("2m");
    expect(formatUptime(7200)).toBe("2.0h");

    expect(formatCwdLabel("")).toBe("未识别工作目录");
    expect(formatCwdLabel("D:/Code/demo")).toBe("D:/Code/demo");
    expect(formatCwdLabel("D:/Very/Long/Path/For/Testing/That/Should/Be/Trimmed/At/Some/Point")).toMatch(/^\.{3}/);
  });
});
