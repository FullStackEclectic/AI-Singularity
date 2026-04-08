import type { NavPage } from "../../App";
import { useTranslation } from "react-i18next";
import "./Sidebar.css";

interface NavItem {
  id: NavPage;
  icon: string;
  label: string;
  badge?: string;
}

const NAV_GROUPS: { title?: string; items: NavItem[] }[] = [
  {
    items: [
      { id: "dashboard", icon: "⬡", label: "总览" },
    ],
  },
  {
    title: "核心网关 (Gateway)",
    items: [
      { id: "accounts", icon: "👤", label: "账号与资产库" },
      { id: "sharing", icon: "🔑", label: "分享发卡与额度" },
      { id: "proxy", icon: "↔", label: "本地路由与隧道" },
      { id: "security", icon: "🛡️", label: "哨站风控台" },
    ],
  },
  {
    title: "系统统计 (Analytics)",
    items: [
      { id: "analytics", icon: "📊", label: "用量看板" },
      { id: "sessions", icon: "💬", label: "流转日志" },
      { id: "speedtest", icon: "⚡", label: "节点测速" },
    ],
  },
  {
    title: "本地研发赋能 (Local Tools)",
    items: [
      { id: "providers", icon: "🔌", label: "终端配置" },
      { id: "mcp", icon: "🔌", label: "MCP 扩展" },
      { id: "skills", icon: "🛠️", label: "技能包" },
      { id: "tools", icon: "⏬", label: "本地兵工厂" },
      { id: "prompts", icon: "📝", label: "全局注入与 Prompt" },
    ],
  },
  {
    title: "平台",
    items: [
      { id: "models", icon: "🤖", label: "模型字典库" },
      { id: "settings", icon: "⚙", label: "全局设置" },
    ],
  },
];

interface SidebarProps {
  activePage: NavPage;
  onNavigate: (page: NavPage) => void;
}

export default function Sidebar({ activePage, onNavigate }: SidebarProps) {
  const { t } = useTranslation();

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

      {/* 导航 */}
      <nav className="sidebar-nav">
        {NAV_GROUPS.map((group, gi) => (
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
    </aside>
  );
}
