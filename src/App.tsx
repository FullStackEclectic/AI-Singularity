import { useState } from "react";
import Sidebar from "./components/layout/Sidebar";
import DashboardPage from "./components/dashboard/DashboardPage";
import ModelsPage from "./components/models/ModelsPage";
import ProxyPage from "./components/proxy/ProxyPage";
import SettingsPage from "./components/settings/SettingsPage";
import ProvidersPage from "./components/providers/ProvidersPage";
import McpPage from "./components/mcp/McpPage";
import SkillsPage from "./components/skills/SkillsPage";
import PromptsPage from "./components/prompts/PromptsPage";
import SpeedTestPage from "./components/speedtest/SpeedTestPage";
import AnalyticsPage from "./components/analytics/AnalyticsPage";
import SessionsPage from "./components/sessions/SessionsPage";
import AccountsContainerPage from "./components/accounts/AccountsContainerPage";
import ToolDepotPage from "./components/tools/ToolDepotPage";
import DeepLinkHandler from "./components/DeepLinkHandler";
import "./App.css";

export type NavPage = "dashboard" | "accounts" | "models" | "proxy" | "providers" | "mcp" | "skills" | "prompts" | "tools" | "speedtest" | "analytics" | "sessions" | "settings";

import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useProviderStore } from "./stores/providerStore";

export default function App() {
  const [activePage, setActivePage] = useState<NavPage>("dashboard");

  useEffect(() => {
    const unlistenProvider = listen("provider_switched", () => {
      useProviderStore.getState().fetch();
    });

    const unlistenRefresh = listen("force_refresh_analytics", () => {
      // 触发全局事件通知 AnalyticsPage 刷新
      window.dispatchEvent(new Event("force_refresh_analytics"));
    });

    // --- 高危环境冲突体检 ---
    const checkEnv = async () => {
      try {
        const { api } = await import("./lib/api");
        const { message } = await import("@tauri-apps/plugin-dialog");
        
        const conflicts = await Promise.all([
          api.env.checkConflicts("claude"),
          api.env.checkConflicts("openai"),
          api.env.checkConflicts("gemini")
        ]);
        
        const allConflicts = conflicts.flat();
        if (allConflicts.length > 0) {
          let msg = "发现您的系统底层存在硬编码环境变量，这将导致您的 CLI 工具强行无视本软件代理设置！\n\n请前往系统配置或 ~/.bashrc 删除以下变量：\n";
          for (const c of allConflicts) {
             msg += `- ${c.varName} \n  (污染源: ${c.sourcePath})\n`;
          }
          await message(msg, { title: "高危：底层环境冲突拦截", kind: "error" });
        }
      } catch (e) {
        console.error("Env check failed:", e);
      }
    };
    checkEnv();

    const unlistenWatcher = listen<string>("external_config_changed", async (event) => {
      console.warn("🛡️ 捕获到底层配置文件遭到外部篡改: ", event.payload);
      // 静默热更新 Zustand 缓存池中的数据，确切保证外挂改动实时映射到界面
      await useProviderStore.getState().fetch();
      
      const { message } = await import("@tauri-apps/plugin-dialog");
      message(`检测到外部程序修改了配置文件:\n${event.payload}\n\n已自动拉取同步！`, { kind: "info", title: "配置文件热更新" });
    });

    return () => {
      unlistenProvider.then((unlisten: () => void) => unlisten());
      unlistenRefresh.then((unlisten: () => void) => unlisten());
      unlistenWatcher.then((unlisten: () => void) => unlisten());
    };
  }, []);

  const renderPage = () => {
    switch (activePage) {
      case "dashboard":  return <DashboardPage />;
      case "accounts":   return <AccountsContainerPage />;
      case "models":     return <ModelsPage />;
      case "proxy":      return <ProxyPage />;
      case "providers":  return <ProvidersPage />;
      case "mcp":        return <McpPage />;
      case "skills":     return <SkillsPage />;
      case "tools":      return <ToolDepotPage />;
      case "prompts":    return <PromptsPage />;
      case "speedtest":  return <SpeedTestPage />;
      case "analytics":  return <AnalyticsPage />;
      case "sessions":   return <SessionsPage />;
      case "settings":   return <SettingsPage />;
      default:           return <ComingSoonPage name={activePage} />;
    }
  };

  return (
    <div className="app-layout">
      <Sidebar activePage={activePage} onNavigate={setActivePage} />
      <main className="main-content animate-fade-in">
        {renderPage()}
      </main>
      <DeepLinkHandler />
    </div>
  );
}

function ComingSoonPage({ name }: { name: string }) {
  return (
    <div className="empty-state" style={{ height: "100%" }}>
      <div className="empty-state-icon">🚧</div>
      <h3 style={{ fontSize: 18, color: "var(--color-text-secondary)" }}>
        {name} — 开发中
      </h3>
      <p>此功能正在紧锣密鼓开发，敬请期待</p>
    </div>
  );
}
