import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { MessageSquare, RefreshCw, Cpu, Activity, Skull, Terminal, Copy, Folder, ChevronRight, ChevronDown } from "lucide-react";
import "./SessionsPage.css";

interface ChatSession {
  id: string;
  title: string;
  created_at: number;
  updated_at: number;
  messages_count: number;
  filepath: string;
  tool_type?: string;
  cwd?: string;
  instance_name?: string;
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

interface SessionGroup {
  cwd: string;
  label: string;
  updated_at: number;
  sessions: ChatSession[];
}

interface CodexInstanceRecord {
  id: string;
  name: string;
  user_data_dir: string;
  extra_args?: string;
  bind_account_id?: string | null;
  bind_provider_id?: string | null;
  last_pid?: number | null;
  last_launched_at?: string | null;
  has_state_db: boolean;
  has_session_index: boolean;
  running?: boolean;
  is_default?: boolean;
  follow_local_account?: boolean;
}

interface ProviderOption {
  id: string;
  name: string;
  tool_targets?: string | null;
  is_active?: boolean;
}

type ActionMessage = { text: string; tone?: "error" | "success" | "info" };

type ConfirmDialogState = {
  title: string;
  description: string;
  confirmLabel: string;
  tone?: "danger" | "primary";
  action: () => Promise<void> | void;
};

type CodexSettingsDialogState = {
  instance: CodexInstanceRecord;
  extraArgs: string;
  bindAccountId: string;
  bindProviderId: string;
  followLocalAccount: boolean;
};

export default function SessionsPage() {
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [zombies, setZombies] = useState<ZombieProcess[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedSession, setSelectedSession] = useState<ChatSession | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [messagesLoading, setMessagesLoading] = useState(false);
  const [expandedGroups, setExpandedGroups] = useState<string[]>([]);
  const [selectedFilepaths, setSelectedFilepaths] = useState<string[]>([]);
  const [codexInstances, setCodexInstances] = useState<CodexInstanceRecord[]>([]);
  const [defaultCodexInstance, setDefaultCodexInstance] = useState<CodexInstanceRecord | null>(null);
  const [codexProviders, setCodexProviders] = useState<ProviderOption[]>([]);
  const [showCodexInstances, setShowCodexInstances] = useState(false);
  const [codexInstanceName, setCodexInstanceName] = useState("");
  const [codexInstanceDir, setCodexInstanceDir] = useState("");
  const [codexInstanceLoading, setCodexInstanceLoading] = useState(false);
  const [actionMessages, setActionMessages] = useState<(ActionMessage & { id: string; createdAt: number })[]>([]);
  const [confirmDialog, setConfirmDialog] = useState<ConfirmDialogState | null>(null);
  const [confirmDialogBusy, setConfirmDialogBusy] = useState(false);
  const [codexSettingsDialog, setCodexSettingsDialog] = useState<CodexSettingsDialogState | null>(null);
  const codexInstanceCount = codexInstances.length + 1;

  const pushActionMessage = (message: ActionMessage) => {
    setActionMessages((prev) => [
      {
        ...message,
        id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
        createdAt: Date.now(),
      },
      ...prev,
    ].slice(0, 6));
  };

  const clearActionMessages = () => setActionMessages([]);

  const fetchAll = async () => {
    setLoading(true);
    try {
      const [sessData, zombieData, codexData, defaultCodex] = await Promise.all([
        invoke<ChatSession[]>("list_sessions"),
        invoke<ZombieProcess[]>("scan_zombies"),
        invoke<CodexInstanceRecord[]>("list_codex_instances"),
        invoke<CodexInstanceRecord>("get_default_codex_instance"),
      ]);
      const providerData = await invoke<ProviderOption[]>("get_providers");
      setSessions(sessData);
      setSelectedFilepaths((prev) => prev.filter((filepath) => sessData.some((item) => item.filepath === filepath)));
      setZombies(zombieData);
      setCodexInstances(codexData);
      setDefaultCodexInstance(defaultCodex);
      setCodexProviders(providerData.filter((item) => {
        try {
          const targets = item.tool_targets ? JSON.parse(item.tool_targets) as string[] : ["claude_code"];
          return targets.includes("codex");
        } catch {
          return false;
        }
      }));
    } catch (e) {
      console.error("Failed to load sessions/zombies:", e);
      pushActionMessage({ text: "加载会话数据失败：" + String(e), tone: "error" });
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

  const getSessionCwd = (session: ChatSession) => {
    if (session.cwd) return session.cwd;
    if (session.id.startsWith("aider-") || session.title.includes("Aider")) {
       const sep = session.filepath.includes('\\') ? '\\' : '/';
       return session.filepath.substring(0, session.filepath.lastIndexOf(sep));
    }
    return "";
  };

  const getResumeCommand = (session: ChatSession) => {
    if (session.tool_type === "Aider" || session.id.startsWith("aider-") || session.title.includes("Aider")) return "aider";
    if (session.tool_type === "Codex" || session.title.includes("Codex")) return "codex";
    if (session.tool_type === "ClaudeCode" || session.title.includes("Claude")) return "claude";
    if (session.tool_type === "GeminiCLI" || session.title.includes("Gemini")) return "gemini";
    return "aider"; // fallback
  };

  const handleCopyCmd = (session: ChatSession) => {
    const cwd = getSessionCwd(session);
    const cmd = getResumeCommand(session);
    const fullCmd = cwd ? `cd /d "${cwd}" && ${cmd}` : cmd;
    navigator.clipboard.writeText(fullCmd).then(() => pushActionMessage({ text: "恢复指令已复制到剪贴板", tone: "success" }));
  };

  const handleCopyDir = (session: ChatSession) => {
    const cwd = getSessionCwd(session);
    if (cwd) {
       navigator.clipboard.writeText(cwd).then(() => pushActionMessage({ text: "目录路径已复制", tone: "success" }));
    } else {
       pushActionMessage({ text: "当前会话没有可推断的工作目录，请改用复制恢复命令。", tone: "info" });
    }
  };

  const handleLaunchTerminal = async (session: ChatSession) => {
    const cwd = getSessionCwd(session);
    const cmd = getResumeCommand(session);
    
    try {
      await invoke("launch_session_terminal", { cwd: cwd || ".", command: cmd });
      pushActionMessage({ text: "已尝试在外部终端启动会话", tone: "success" });
    } catch (e) {
      pushActionMessage({ text: "外置终端下发执行失败：" + String(e), tone: "error" });
    }
  };

  const sessionGroups = useMemo<SessionGroup[]>(() => {
    const grouped = new Map<string, ChatSession[]>();

    for (const session of sessions) {
      const cwd = getSessionCwd(session) || `[${session.tool_type || "Unknown"}]`;
      const bucket = grouped.get(cwd) ?? [];
      bucket.push(session);
      grouped.set(cwd, bucket);
    }

    return Array.from(grouped.entries())
      .map(([cwd, groupedSessions]) => {
        const normalized = cwd.replace(/\\/g, "/").replace(/\/$/, "");
        const parts = normalized.split("/").filter(Boolean);
        const label = cwd.startsWith("[") ? cwd : (parts[parts.length - 1] || cwd);
        const sortedSessions = [...groupedSessions].sort((a, b) => b.updated_at - a.updated_at);
        return {
          cwd,
          label,
          updated_at: sortedSessions[0]?.updated_at ?? 0,
          sessions: sortedSessions,
        };
      })
      .sort((a, b) => b.updated_at - a.updated_at || a.label.localeCompare(b.label, "zh-CN"));
  }, [sessions]);

  const toggleGroupExpanded = (cwd: string) => {
    setExpandedGroups((prev) =>
      prev.includes(cwd) ? prev.filter((item) => item !== cwd) : [...prev, cwd]
    );
  };

  const toggleSessionSelected = (filepath: string) => {
    setSelectedFilepaths((prev) =>
      prev.includes(filepath) ? prev.filter((item) => item !== filepath) : [...prev, filepath]
    );
  };

  const toggleAllSessionsSelected = () => {
    if (selectedFilepaths.length === sessions.length) {
      setSelectedFilepaths([]);
    } else {
      setSelectedFilepaths(sessions.map((item) => item.filepath));
    }
  };

  const toggleGroupSelected = (group: SessionGroup) => {
    const allSelected = group.sessions.every((session) => selectedFilepaths.includes(session.filepath));
    setSelectedFilepaths((prev) => {
      const next = new Set(prev);
      if (allSelected) {
        group.sessions.forEach((session) => next.delete(session.filepath));
      } else {
        group.sessions.forEach((session) => next.add(session.filepath));
      }
      return Array.from(next);
    });
  };

  const handleMoveToTrash = async () => {
    if (selectedFilepaths.length === 0) {
      pushActionMessage({ text: "请至少选择一条会话", tone: "error" });
      return;
    }
    setConfirmDialog({
      title: "移到废纸篓",
      description: `确认将选中的 ${selectedFilepaths.length} 条会话移到废纸篓吗？`,
      confirmLabel: "确认移动",
      tone: "danger",
      action: async () => {
        try {
          const result = await invoke<{ message: string }>("move_sessions_to_trash", { filepaths: selectedFilepaths });
          pushActionMessage({ text: result.message, tone: "success" });
          setSelectedFilepaths([]);
          if (selectedSession && selectedFilepaths.includes(selectedSession.filepath)) {
            setSelectedSession(null);
            setMessages([]);
          }
          await fetchAll();
        } catch (e) {
          pushActionMessage({ text: "移动会话失败：" + String(e), tone: "error" });
          throw e;
        }
      },
    });
  };

  const handleRepairCodexIndex = async () => {
    try {
      const result = await invoke<{ message: string }>("repair_codex_session_index");
      pushActionMessage({ text: result.message, tone: "success" });
      await fetchAll();
    } catch (e) {
      pushActionMessage({ text: "修复 Codex 会话索引失败：" + String(e), tone: "error" });
    }
  };

  const handleSyncCodexThreads = async () => {
    try {
      const result = await invoke<{ message: string }>("sync_codex_threads_across_instances");
      pushActionMessage({ text: result.message, tone: "success" });
      await fetchAll();
    } catch (e) {
      pushActionMessage({ text: "同步 Codex 线程失败：" + String(e), tone: "error" });
    }
  };

  const handlePickCodexDir = async () => {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "选择 Codex 实例目录",
    });
    if (typeof selected === "string") {
      setCodexInstanceDir(selected);
    }
  };

  const handleAddCodexInstance = async () => {
    if (!codexInstanceName.trim() || !codexInstanceDir.trim()) {
      pushActionMessage({ text: "请填写实例名称并选择目录", tone: "error" });
      return;
    }
    setCodexInstanceLoading(true);
    try {
      await invoke("add_codex_instance", {
        name: codexInstanceName.trim(),
        userDataDir: codexInstanceDir.trim(),
      });
      setCodexInstanceName("");
      setCodexInstanceDir("");
      pushActionMessage({ text: "Codex 实例已添加", tone: "success" });
      await fetchAll();
    } catch (e) {
      pushActionMessage({ text: "添加 Codex 实例失败：" + String(e), tone: "error" });
    } finally {
      setCodexInstanceLoading(false);
    }
  };

  const handleDeleteCodexInstance = async (id: string) => {
    setConfirmDialog({
      title: "删除 Codex 实例",
      description: "确认删除这个 Codex 实例目录吗？不会删除真实文件，只会移除索引。",
      confirmLabel: "删除",
      tone: "danger",
      action: async () => {
        try {
          await invoke("delete_codex_instance", { id });
          pushActionMessage({ text: "Codex 实例已删除", tone: "success" });
          await fetchAll();
        } catch (e) {
          pushActionMessage({ text: "删除 Codex 实例失败：" + String(e), tone: "error" });
          throw e;
        }
      },
    });
  };

  const handleUpdateCodexInstanceSettings = async (instance: CodexInstanceRecord) => {
    setCodexSettingsDialog({
      instance,
      extraArgs: instance.extra_args || "",
      bindAccountId: instance.bind_account_id || "",
      bindProviderId: instance.bind_provider_id || "",
      followLocalAccount: !!instance.follow_local_account,
    });
  };

  const handleStartCodexInstance = async (id: string) => {
    try {
      await invoke("start_codex_instance", { id });
      pushActionMessage({ text: "Codex 实例已启动", tone: "success" });
      await fetchAll();
    } catch (e) {
      pushActionMessage({ text: "启动 Codex 实例失败：" + String(e), tone: "error" });
    }
  };

  const handleStopCodexInstance = async (id: string) => {
    try {
      await invoke("stop_codex_instance", { id });
      pushActionMessage({ text: "Codex 实例已停止", tone: "success" });
      await fetchAll();
    } catch (e) {
      pushActionMessage({ text: "停止 Codex 实例失败：" + String(e), tone: "error" });
    }
  };

  const handleOpenCodexWindow = async (id: string) => {
    try {
      await invoke("open_codex_instance_window", { id });
      pushActionMessage({ text: "已尝试切换到 Codex 实例窗口", tone: "success" });
    } catch (e) {
      pushActionMessage({ text: "切换 Codex 实例窗口失败：" + String(e), tone: "error" });
    }
  };

  const handleCloseAllCodexInstances = async () => {
    setConfirmDialog({
      title: "关闭全部 Codex 实例",
      description: "确认关闭所有 Codex 实例吗？",
      confirmLabel: "全部关闭",
      tone: "danger",
      action: async () => {
        try {
          await invoke("close_all_codex_instances");
          pushActionMessage({ text: "已关闭全部 Codex 实例", tone: "success" });
          await fetchAll();
        } catch (e) {
          pushActionMessage({ text: "关闭全部 Codex 实例失败：" + String(e), tone: "error" });
          throw e;
        }
      },
    });
  };

  const selectedSessions = useMemo(
    () => sessions.filter((item) => selectedFilepaths.includes(item.filepath)),
    [sessions, selectedFilepaths]
  );

  const selectedGroupsCount = useMemo(
    () =>
      sessionGroups.filter((group) =>
        group.sessions.some((item) => selectedFilepaths.includes(item.filepath))
      ).length,
    [sessionGroups, selectedFilepaths]
  );

  const formatCwdLabel = (cwd: string) => {
    if (!cwd) return "未识别工作目录";
    const normalized = cwd.replace(/\\/g, "/");
    return normalized.length > 46 ? `...${normalized.slice(-46)}` : normalized;
  };

  const handleCopyText = (text: string, successText: string) => {
    navigator.clipboard
      .writeText(text)
      .then(() => pushActionMessage({ text: successText, tone: "success" }))
      .catch((e) => pushActionMessage({ text: "复制失败：" + String(e), tone: "error" }));
  };

  const codexInstanceCards = useMemo(() => {
    const allInstances = [...(defaultCodexInstance ? [defaultCodexInstance] : []), ...codexInstances];
    return allInstances.map((instance) => {
      const sessionCount = sessions.filter((session) => {
        if (session.tool_type !== "Codex") return false;
        if (instance.is_default) {
          return !session.instance_name || session.instance_name === instance.name;
        }
        return session.instance_name === instance.name;
      }).length;
      return { ...instance, sessionCount };
    });
  }, [defaultCodexInstance, codexInstances, sessions]);

  const runningCodexInstances = useMemo(
    () => codexInstanceCards.filter((item) => item.running).length,
    [codexInstanceCards]
  );

  return (
    <div className="sessions-page cyberpunk-theme">
      {/* 侧边栏结构 */}
      <div className="sessions-sidebar cyber-sidebar">
        <div className="sessions-header">
          <h2 className="cyber-title-sm">
            <Activity size={16} className="pulse-icon text-accent" /> ZOMBIE_RADAR // 全域劫持雷达
          </h2>
          <div className="sessions-header-actions">
            <button
              className="cyber-icon-btn"
              onClick={handleRepairCodexIndex}
              title="修复 Codex 会话索引"
            >
              <Folder size={14} />
            </button>
            <button
              className="cyber-icon-btn"
              onClick={handleSyncCodexThreads}
              title="同步 Codex 缺失线程"
              disabled={codexInstanceCount < 2}
            >
              <RefreshCw size={14} />
            </button>
            <button
              className="cyber-icon-btn"
              onClick={() => setShowCodexInstances(true)}
              title="管理 Codex 实例目录"
            >
              <Cpu size={14} />
            </button>
            <button
              className="cyber-icon-btn danger"
              onClick={handleMoveToTrash}
              disabled={selectedFilepaths.length === 0}
              title="将选中的会话移到废纸篓"
            >
              <Skull size={14} />
            </button>
            <button className="cyber-icon-btn" onClick={fetchAll} disabled={loading} title="刷新系统探针">
              <RefreshCw size={14} className={loading ? "spin" : ""} />
            </button>
          </div>
        </div>
        {selectedFilepaths.length > 0 && (
          <div className="session-batch-bar">
            <div className="session-batch-title">批量处理队列</div>
            <div className="session-batch-meta">
              已选 {selectedSessions.length} 条会话，覆盖 {selectedGroupsCount} 个工作区
            </div>
            <div className="session-batch-actions">
              <button className="btn btn-ghost btn-xs" onClick={toggleAllSessionsSelected}>
                {selectedFilepaths.length === sessions.length ? "取消全选" : "全选全部"}
              </button>
              <button className="btn btn-ghost btn-xs" onClick={() => setSelectedFilepaths([])}>
                清空选择
              </button>
              <button className="btn btn-danger-ghost btn-xs" onClick={handleMoveToTrash}>
                移到废纸篓
              </button>
            </div>
          </div>
        )}

        {actionMessages.length > 0 && (
          <div className="session-action-stack">
            <div className="session-action-stack-header">
              <span>操作结果</span>
              <button className="btn btn-ghost btn-xs" onClick={clearActionMessages}>清空</button>
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
        )}
        
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
                <button className="cyber-btn-mini toxic" onClick={() => pushActionMessage({ text: `功能研发中：自动修改 ${z.tool_type} 的路由并热重启进程`, tone: "info" })}>
                  <Skull size={10}/> 注入毒素代理
                </button>
              </div>
            </div>
          ))}

          <div className="section-divider mt-4">
            <span>[ CODEX 多实例 ] INSTANCE_MATRIX</span>
          </div>

          <div className="instance-overview-card">
            <div className="instance-overview-stats">
              <div className="instance-overview-stat">
                <span className="instance-overview-value">{codexInstanceCards.length}</span>
                <span className="instance-overview-label">实例</span>
              </div>
              <div className="instance-overview-stat">
                <span className="instance-overview-value">{runningCodexInstances}</span>
                <span className="instance-overview-label">运行中</span>
              </div>
              <div className="instance-overview-stat">
                <span className="instance-overview-value">
                  {sessions.filter((item) => item.tool_type === "Codex").length}
                </span>
                <span className="instance-overview-label">Codex 会话</span>
              </div>
            </div>
            <div className="instance-overview-list">
              {codexInstanceCards.map((item) => (
                <button
                  key={item.id}
                  className="instance-overview-item"
                  onClick={() => setShowCodexInstances(true)}
                  title={item.user_data_dir}
                >
                  <div className="instance-overview-item-main">
                    <span className="instance-overview-item-name">
                      {item.name}
                      {item.is_default ? " · 默认" : ""}
                    </span>
                    <span className={`instance-overview-state ${item.running ? "running" : "stopped"}`}>
                      {item.running ? "运行中" : "未运行"}
                    </span>
                  </div>
                  <div className="instance-overview-item-meta">
                    <span>{item.sessionCount} 条会话</span>
                    <span>{formatCwdLabel(item.user_data_dir)}</span>
                  </div>
                </button>
              ))}
            </div>
            <div className="instance-overview-actions">
              <button className="btn btn-ghost btn-xs" onClick={() => setShowCodexInstances(true)}>
                打开实例面板
              </button>
            </div>
          </div>

          {/* Sessions Section */}
          <div className="section-divider mt-4">
            <span>[ 沉睡数据缓冲 ] OFFLINE_CACHE</span>
          </div>

          {sessions.length === 0 && !loading && (
            <div className="empty-text">未发现沉睡的历史数据</div>
          )}

          {sessionGroups.map((group) => {
            const isExpanded = expandedGroups.includes(group.cwd);
            const allSelected = group.sessions.length > 0 && group.sessions.every((session) => selectedFilepaths.includes(session.filepath));
            return (
              <div key={group.cwd} className="session-group">
                <button
                  className="session-group-header"
                  onClick={() => toggleGroupExpanded(group.cwd)}
                  title={group.cwd}
                >
                  <div className="session-group-left">
                    <input
                      type="checkbox"
                      checked={allSelected}
                      onChange={(e) => {
                        e.stopPropagation();
                        toggleGroupSelected(group);
                      }}
                      onClick={(e) => e.stopPropagation()}
                    />
                    {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                    <Folder size={14} />
                    <div className="session-group-texts">
                      <span className="session-group-label">{group.label}</span>
                      <span className="session-group-path">{formatCwdLabel(group.cwd)}</span>
                    </div>
                    <span className="session-group-count">
                      {group.sessions.length}
                      {group.sessions.some((session) => selectedFilepaths.includes(session.filepath))
                        ? ` / 已选 ${group.sessions.filter((session) => selectedFilepaths.includes(session.filepath)).length}`
                        : ""}
                    </span>
                  </div>
                  <span className="session-group-time">{formatDate(group.updated_at)}</span>
                </button>
                {isExpanded && (
                  <div className="session-group-children">
                    {group.sessions.map((s) => (
                      <div 
                        key={s.filepath} 
                        className={`session-item ${selectedSession?.filepath === s.filepath ? 'active' : ''}`}
                        onClick={() => loadSession(s)}
                      >
                        <div className="session-item-select">
                          <input
                            type="checkbox"
                            checked={selectedFilepaths.includes(s.filepath)}
                            onChange={(e) => {
                              e.stopPropagation();
                              toggleSessionSelected(s.filepath);
                            }}
                            onClick={(e) => e.stopPropagation()}
                          />
                        </div>
                        <div className="session-item-title">{s.title || "Unnamed Chat"}</div>
                        <div className="session-item-meta">
                          <span>
                            {s.tool_type || "Unknown"}
                            {s.instance_name ? ` / ${s.instance_name}` : ""}
                          </span>
                          <span className="text-accent"><MessageSquare size={10}/> {s.messages_count} logs</span>
                          <span>L: {formatDate(s.updated_at)}</span>
                        </div>
                        <div className="session-item-path" title={getSessionCwd(s) || s.filepath}>
                          {getSessionCwd(s) || s.filepath}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* 详情主视口 */}
      <div className="session-content cyber-main">
        {selectedSession ? (
          <>
            <div className="session-content-header cyber-header-box" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <div>
                <h3 className="glow-text">{selectedSession.title}</h3>
                <div className="session-path-text">
                  {selectedSession.filepath}
                </div>
                <div className="session-location-row">
                  <span className="session-location-chip">工作区：{getSessionCwd(selectedSession) || "未识别"}</span>
                  <span className="session-location-chip">工具：{selectedSession.tool_type || "Unknown"}</span>
                  <span className="session-location-chip">消息：{selectedSession.messages_count}</span>
                </div>
                {selectedSession.tool_type === "Codex" && (
                  <button className="session-instance-chip clickable" onClick={() => setShowCodexInstances(true)}>
                    实例：{selectedSession.instance_name || "默认实例"}
                  </button>
                )}
              </div>
              <div className="session-actions" style={{ display: 'flex', gap: '8px' }}>
                <button className="btn btn-ghost btn-sm" onClick={() => handleCopyDir(selectedSession)} title="复制会话对应的工作目录">
                  <Folder size={14} style={{ marginRight: 4 }} /> 目录
                </button>
                <button className="btn btn-ghost btn-sm" onClick={() => handleCopyCmd(selectedSession)} title="生成恢复脚本并复制剪贴板">
                  <Copy size={14} style={{ marginRight: 4 }} /> 复制指令
                </button>
                <button className="btn btn-primary btn-sm" onClick={() => handleLaunchTerminal(selectedSession)} title="在新终端以当前目录直接拉起">
                  <Terminal size={14} style={{ marginRight: 4 }} /> 外置终端拉起
                </button>
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

      {showCodexInstances && (
        <div className="modal-overlay" onClick={() => setShowCodexInstances(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>Codex 实例目录</h2>
              <button className="btn btn-icon" onClick={() => setShowCodexInstances(false)}>✕</button>
            </div>
            <div className="modal-body">
              <div className="alert alert-info" style={{ marginBottom: 16 }}>
                默认实例 <code>~/.codex</code> 会自动生效，这里只管理额外实例目录。
              </div>

              {defaultCodexInstance && (
                <div className="session-instance-item" style={{ marginBottom: 16 }}>
                  <div>
                    <div className="session-instance-title">{defaultCodexInstance.name}</div>
                    <div className="session-instance-path">{defaultCodexInstance.user_data_dir}</div>
                    <div className="session-instance-meta">
                      <span>{defaultCodexInstance.running ? `运行中 PID ${defaultCodexInstance.last_pid}` : "未运行"}</span>
                      <span>{codexInstanceCards.find((item) => item.id === defaultCodexInstance.id)?.sessionCount ?? 0} 条会话</span>
                      <span>{defaultCodexInstance.follow_local_account ? "跟随当前本地账号" : (defaultCodexInstance.bind_account_id ? `绑定账号 ${defaultCodexInstance.bind_account_id}` : "未绑定账号")}</span>
                      <span>{defaultCodexInstance.bind_provider_id ? `绑定 Provider ${defaultCodexInstance.bind_provider_id}` : "使用当前激活 Provider"}</span>
                      <span>{defaultCodexInstance.extra_args ? `参数 ${defaultCodexInstance.extra_args}` : "无额外参数"}</span>
                    </div>
                    <div className="session-instance-flags">
                      <span className={`instance-flag ${defaultCodexInstance.has_state_db ? "ok" : "bad"}`}>
                        state_5.sqlite
                      </span>
                      <span className={`instance-flag ${defaultCodexInstance.has_session_index ? "ok" : "bad"}`}>
                        session_index.jsonl
                      </span>
                    </div>
                  </div>
                  <div className="session-instance-actions">
                    <span className="badge badge-success">默认</span>
                    <button className="btn btn-ghost btn-xs" onClick={() => handleCopyText(defaultCodexInstance.user_data_dir, "实例目录已复制")}>
                      复制路径
                    </button>
                    <button className="btn btn-ghost btn-xs" onClick={() => handleUpdateCodexInstanceSettings(defaultCodexInstance)}>
                      设置
                    </button>
                    {defaultCodexInstance.running ? (
                      <>
                        <button className="btn btn-ghost btn-xs" onClick={() => handleOpenCodexWindow(defaultCodexInstance.id)}>
                          打开
                        </button>
                        <button className="btn btn-danger-ghost btn-xs" onClick={() => handleStopCodexInstance(defaultCodexInstance.id)}>
                          停止
                        </button>
                      </>
                    ) : (
                      <button className="btn btn-primary btn-xs" onClick={() => handleStartCodexInstance(defaultCodexInstance.id)}>
                        启动
                      </button>
                    )}
                  </div>
                </div>
              )}

              <div className="form-row">
                <label className="form-label">实例名称</label>
                <input
                  className="form-input"
                  value={codexInstanceName}
                  onChange={(e) => setCodexInstanceName(e.target.value)}
                  placeholder="例如：工作目录实例 / 沙盒实例"
                />
              </div>

              <div className="form-row">
                <label className="form-label">实例目录</label>
                <div style={{ display: "flex", gap: 8 }}>
                  <input
                    className="form-input font-mono"
                    value={codexInstanceDir}
                    onChange={(e) => setCodexInstanceDir(e.target.value)}
                    placeholder="选择或粘贴 Codex user data 目录"
                  />
                  <button type="button" className="btn btn-ghost" onClick={handlePickCodexDir}>
                    浏览
                  </button>
                </div>
              </div>

              <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 16 }}>
                <button className="btn btn-danger-ghost" style={{ marginRight: 8 }} onClick={handleCloseAllCodexInstances}>
                  全部关闭
                </button>
                <button className="btn btn-primary" onClick={handleAddCodexInstance} disabled={codexInstanceLoading}>
                  {codexInstanceLoading ? "添加中..." : "添加实例"}
                </button>
              </div>

              <div className="session-instance-list">
                {codexInstances.length === 0 ? (
                  <div className="empty-text">当前还没有额外 Codex 实例</div>
                ) : (
                  codexInstances.map((item) => (
                    <div key={item.id} className="session-instance-item">
                      <div>
                        <div className="session-instance-title">{item.name}</div>
                        <div className="session-instance-path">{item.user_data_dir}</div>
                        <div className="session-instance-meta">
                          <span>{item.running ? `运行中 PID ${item.last_pid}` : "未运行"}</span>
                          <span>{codexInstanceCards.find((card) => card.id === item.id)?.sessionCount ?? 0} 条会话</span>
                          <span>{item.bind_account_id ? `绑定账号 ${item.bind_account_id}` : "未绑定账号"}</span>
                          <span>{item.bind_provider_id ? `绑定 Provider ${item.bind_provider_id}` : "使用当前激活 Provider"}</span>
                          <span>{item.extra_args ? `参数 ${item.extra_args}` : "无额外参数"}</span>
                        </div>
                        <div className="session-instance-flags">
                          <span className={`instance-flag ${item.has_state_db ? "ok" : "bad"}`}>
                            state_5.sqlite
                          </span>
                          <span className={`instance-flag ${item.has_session_index ? "ok" : "bad"}`}>
                            session_index.jsonl
                          </span>
                        </div>
                      </div>
                      <div className="session-instance-actions">
                        <button className="btn btn-ghost btn-xs" onClick={() => handleCopyText(item.user_data_dir, "实例目录已复制")}>
                          复制路径
                        </button>
                        <button className="btn btn-ghost btn-xs" onClick={() => handleUpdateCodexInstanceSettings(item)}>
                          设置
                        </button>
                        {item.running ? (
                          <>
                            <button className="btn btn-ghost btn-xs" onClick={() => handleOpenCodexWindow(item.id)}>
                              打开
                            </button>
                            <button className="btn btn-danger-ghost btn-xs" onClick={() => handleStopCodexInstance(item.id)}>
                              停止
                            </button>
                          </>
                        ) : (
                          <button className="btn btn-primary btn-xs" onClick={() => handleStartCodexInstance(item.id)}>
                            启动
                          </button>
                        )}
                        <button className="btn btn-danger-ghost btn-xs" onClick={() => handleDeleteCodexInstance(item.id)}>
                          删除
                        </button>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        </div>
      )}

      {confirmDialog && (
        <div className="modal-overlay" onClick={() => !confirmDialogBusy && setConfirmDialog(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>{confirmDialog.title}</h2>
              <button className="btn btn-icon" onClick={() => setConfirmDialog(null)}>✕</button>
            </div>
            <div className="modal-body">
              <p>{confirmDialog.description}</p>
              <div className="modal-footer">
                <button className="btn btn-ghost" onClick={() => setConfirmDialog(null)} disabled={confirmDialogBusy}>取消</button>
                <button
                  className={confirmDialog.tone === "danger" ? "btn btn-danger" : "btn btn-primary"}
                  disabled={confirmDialogBusy}
                  onClick={async () => {
                    try {
                      setConfirmDialogBusy(true);
                      await confirmDialog.action();
                      setConfirmDialog(null);
                    } finally {
                      setConfirmDialogBusy(false);
                    }
                  }}
                >
                  {confirmDialogBusy ? "处理中..." : confirmDialog.confirmLabel}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {codexSettingsDialog && (
        <div className="modal-overlay" onClick={() => setCodexSettingsDialog(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>设置 Codex 实例</h2>
              <button className="btn btn-icon" onClick={() => setCodexSettingsDialog(null)}>✕</button>
            </div>
            <div className="modal-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <div className="form-row">
                <label className="form-label">额外启动参数</label>
                <input
                  className="form-input"
                  value={codexSettingsDialog.extraArgs}
                  onChange={(e) => setCodexSettingsDialog({ ...codexSettingsDialog, extraArgs: e.target.value })}
                />
              </div>
              <div className="form-row">
                <label className="form-label">绑定账号 ID</label>
                <input
                  className="form-input"
                  value={codexSettingsDialog.bindAccountId}
                  onChange={(e) => setCodexSettingsDialog({ ...codexSettingsDialog, bindAccountId: e.target.value })}
                  disabled={codexSettingsDialog.instance.is_default && codexSettingsDialog.followLocalAccount}
                />
              </div>
              <div className="form-row">
                <label className="form-label">绑定 Provider</label>
                <select
                  className="form-input"
                  value={codexSettingsDialog.bindProviderId}
                  onChange={(e) => setCodexSettingsDialog({ ...codexSettingsDialog, bindProviderId: e.target.value })}
                >
                  <option value="">使用当前激活 Provider</option>
                  {codexProviders.map((provider) => (
                    <option key={provider.id} value={provider.id}>
                      {provider.name}{provider.is_active ? " (当前激活)" : ""}
                    </option>
                  ))}
                </select>
              </div>
              {codexSettingsDialog.instance.is_default && (
                <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13 }}>
                  <input
                    type="checkbox"
                    checked={codexSettingsDialog.followLocalAccount}
                    onChange={(e) => setCodexSettingsDialog({ ...codexSettingsDialog, followLocalAccount: e.target.checked })}
                  />
                  跟随当前本地 Codex 账号
                </label>
              )}
              <div className="modal-footer">
                <button className="btn btn-ghost" onClick={() => setCodexSettingsDialog(null)}>取消</button>
                <button
                  className="btn btn-primary"
                  onClick={async () => {
                    try {
                      await invoke("update_codex_instance_settings", {
                        id: codexSettingsDialog.instance.id,
                        extraArgs: codexSettingsDialog.extraArgs,
                        bindAccountId: codexSettingsDialog.bindAccountId.trim() || null,
                        bindProviderId: codexSettingsDialog.bindProviderId.trim() || null,
                        followLocalAccount: codexSettingsDialog.instance.is_default
                          ? codexSettingsDialog.followLocalAccount
                          : undefined,
                      });
                      pushActionMessage({ text: "Codex 实例设置已更新", tone: "success" });
                      setCodexSettingsDialog(null);
                      await fetchAll();
                    } catch (e) {
                      pushActionMessage({ text: "更新 Codex 实例设置失败：" + String(e), tone: "error" });
                    }
                  }}
                >
                  保存
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
