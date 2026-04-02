import { useState } from "react";
import Sidebar from "./components/layout/Sidebar";
import KeysPage from "./components/keys/KeysPage";
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
import DeepLinkHandler from "./components/DeepLinkHandler";
import "./App.css";

export type NavPage = "dashboard" | "keys" | "models" | "proxy" | "providers" | "mcp" | "skills" | "prompts" | "speedtest" | "analytics" | "sessions" | "settings";

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

    return () => {
      unlistenProvider.then((unlisten: () => void) => unlisten());
      unlistenRefresh.then((unlisten: () => void) => unlisten());
    };
  }, []);

  const renderPage = () => {
    switch (activePage) {
      case "dashboard":  return <DashboardPage />;
      case "keys":       return <KeysPage />;
      case "models":     return <ModelsPage />;
      case "proxy":      return <ProxyPage />;
      case "providers":  return <ProvidersPage />;
      case "mcp":        return <McpPage />;
      case "skills":     return <SkillsPage />;
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
