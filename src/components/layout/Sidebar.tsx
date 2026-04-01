import type { NavPage } from "../../App";
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
    title: "账号管理",
    items: [
      { id: "keys", icon: "🔑", label: "API Keys" },
    ],
  },
  {
    title: "AI 工具",
    items: [
      { id: "providers", icon: "⚡", label: "Provider" },
      { id: "mcp", icon: "🔌", label: "MCP Server" },
      { id: "prompts", icon: "📝", label: "系统配置" },
    ],
  },
  {
    title: "代理网关",
    items: [
      { id: "proxy", icon: "↔", label: "本地代理" },
    ],
  },
  {
    title: "信息",
    items: [
      { id: "speedtest", icon: "⚡", label: "延迟测速" },
      { id: "models", icon: "🤖", label: "模型目录" },
      { id: "settings", icon: "⚙", label: "设置" },
    ],
  },
];

interface SidebarProps {
  activePage: NavPage;
  onNavigate: (page: NavPage) => void;
}

export default function Sidebar({ activePage, onNavigate }: SidebarProps) {
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
              <div className="sidebar-group-title">{group.title}</div>
            )}
            {group.items.map((item) => (
              <button
                key={item.id}
                className={`sidebar-nav-item ${activePage === item.id ? "active" : ""}`}
                onClick={() => onNavigate(item.id)}
              >
                <span className="sidebar-nav-icon">{item.icon}</span>
                <span className="sidebar-nav-label">{item.label}</span>
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
