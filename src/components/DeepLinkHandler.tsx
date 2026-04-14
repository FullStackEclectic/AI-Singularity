import { useEffect, useState } from "react";
import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { useProviderStore } from "../stores/providerStore";
import { useMcpStore } from "../stores/mcpStore";
import { useQueryClient } from "@tanstack/react-query";

interface PendingImport {
  type: "provider" | "mcp" | "unknown";
  data: any;
}

export default function DeepLinkHandler() {
  const [pending, setPending] = useState<PendingImport | null>(null);
  const [message, setMessage] = useState<string>("");
  const { add: addProvider } = useProviderStore();
  const { add: addMcp } = useMcpStore();
  const qc = useQueryClient();

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupDeepLink = async () => {
      try {
        unlisten = await onOpenUrl((urls) => {
          console.log("Deep link received:", urls);
          for (const url of urls) {
            handleDeepLink(url);
          }
        });
      } catch (err) {
        console.warn("Failed to setup deep link listener:", err);
      }
    };

    setupDeepLink();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const handleDeepLink = (urlStr: string) => {
    try {
      // Protocol is typically ais:// or ais://provider?data=...
      const url = new URL(urlStr);
      if (url.protocol !== "ais:") return;

      const typeMatch = url.hostname || url.pathname.replace(/^\/\//, ""); // ais://provider -> hostname=provider OR pathname=//provider
      const type = typeMatch.replace(/\//g, "").toLowerCase();

      const dataBase64 = url.searchParams.get("data");
      if (!dataBase64) return;

      const jsonStr = decodeURIComponent(atob(dataBase64));
      const parsed = JSON.parse(jsonStr);

      if (type === "provider" || type === "mcp") {
        setPending({ type, data: parsed });
      }
    } catch (e) {
      console.error("Deep link parsing failed:", e);
      setMessage("深链数据解析失败 (Deep link format invalid)");
    }
  };

  const confirmImport = async () => {
    if (!pending) return;
    try {
      if (pending.type === "provider") {
        await addProvider({
          id: pending.data.id || crypto.randomUUID(),
          name: pending.data.name || "Imported Provider",
          platform: pending.data.platform || "custom",
          api_key_id: pending.data.api_key_id ?? null,
          base_url: pending.data.base_url ?? null,
          model_name: pending.data.model_name || "",
          is_active: Boolean(pending.data.is_active),
          tool_targets: pending.data.tool_targets ?? JSON.stringify(["claude_code"]),
          website_url: pending.data.website_url ?? null,
          api_key_url: pending.data.api_key_url ?? null,
          notes: pending.data.notes ?? null,
          extra_config: pending.data.extra_config ?? "{}",
        });
        qc.invalidateQueries({ queryKey: ["providers"] });
      } else if (pending.type === "mcp") {
        const now = new Date().toISOString();
        await addMcp({
          id: pending.data.id || crypto.randomUUID(),
          name: pending.data.name || "Imported MCP",
          command: pending.data.command || "",
          args: pending.data.args ?? "[]",
          env: pending.data.env ?? "{}",
          description: pending.data.description ?? null,
          is_active: Boolean(pending.data.is_active),
          tool_targets: pending.data.tool_targets ?? null,
          created_at: pending.data.created_at || now,
          updated_at: now,
        });
        qc.invalidateQueries({ queryKey: ["mcps"] });
      }
      setPending(null);
    } catch (e) {
      console.error("Import failed:", e);
      setMessage("导入失败: " + String(e));
    }
  };

  if (!pending && !message) return null;

  return (
    <div className="modal-overlay" style={{ zIndex: 9999 }}>
      <div className="modal">
        <div className="modal-header">
          <h2>{pending ? `📥 确认导入 ${pending.type === "provider" ? "API 提供商" : "MCP 服务"}` : "深链提示"}</h2>
          <button className="btn btn-icon" onClick={() => { setPending(null); setMessage(""); }}>✕</button>
        </div>
        <div className="modal-body">
          {message && (
            <div className="alert alert-warning" style={{ marginBottom: 16 }}>
              {message}
            </div>
          )}
          {pending && (
            <>
              <p>您收到了一个分享链接，是否确认将其导入您的 AI Singularity 控制中心？</p>
              <div style={{ background: "var(--bg-inset)", padding: 12, borderRadius: 8, marginTop: 16 }}>
                <h4 style={{ margin: "0 0 8px 0" }}>名称: {pending.data.name || "Unknown"}</h4>
                {pending.type === "provider" && (
                  <pre className="font-mono text-muted" style={{ margin: 0, fontSize: 13, whiteSpace: "pre-wrap" }}>
                    Platform: {pending.data.platform}{"\n"}
                    BaseURL: {pending.data.base_url || "官方源"}
                  </pre>
                )}
                {pending.type === "mcp" && (
                  <pre className="font-mono text-muted" style={{ margin: 0, fontSize: 13, whiteSpace: "pre-wrap" }}>
                    Command: {pending.data.command}{"\n"}
                    Args: {pending.data.args}
                  </pre>
                )}
              </div>
              <div className="alert alert-warning" style={{ marginTop: 16 }}>
                <span style={{ marginRight: 8 }}>⚠️</span> 注意：未知来源的深链接可能覆写您的现有配置，请只确认您信任的导入。
              </div>
            </>
          )}
        </div>
        <div className="modal-footer">
          <button className="btn btn-ghost" onClick={() => { setPending(null); setMessage(""); }}>关闭</button>
          {pending && <button className="btn btn-primary" onClick={confirmImport}>安全导入并保存</button>}
        </div>
      </div>
    </div>
  );
}
