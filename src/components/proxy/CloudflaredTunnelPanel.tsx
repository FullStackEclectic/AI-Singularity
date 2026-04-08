import { useState, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Globe, GlobeLock, Copy, CheckCircle2 } from "lucide-react";
import "./CloudflaredTunnelPanel.css";

export default function CloudflaredTunnelPanel({ localPort }: { localPort: number }) {
  const qc = useQueryClient();
  const [copied, setCopied] = useState(false);
  const [loading, setLoading] = useState(false);

  const { data: tunnelUrl } = useQuery<string | null>({
    queryKey: ["tunnel-url"],
    queryFn: () => invoke("filter_tunnel_status"),
    refetchInterval: 3000,
  });

  useEffect(() => {
    const unlisten = listen<string>("tunnel_url_ready", (e) => {
      qc.setQueryData(["tunnel-url"], e.payload);
      setLoading(false);
    });
    return () => {
      unlisten.then(f => f());
    };
  }, [qc]);

  const toggleMut = useMutation({
    mutationFn: async () => {
      if (tunnelUrl) {
         setLoading(true);
         await invoke("stop_tunnel");
         setLoading(false);
         qc.setQueryData(["tunnel-url"], null);
      } else {
         setLoading(true);
         await invoke("start_tunnel", { port: localPort });
         // state will be updated via event listener or refetch
      }
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ["tunnel-url"] })
  });

  const handleCopy = () => {
    if (tunnelUrl) {
      navigator.clipboard.writeText(tunnelUrl);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const isActive = !!tunnelUrl;

  return (
    <div className={`proxy-card ${isActive ? 'active' : ''}`} style={{ marginTop: 24, borderColor: isActive ? "var(--color-primary)" : undefined }}>
      <div className="proxy-card-header" style={{ marginBottom: 12, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {isActive ? <Globe size={18} className="text-primary" /> : <GlobeLock size={18} className="text-muted" />}
          <span>公网隧道穿透 (Cloudflared Tunnel)</span>
        </div>
        <button 
          className={`btn btn-sm ${isActive ? 'btn-danger' : 'btn-primary'}`}
          onClick={() => toggleMut.mutate()}
          disabled={loading || toggleMut.isPending}
        >
           {loading ? "握手中..." : (isActive ? "关闭连接" : "开启公网穿透")}
        </button>
      </div>
      <p style={{ margin: "0 0 16px 0", fontSize: 13, color: "var(--color-text-secondary)" }}>基于 Cloudflared Zero Trust，一键将本地网关跨网分享给公网设备使用。</p>

      {(isActive || loading) && (
        <div className="info-banner" style={{ margin: 0, padding: "12px 16px" }}>
           {loading && !tunnelUrl && <span style={{ fontSize: 13 }}>Waiting for Cloudflared handshake...</span>}
           {tunnelUrl && (
             <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <code style={{ fontSize: 14, fontFamily: "monospace", color: "var(--color-text)", fontWeight: 600 }}>{tunnelUrl}</code>
                <button className={`btn ${copied ? "btn-success" : "btn-secondary"} btn-sm`} onClick={handleCopy}>
                  {copied ? <><CheckCircle2 size={14}/> 已复制</> : <><Copy size={14}/> 复制链接</>}
                </button>
             </div>
           )}
        </div>
      )}
    </div>
  );
}
