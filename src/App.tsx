import { useState } from "react";
import Sidebar from "./components/layout/Sidebar";
import KeysPage from "./components/keys/KeysPage";
import DashboardPage from "./components/dashboard/DashboardPage";
import ModelsPage from "./components/models/ModelsPage";
import ProxyPage from "./components/proxy/ProxyPage";
import SettingsPage from "./components/settings/SettingsPage";
import ProvidersPage from "./components/providers/ProvidersPage";
import McpPage from "./components/mcp/McpPage";
import PromptsPage from "./components/prompts/PromptsPage";
import SpeedTestPage from "./components/speedtest/SpeedTestPage";
import AnalyticsPage from "./components/analytics/AnalyticsPage";
import "./App.css";

export type NavPage = "dashboard" | "keys" | "models" | "proxy" | "providers" | "mcp" | "prompts" | "speedtest" | "analytics" | "settings";

export default function App() {
  const [activePage, setActivePage] = useState<NavPage>("dashboard");

  const renderPage = () => {
    switch (activePage) {
      case "dashboard":  return <DashboardPage />;
      case "keys":       return <KeysPage />;
      case "models":     return <ModelsPage />;
      case "proxy":      return <ProxyPage />;
      case "providers":  return <ProvidersPage />;
      case "mcp":        return <McpPage />;
      case "prompts":    return <PromptsPage />;
      case "speedtest":  return <SpeedTestPage />;
      case "analytics":  return <AnalyticsPage />;
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
