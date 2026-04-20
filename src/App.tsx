import { Suspense, lazy, useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import Sidebar from "./components/layout/Sidebar";
import DashboardPage from "./components/dashboard/DashboardPage";
import DeepLinkHandler from "./components/DeepLinkHandler";
import FloatingAccountCardsLayer from "./components/floating/FloatingAccountCardsLayer";
import { message } from "@tauri-apps/plugin-dialog";
import { api } from "./lib/api";
import "./App.css";

export type NavPage = "dashboard" | "accounts" | "sharing" | "models" | "proxy" | "providers" | "mcp" | "skills" | "prompts" | "tools" | "tokenCalculator" | "mfa" | "wakeup" | "speedtest" | "analytics" | "report" | "logs" | "sessions" | "settings" | "security";

import { listen } from "@tauri-apps/api/event";
import { useProviderStore } from "./stores/providerStore";

const UnifiedAccountsList = lazy(() => import("./components/accounts/UnifiedAccountsList"));
const SharingPage = lazy(() => import("./components/accounts/SharingPage"));
const ModelsPage = lazy(() => import("./components/models/ModelsPage"));
const ProxyPage = lazy(() => import("./components/proxy/ProxyPage"));
const ToolSyncPage = lazy(() => import("./components/providers/ToolSyncPage"));
const McpPage = lazy(() => import("./components/mcp/McpPage"));
const SkillsPage = lazy(() => import("./components/skills/SkillsPage"));
const PromptsPage = lazy(() => import("./components/prompts/PromptsPage"));
const ToolDepotPage = lazy(() => import("./components/tools/ToolDepotPage"));
const TokenCalculatorPage = lazy(() => import("./components/tokenCalculator/TokenCalculatorPage"));
const MfaVaultPage = lazy(() => import("./components/mfa/MfaVaultPage"));
const WakeupPage = lazy(() => import("./components/wakeup/WakeupPage"));
const SpeedTestPage = lazy(() => import("./components/speedtest/SpeedTestPage"));
const AnalyticsPage = lazy(() => import("./components/analytics/AnalyticsPage"));
const WebReportPage = lazy(() => import("./components/report/WebReportPage"));
const LogsPage = lazy(() => import("./components/logs/LogsPage"));
const SessionsPage = lazy(() => import("./components/sessions/SessionsPage"));
const SettingsPage = lazy(() => import("./components/settings/SettingsPage"));
const SecurityPage = lazy(() => import("./components/security/SecurityPage"));

export default function App() {
  const [activePage, setActivePage] = useState<NavPage>("dashboard");
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlistenProvider = listen("provider_switched", () => {
      useProviderStore.getState().fetch();
    });
    const unlistenNavigate = listen<string>("navigate_to_page", (event) => {
      const target = normalizeNavPageTarget(event.payload);
      if (target) {
        setActivePage(target);
      }
    });

    const handleWindowNavigate = (event: Event) => {
      const detail = (event as CustomEvent<string>).detail;
      const target = normalizeNavPageTarget(detail);
      if (target) {
        setActivePage(target);
      }
    };
    window.addEventListener("ais:navigate", handleWindowNavigate as EventListener);

    const unlistenRefresh = listen("force_refresh_analytics", () => {
      // 触发全局事件通知 AnalyticsPage 刷新
      window.dispatchEvent(new Event("force_refresh_analytics"));
    });

    // --- 高危环境冲突体检 ---
    const checkEnv = async () => {
      try {
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

      message(`检测到外部程序修改了配置文件:\n${event.payload}\n\n已自动拉取同步！`, { kind: "info", title: "配置文件热更新" });
    });

    const unlistenDataChanged = listen<{ domain?: string; action?: string; source?: string }>("data:changed", async (event) => {
      const payload = event.payload || {};
      await queryClient.invalidateQueries();
      await useProviderStore.getState().fetch();
      window.dispatchEvent(new CustomEvent("ais-data-changed", { detail: payload }));
    });

    return () => {
      unlistenProvider.then((unlisten: () => void) => unlisten());
      unlistenNavigate.then((unlisten: () => void) => unlisten());
      unlistenRefresh.then((unlisten: () => void) => unlisten());
      unlistenWatcher.then((unlisten: () => void) => unlisten());
      unlistenDataChanged.then((unlisten: () => void) => unlisten());
      window.removeEventListener("ais:navigate", handleWindowNavigate as EventListener);
    };
  }, [queryClient]);

  const renderPage = () => {
    switch (activePage) {
      case "dashboard":  return <DashboardPage />;
      case "accounts":   return <UnifiedAccountsList />;
      case "sharing":    return <SharingPage />;
      case "models":     return <ModelsPage />;
      case "proxy":      return <ProxyPage />;
      case "providers":  return <ToolSyncPage />;
      case "mcp":        return <McpPage />;
      case "skills":     return <SkillsPage />;
      case "tools":      return <ToolDepotPage />;
      case "tokenCalculator": return <TokenCalculatorPage />;
      case "mfa":        return <MfaVaultPage />;
      case "wakeup":     return <WakeupPage />;
      case "prompts":    return <PromptsPage />;
      case "speedtest":  return <SpeedTestPage />;
      case "analytics":  return <AnalyticsPage />;
      case "report":     return <WebReportPage />;
      case "logs":       return <LogsPage />;
      case "sessions":   return <SessionsPage />;
      case "settings":   return <SettingsPage />;
      case "security":   return <SecurityPage />;
      default:           return <DashboardPage />;
    }
  };

  return (
    <div className="app-layout">
      <Sidebar activePage={activePage} onNavigate={setActivePage} />
      <main className="main-content animate-fade-in">
        <Suspense fallback={<PageLoadingState activePage={activePage} />}>
          {renderPage()}
        </Suspense>
      </main>
      <FloatingAccountCardsLayer />
      <DeepLinkHandler />
    </div>
  );
}

const ALLOWED_NAV_PAGES: NavPage[] = [
  "dashboard",
  "accounts",
  "sharing",
  "models",
  "proxy",
  "providers",
  "mcp",
  "skills",
  "prompts",
  "tools",
  "tokenCalculator",
  "mfa",
  "wakeup",
  "speedtest",
  "analytics",
  "report",
  "logs",
  "sessions",
  "settings",
  "security",
];

const NAV_PAGE_LABELS: Record<NavPage, string> = {
  dashboard: "仪表盘",
  accounts: "账号管理",
  sharing: "账号共享",
  models: "大模型目录",
  proxy: "代理与转发",
  providers: "环境配置",
  mcp: "MCP",
  skills: "Skills",
  prompts: "提示词",
  tools: "工具池",
  tokenCalculator: "TOKEN 计算器",
  mfa: "MFA 保险库",
  wakeup: "唤醒任务",
  speedtest: "测速",
  analytics: "数据分析",
  report: "网页报告",
  logs: "日志",
  sessions: "会话",
  settings: "设置",
  security: "安全",
};

function normalizeNavPageTarget(raw: unknown): NavPage | null {
  if (typeof raw !== "string") return null;
  const trimmed = raw.trim();
  if (!trimmed) return null;

  const normalized = trimmed
    .replace(/^#/, "")
    .replace(/^\//, "")
    .replace(/\?.*$/, "")
    .replace(/\/.*$/, "") as NavPage;

  if (ALLOWED_NAV_PAGES.includes(normalized)) {
    return normalized;
  }
  return null;
}

function PageLoadingState({ activePage }: { activePage: NavPage }) {
  return (
    <div className="page-loading-state">
      <div className="page-loading-spinner">⟳</div>
      <div className="page-loading-text">正在加载 {NAV_PAGE_LABELS[activePage] || activePage} 页面...</div>
    </div>
  );
}
