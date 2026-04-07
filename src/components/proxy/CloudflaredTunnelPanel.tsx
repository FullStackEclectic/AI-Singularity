import { useState, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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
    <div className={`tunnel-panel ${isActive ? 'active' : ''}`}>
      <div className="tunnel-header">
        <div className="tunnel-info">
           <h3>公网隧道互联</h3>
           <p className="text-muted">基于 Cloudflared Zero Trust，一键将本地网关跨网分享给公网设备使用。</p>
        </div>
        <button 
          className={`cyber-btn cyber-btn-sm ${isActive ? 'active danger' : ''}`}
          onClick={() => toggleMut.mutate()}
          disabled={loading || toggleMut.isPending}
        >
           {loading ? "CONNECTING..." : (isActive ? "SHUTDOWN" : "START TUNNEL")}
        </button>
      </div>

      {(isActive || loading) && (
        <div className="tunnel-output glow-text">
           {loading && !tunnelUrl && <span>Waiting for Cloudflared handshake...</span>}
           {tunnelUrl && (
             <div className="tunnel-url-box">
                <code>{tunnelUrl}</code>
                <button className="cyber-icon-btn" onClick={handleCopy}>
                  {copied ? "COPIED!" : "COPY_URL"}
                </button>
             </div>
           )}
        </div>
      )}
    </div>
  );
}
