import { Box, Database, Server, ShieldCheck } from "lucide-react";
import type { ChannelMeta } from "./unifiedAccountsTypes";
import "./UnifiedAccountsSidebar.css";

export function UnifiedAccountsSidebar({
  activeChannelId,
  channels,
  onSelectChannel,
}: {
  activeChannelId: string;
  channels: {
    all: ChannelMeta;
    apiChs: ChannelMeta[];
    ideChs: ChannelMeta[];
  };
  onSelectChannel: (channelId: string) => void;
}) {
  return (
    <div className="unified-sidebar">
      <div className="sidebar-brand">
        <Database size={20} color="var(--accent-primary, #2563eb)" />
        <span>资产仓库</span>
      </div>

      <div className="sidebar-section">
        <div
          className={`channel-nav-item ${activeChannelId === "all" ? "active" : ""}`}
          onClick={() => onSelectChannel("all")}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <Box size={14} />
            {" "}
            全部账号
          </div>
          <span className="channel-count">{channels.all.count}</span>
        </div>
      </div>

      {channels.apiChs.length > 0 && (
        <div className="sidebar-section">
          <div className="sidebar-section-title">官方 API 渠道</div>
          {channels.apiChs.map((channel) => (
            <div
              key={channel.id}
              className={`channel-nav-item ${activeChannelId === channel.id ? "active" : ""}`}
              onClick={() => onSelectChannel(channel.id)}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <Server size={14} />
                {" "}
                {channel.label}
              </div>
              <span className="channel-count">{channel.count}</span>
            </div>
          ))}
        </div>
      )}

      {channels.ideChs.length > 0 && (
        <div className="sidebar-section">
          <div className="sidebar-section-title">IDE 沙盒池</div>
          {channels.ideChs.map((channel) => (
            <div
              key={channel.id}
              className={`channel-nav-item ${activeChannelId === channel.id ? "active" : ""}`}
              onClick={() => onSelectChannel(channel.id)}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <ShieldCheck size={14} />
                {" "}
                {channel.label}
              </div>
              <span className="channel-count">{channel.count}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
