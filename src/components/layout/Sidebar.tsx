import type { NavPage } from "../../App";
import { useTranslation } from "react-i18next";
import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PLATFORM_LABELS } from "../../types";
import {
  SIDEBAR_ACTIVE_LAYOUT_STORAGE_KEY,
  SIDEBAR_LAYOUTS_STORAGE_KEY,
  loadSidebarLayouts,
  pickActiveLayoutId,
  type SidebarLayout,
} from "./sidebarLayoutStore";
import "./Sidebar.css";

interface NavItem {
  id: NavPage;
  icon: string;
  label: string;
  badge?: string;
}

interface NavGroup {
  key: string;
  title?: string;
  items: NavItem[];
}

const NAV_GROUPS: NavGroup[] = [
  {
    key: "overview",
    items: [
      { id: "dashboard", icon: "⬡", label: "总览" },
    ],
  },
  {
    key: "gateway",
    title: "核心网关 (Gateway)",
    items: [
      { id: "accounts", icon: "👤", label: "账号与资产库" },
      { id: "sharing", icon: "🔑", label: "分享发卡与额度" },
      { id: "proxy", icon: "↔", label: "本地路由与隧道" },
      { id: "security", icon: "🛡️", label: "哨站风控台" },
    ],
  },
  {
    key: "analytics",
    title: "系统统计 (Analytics)",
    items: [
      { id: "analytics", icon: "📊", label: "用量看板" },
      { id: "report", icon: "📰", label: "Web 报告" },
      { id: "logs", icon: "📜", label: "桌面日志" },
      { id: "sessions", icon: "💬", label: "流转日志" },
      { id: "speedtest", icon: "⚡", label: "节点测速" },
    ],
  },
  {
    key: "local_tools",
    title: "本地研发赋能 (Local Tools)",
    items: [
      { id: "providers", icon: "🔌", label: "终端配置" },
      { id: "mcp", icon: "🔌", label: "MCP 扩展" },
      { id: "skills", icon: "🛠️", label: "技能包" },
      { id: "tools", icon: "⏬", label: "本地兵工厂" },
      { id: "tokenCalculator", icon: "🧮", label: "TOKEN 计算器" },
      { id: "mfa", icon: "🔐", label: "2FA / MFA 管理" },
      { id: "wakeup", icon: "⏰", label: "Wakeup / Verification" },
      { id: "prompts", icon: "📝", label: "全局注入与 Prompt" },
    ],
  },
  {
    key: "platform",
    title: "平台",
    items: [
      { id: "models", icon: "🤖", label: "模型字典库" },
      { id: "settings", icon: "⚙", label: "全局设置" },
    ],
  },
];

const ALL_GROUP_KEYS = NAV_GROUPS.map((group) => group.key);
const EXTRA_TRAY_PLATFORMS: Record<string, string> = {
  codex: "Codex",
  cursor: "Cursor",
  windsurf: "Windsurf",
  kiro: "Kiro",
  qoder: "Qoder",
  trae: "Trae",
  codebuddy: "CodeBuddy",
  codebuddy_cn: "CodeBuddy CN",
  workbuddy: "WorkBuddy",
  zed: "Zed",
  claude_code: "Claude Code",
  aider: "Aider",
  open_code: "OpenCode",
  open_claw: "OpenClaw",
};
const TRAY_PLATFORM_OPTIONS: { id: string; label: string }[] = [
  ...Object.entries(PLATFORM_LABELS).map(([id, label]) => ({ id, label })),
  ...Object.entries(EXTRA_TRAY_PLATFORMS).map(([id, label]) => ({ id, label })),
];
const ALL_TRAY_PLATFORM_KEYS = TRAY_PLATFORM_OPTIONS.map((item) => item.id);

const DEFAULT_LAYOUTS: SidebarLayout[] = [
  {
    id: "layout-default",
    name: "默认布局",
    group_keys: [...ALL_GROUP_KEYS],
    tray_platforms: [],
  },
];

interface SidebarProps {
  activePage: NavPage;
  onNavigate: (page: NavPage) => void;
}

export default function Sidebar({ activePage, onNavigate }: SidebarProps) {
  const { t } = useTranslation();
  const [layouts, setLayouts] = useState<SidebarLayout[]>(() =>
    loadSidebarLayouts(ALL_GROUP_KEYS, ALL_TRAY_PLATFORM_KEYS, DEFAULT_LAYOUTS)
  );
  const [activeLayoutId, setActiveLayoutId] = useState<string>(() => {
    try {
      return localStorage.getItem(SIDEBAR_ACTIVE_LAYOUT_STORAGE_KEY) || DEFAULT_LAYOUTS[0].id;
    } catch {
      return DEFAULT_LAYOUTS[0].id;
    }
  });
  const [showLayoutManager, setShowLayoutManager] = useState(false);

  useEffect(() => {
    setActiveLayoutId((prev) => pickActiveLayoutId(layouts, prev));
  }, [layouts, activeLayoutId]);

  useEffect(() => {
    try {
      localStorage.setItem(SIDEBAR_LAYOUTS_STORAGE_KEY, JSON.stringify(layouts));
      localStorage.setItem(SIDEBAR_ACTIVE_LAYOUT_STORAGE_KEY, activeLayoutId);
    } catch {
      // ignore storage failures
    }
  }, [layouts, activeLayoutId]);

  const activeLayout = useMemo(
    () => layouts.find((layout) => layout.id === activeLayoutId) || layouts[0] || DEFAULT_LAYOUTS[0],
    [layouts, activeLayoutId]
  );

  useEffect(() => {
    const platforms = activeLayout.tray_platforms || [];
    void invoke("tray_set_platform_scope", { platforms }).catch(() => {
      // ignore when backend command is unavailable
    });
  }, [activeLayout.id, JSON.stringify(activeLayout.tray_platforms || [])]);

  const visibleGroups = useMemo(() => {
    const visibleSet = new Set(activeLayout.group_keys);
    return NAV_GROUPS.filter((group) => visibleSet.has(group.key));
  }, [activeLayout]);

  const moveLayout = (layoutId: string, direction: "up" | "down") => {
    setLayouts((prev) => {
      const index = prev.findIndex((item) => item.id === layoutId);
      if (index === -1) return prev;
      const target = direction === "up" ? index - 1 : index + 1;
      if (target < 0 || target >= prev.length) return prev;
      const next = [...prev];
      const [current] = next.splice(index, 1);
      next.splice(target, 0, current);
      return next;
    });
  };

  const addLayout = () => {
    const nextId = `layout-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
    const nextLayout: SidebarLayout = {
      id: nextId,
      name: `新布局 ${layouts.length + 1}`,
      group_keys: [...activeLayout.group_keys],
      tray_platforms: [...(activeLayout.tray_platforms || [])],
    };
    setLayouts((prev) => [...prev, nextLayout]);
    setActiveLayoutId(nextId);
  };

  const deleteLayout = (layoutId: string) => {
    setLayouts((prev) => {
      if (prev.length <= 1) return prev;
      const next = prev.filter((item) => item.id !== layoutId);
      if (activeLayoutId === layoutId) {
        setActiveLayoutId(next[0]?.id || DEFAULT_LAYOUTS[0].id);
      }
      return next;
    });
  };

  const toggleLayoutGroup = (layoutId: string, groupKey: string) => {
    setLayouts((prev) =>
      prev.map((layout) => {
        if (layout.id !== layoutId) return layout;
        const hasGroup = layout.group_keys.includes(groupKey);
        if (hasGroup && layout.group_keys.length <= 1) {
          return layout;
        }
        return {
          ...layout,
          group_keys: hasGroup
            ? layout.group_keys.filter((key) => key !== groupKey)
            : [...layout.group_keys, groupKey],
        };
      })
    );
  };

  const setLayoutTrayPlatforms = (layoutId: string, platforms: string[]) => {
    setLayouts((prev) =>
      prev.map((layout) =>
        layout.id === layoutId ? { ...layout, tray_platforms: [...new Set(platforms)] } : layout
      )
    );
  };

  const toggleLayoutTrayPlatform = (layoutId: string, platformId: string) => {
    setLayouts((prev) =>
      prev.map((layout) => {
        if (layout.id !== layoutId) return layout;
        const current = layout.tray_platforms || [];
        if (current.length === 0) {
          return { ...layout, tray_platforms: [platformId] };
        }
        return current.includes(platformId)
          ? { ...layout, tray_platforms: current.filter((id) => id !== platformId) }
          : { ...layout, tray_platforms: [...current, platformId] };
      })
    );
  };

  const isTrayAllPlatforms = (activeLayout.tray_platforms || []).length === 0;

  return (
    <aside className="sidebar">
      {/* Logo */}
      <div className="sidebar-logo">
        <div className="sidebar-logo-icon">
          <span>✦</span>
        </div>
        <div>
          <div className="sidebar-logo-title">AI Singularity</div>
          <div className="sidebar-logo-sub">Control Center</div>
        </div>
      </div>
      <div className="sidebar-layout-switcher">
        <select
          className="sidebar-layout-select"
          value={activeLayout.id}
          onChange={(e) => setActiveLayoutId(e.target.value)}
        >
          {layouts.map((layout) => (
            <option key={layout.id} value={layout.id}>
              {layout.name}
            </option>
          ))}
        </select>
        <button className="sidebar-layout-manage-btn" onClick={() => setShowLayoutManager(true)}>
          布局
        </button>
      </div>

      {/* 导航 */}
      <nav className="sidebar-nav">
        {visibleGroups.map((group, gi) => (
          <div key={gi} className="sidebar-group">
            {group.title && (
              <div className="sidebar-group-title">
                 {group.title}
              </div>
            )}
            {group.items.map((item) => (
              <button
                key={item.id}
                className={`sidebar-nav-item ${activePage === item.id ? "active" : ""}`}
                onClick={() => onNavigate(item.id)}
              >
                <span className="sidebar-nav-icon">{item.icon}</span>
                <span className="sidebar-nav-label">
                  {t(`sidebar.${item.id}`, item.label)}
                </span>
                {item.badge && (
                  <span className="sidebar-nav-badge">{item.badge}</span>
                )}
              </button>
            ))}
          </div>
        ))}
      </nav>

      {/* 底部版本 */}
      <div className="sidebar-footer">
        <div className="text-muted" style={{ fontSize: 12 }}>v0.1.0-alpha</div>
      </div>

      {showLayoutManager && (
        <div className="sidebar-layout-modal-overlay" onClick={() => setShowLayoutManager(false)}>
          <div className="sidebar-layout-modal" onClick={(e) => e.stopPropagation()}>
            <div className="sidebar-layout-modal-header">
              <strong>平台布局管理器</strong>
              <button className="sidebar-layout-close-btn" onClick={() => setShowLayoutManager(false)}>
                ✕
              </button>
            </div>
            <div className="sidebar-layout-list">
              {layouts.map((layout, index) => (
                <div key={layout.id} className={`sidebar-layout-item ${layout.id === activeLayout.id ? "active" : ""}`}>
                  <button className="sidebar-layout-use-btn" onClick={() => setActiveLayoutId(layout.id)}>
                    {layout.id === activeLayout.id ? "当前" : "切换"}
                  </button>
                  <input
                    className="sidebar-layout-name-input"
                    value={layout.name}
                    onChange={(e) =>
                      setLayouts((prev) =>
                        prev.map((item) =>
                          item.id === layout.id ? { ...item, name: e.target.value || "未命名布局" } : item
                        )
                      )
                    }
                  />
                  <button
                    className="sidebar-layout-op-btn"
                    onClick={() => moveLayout(layout.id, "up")}
                    disabled={index === 0}
                  >
                    ↑
                  </button>
                  <button
                    className="sidebar-layout-op-btn"
                    onClick={() => moveLayout(layout.id, "down")}
                    disabled={index === layouts.length - 1}
                  >
                    ↓
                  </button>
                  <button
                    className="sidebar-layout-op-btn danger"
                    onClick={() => deleteLayout(layout.id)}
                    disabled={layouts.length <= 1}
                  >
                    删
                  </button>
                </div>
              ))}
            </div>
            <button className="sidebar-layout-add-btn" onClick={addLayout}>
              新增布局
            </button>
            <div className="sidebar-layout-groups">
              <div className="sidebar-layout-groups-title">当前布局可见分组</div>
              {NAV_GROUPS.map((group) => (
                <label key={group.key} className="sidebar-layout-group-row">
                  <input
                    type="checkbox"
                    checked={activeLayout.group_keys.includes(group.key)}
                    onChange={() => toggleLayoutGroup(activeLayout.id, group.key)}
                  />
                  <span>{group.title || "总览"}</span>
                </label>
              ))}
            </div>
            <div className="sidebar-layout-groups">
              <div className="sidebar-layout-groups-title">托盘显示范围（按平台）</div>
              <div className="sidebar-layout-tray-actions">
                <button
                  className="sidebar-layout-op-btn"
                  onClick={() => setLayoutTrayPlatforms(activeLayout.id, [])}
                  disabled={isTrayAllPlatforms}
                >
                  全平台
                </button>
              </div>
              {!isTrayAllPlatforms && (
                <div className="sidebar-layout-groups-title" style={{ marginTop: 4 }}>
                  当前已限制 {activeLayout.tray_platforms?.length || 0} 个平台
                </div>
              )}
              {TRAY_PLATFORM_OPTIONS.map((platform) => {
                const selected = isTrayAllPlatforms || (activeLayout.tray_platforms || []).includes(platform.id);
                return (
                  <label key={platform.id} className="sidebar-layout-group-row">
                    <input
                      type="checkbox"
                      checked={selected}
                      onChange={() => toggleLayoutTrayPlatform(activeLayout.id, platform.id)}
                    />
                    <span>{platform.label}</span>
                  </label>
                );
              })}
            </div>
          </div>
        </div>
      )}
    </aside>
  );
}
