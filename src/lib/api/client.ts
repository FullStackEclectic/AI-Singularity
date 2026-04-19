import { alerts, env, proxy, security, skills, speedtest, stats, tools } from "./system";
import { balance, keys, models, providers, tokenCalculator } from "./finance";
import { analytics, floatingCards, ideAccounts, mcp, prompts, providerCurrent, userTokens } from "./integration";
import { announcements, logs, update, webReport, websocket } from "./runtime";
import { geminiInstances, oauth, wakeup, webdav } from "./automation";

export const api = {
  stats,
  env,
  proxy,
  security,
  keys,
  balance,
  models,
  tokenCalculator,
  providers,
  providerCurrent,
  floatingCards,
  mcp,
  prompts,
  alerts,
  speedtest,
  ideAccounts,
  tools,
  userTokens,
  analytics,
  logs,
  update,
  websocket,
  webReport,
  announcements,
  wakeup,
  oauth,
  geminiInstances,
  webdav,
  skills,
};
