import { useState, useCallback } from "react";
import { message } from "@tauri-apps/plugin-dialog";
import { api } from "../lib/api";

export interface StreamCheckResult {
  status: "operational" | "degraded" | "failed";
  success: boolean;
  message: string;
  responseTimeMs?: number;
  modelUsed: string;
}

export function useStreamCheck() {
  const [checkingIds, setCheckingIds] = useState<Set<string>>(new Set());

  const checkProvider = useCallback(
    async (
      providerId: string,
      providerName: string,
    ): Promise<StreamCheckResult | null> => {
      setCheckingIds((prev) => new Set(prev).add(providerId));

      try {
        const result: StreamCheckResult = await api.providers.streamCheck(providerId);

        if (result.status === "operational") {
          await message(
            `${providerName} 流式网络正常\n耗时: ${result.responseTimeMs}ms\n测试模型: ${result.modelUsed}`,
            { title: "🟢 流式检测连通正常", kind: "info" }
          );
        } else if (result.status === "degraded") {
          await message(
            `${providerName} 响应较慢\n耗时: ${result.responseTimeMs}ms\n测试模型: ${result.modelUsed}`,
            { title: "🟡 连接缓慢", kind: "warning" }
          );
        } else {
          await message(
            `${providerName} 流式检查失败:\n${result.message}`,
            { title: "🔴 流式连通失败", kind: "error" }
          );
        }

        return result;
      } catch (e: any) {
        let msg = String(e);
        if (e && e.includes && e.includes("Other")) {
          try {
             msg = typeof e === 'string' ? e.replace(/^Other\("?|"?\)$/g, "") : JSON.stringify(e);
          // eslint-disable-next-line no-empty
          } catch(_err){}
        }
        await message(
          `${providerName} 测试遇到异常:\n${msg}`,
          { title: "❌ 系统错误", kind: "error" }
        );
        return null;
      } finally {
        setCheckingIds((prev) => {
          const next = new Set(prev);
          next.delete(providerId);
          return next;
        });
      }
    },
    [],
  );

  const isChecking = useCallback(
    (providerId: string) => checkingIds.has(providerId),
    [checkingIds],
  );

  return { checkProvider, isChecking };
}
