import { api } from "../../lib/api";
import type {
  FileImportScanResult,
  IdeOrigin,
  ImportSummary,
  ImportSummaryItem,
  ScannedIdeAccount,
} from "./addAccountWizardTypes";

export function parseMetaJson(metaJson?: string | null): Record<string, unknown> {
  if (!metaJson) return {};
  try {
    const value = JSON.parse(metaJson);
    return typeof value === "object" && value ? value : {};
  } catch {
    return {};
  }
}

export function isCodexApiKeyAccount(acc: ScannedIdeAccount, origin: IdeOrigin) {
  if (origin !== "codex") return false;
  const meta = parseMetaJson(acc.meta_json);
  return (
    meta?.auth_mode === "apikey" &&
    typeof meta?.openai_api_key === "string" &&
    meta.openai_api_key.trim().length > 0
  );
}

export function basenameOfPath(path: string) {
  const normalized = path.replace(/\\/g, "/");
  const segments = normalized.split("/");
  return segments[segments.length - 1] || path;
}

export async function importScannedAccounts(
  accounts: ScannedIdeAccount[],
  fallbackOrigin: IdeOrigin,
): Promise<ImportSummary> {
  let ok = 0;
  let fail = 0;
  const successes: ImportSummaryItem[] = [];
  const failures: ImportSummaryItem[] = [];

  for (let i = 0; i < accounts.length; i++) {
    const acc = accounts[i];
    const origin = (acc.origin_platform || fallbackOrigin) as IdeOrigin;
    const hasRefresh = acc.refresh_token && acc.refresh_token.length > 0;
    const hasAccess = acc.access_token && acc.access_token.length > 0;
    const codexApiKeyAccount = isCodexApiKeyAccount(acc, origin);
    const label = acc.label?.trim() || acc.email || `${origin}#${i + 1}`;

    if (!hasRefresh && !hasAccess && !codexApiKeyAccount) {
      fail++;
      failures.push({
        label,
        origin_platform: origin,
        source_path: acc.source_path,
        reason: "缺少可导入的 access_token / refresh_token",
      });
      continue;
    }

    try {
      const meta = parseMetaJson(acc.meta_json);
      await api.ideAccounts.import([
        {
          id: `scan-${Date.now()}-${i}`,
          email: acc.email || (codexApiKeyAccount ? `codex-apikey-${i}@local` : `scan-${i}@local`),
          origin_platform: origin,
          token: {
            access_token: hasAccess ? acc.access_token! : codexApiKeyAccount ? "" : "requires_refresh",
            refresh_token: hasRefresh ? acc.refresh_token! : "missing",
            expires_in: 3600,
            token_type: "Bearer",
            updated_at: new Date().toISOString(),
          },
          status: "active",
          is_proxy_disabled: false,
          device_profile: {
            machine_id: `sys-${Math.random().toString(36).substring(2, 10)}`,
            mac_machine_id: `mac-${Math.random().toString(36).substring(2, 10)}`,
            dev_device_id: crypto.randomUUID(),
            sqm_id: `{${crypto.randomUUID()}}`,
          },
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          last_used: new Date().toISOString(),
          meta_json: Object.keys(meta).length > 0 ? JSON.stringify(meta) : null,
          label: acc.label?.trim() || null,
        },
      ]);
      ok++;
      successes.push({
        label,
        origin_platform: origin,
        source_path: acc.source_path,
      });
    } catch (e) {
      fail++;
      failures.push({
        label,
        origin_platform: origin,
        source_path: acc.source_path,
        reason: String(e),
      });
    }

    await new Promise((resolve) => setTimeout(resolve, 60));
  }

  return { ok, fail, successes, failures };
}

export function mergeFileImportSummary(
  summary: ImportSummary,
  fileResult: FileImportScanResult,
  origin: IdeOrigin,
): ImportSummary {
  return {
    ...summary,
    fail: summary.fail + fileResult.failures.length,
    failures: [
      ...summary.failures,
      ...fileResult.failures.map((item) => ({
        label: basenameOfPath(item.source_path),
        origin_platform: origin,
        source_path: item.source_path,
        reason: item.reason,
      })),
    ],
  };
}
