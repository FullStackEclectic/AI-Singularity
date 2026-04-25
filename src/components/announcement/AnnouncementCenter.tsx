import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Bell, ExternalLink, RefreshCw, Sparkles, X } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "../../lib/api";
import type {
  Announcement,
  AnnouncementAction,
  AnnouncementState,
} from "../../lib/api";
import "./AnnouncementCenter.css";

const POLL_INTERVAL_MS = 5 * 60 * 1000;

type AnnouncementCenterProps = {
  locale?: string;
  onNavigate?: (target: string) => void;
};

export function AnnouncementCenter({
  locale,
  onNavigate,
}: AnnouncementCenterProps) {
  const [state, setState] = useState<AnnouncementState | null>(null);
  const [open, setOpen] = useState(false);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [popup, setPopup] = useState<Announcement | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const popupSeenRef = useRef<Set<string>>(new Set());
  const popoverRef = useRef<HTMLDivElement | null>(null);

  const load = useCallback(async (force = false) => {
    try {
      const next = force
        ? await api.announcements.refresh(locale)
        : await api.announcements.getState(locale);
      setState(next);
      const candidate = next.popupAnnouncement;
      if (candidate && !popupSeenRef.current.has(candidate.id)) {
        popupSeenRef.current.add(candidate.id);
        setPopup(candidate);
      }
    } catch {
      // intentional silence — announcement service should never block the app
    }
  }, [locale]);

  useEffect(() => {
    void load(false);
    const interval = window.setInterval(() => void load(false), POLL_INTERVAL_MS);
    return () => window.clearInterval(interval);
  }, [load]);

  useEffect(() => {
    if (!open) return;
    const handleClick = (event: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    window.addEventListener("mousedown", handleClick);
    return () => window.removeEventListener("mousedown", handleClick);
  }, [open]);

  const announcements = state?.announcements ?? [];
  const unreadIds = useMemo(() => new Set(state?.unreadIds ?? []), [state]);
  const unreadCount = unreadIds.size;
  const sorted = useMemo(
    () =>
      announcements.slice().sort((a, b) => {
        if (b.priority !== a.priority) return b.priority - a.priority;
        return (b.createdAt ?? "").localeCompare(a.createdAt ?? "");
      }),
    [announcements],
  );
  const activeAnnouncement = useMemo(
    () => sorted.find((item) => item.id === activeId) ?? null,
    [sorted, activeId],
  );

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    try {
      await load(true);
    } finally {
      setRefreshing(false);
    }
  }, [load]);

  const handleSelect = useCallback(
    async (announcement: Announcement) => {
      setActiveId(announcement.id);
      if (unreadIds.has(announcement.id)) {
        try {
          await api.announcements.markRead(announcement.id);
          setState((prev) => prev && {
            ...prev,
            unreadIds: prev.unreadIds.filter((id) => id !== announcement.id),
          });
        } catch {
          /* swallow */
        }
      }
    },
    [unreadIds],
  );

  const handleMarkAllRead = useCallback(async () => {
    try {
      await api.announcements.markAllRead(locale);
      setState((prev) => prev && { ...prev, unreadIds: [] });
    } catch {
      /* swallow */
    }
  }, [locale]);

  const handleAction = useCallback(
    async (action: AnnouncementAction | null | undefined) => {
      if (!action) return;
      const target = action.target.trim();
      if (!target) return;
      switch (action.type) {
        case "open_url":
        case "url":
          await openUrl(target).catch(() => undefined);
          break;
        case "navigate":
        case "tab":
          onNavigate?.(target);
          setOpen(false);
          setPopup(null);
          break;
        default:
          // unknown action types: try url first, then navigate
          if (target.startsWith("http://") || target.startsWith("https://")) {
            await openUrl(target).catch(() => undefined);
          } else {
            onNavigate?.(target);
            setOpen(false);
            setPopup(null);
          }
      }
    },
    [onNavigate],
  );

  return (
    <>
      <div className="announcement-bell" ref={popoverRef}>
        <button
          type="button"
          className={`announcement-bell-button ${unreadCount > 0 ? "has-unread" : ""}`}
          onClick={() => setOpen((prev) => !prev)}
          aria-label="公告"
          title="公告中心"
        >
          <Bell size={16} />
          {unreadCount > 0 && (
            <span className="announcement-badge">
              {unreadCount > 99 ? "99+" : unreadCount}
            </span>
          )}
        </button>

        {open && (
          <div className="announcement-popover">
            <div className="announcement-popover-header">
              <div className="announcement-popover-title">
                <Sparkles size={14} /> 公告中心
              </div>
              <div className="announcement-popover-actions">
                <button
                  type="button"
                  className="announcement-icon-btn"
                  onClick={handleRefresh}
                  disabled={refreshing}
                  title="强制刷新远端公告"
                >
                  <RefreshCw size={14} className={refreshing ? "spin" : ""} />
                </button>
                {unreadCount > 0 && (
                  <button
                    type="button"
                    className="announcement-text-btn"
                    onClick={handleMarkAllRead}
                  >
                    全部已读
                  </button>
                )}
              </div>
            </div>

            <div className="announcement-popover-body">
              {sorted.length === 0 ? (
                <div className="announcement-empty">暂无公告</div>
              ) : activeAnnouncement ? (
                <AnnouncementDetail
                  announcement={activeAnnouncement}
                  onBack={() => setActiveId(null)}
                  onAction={handleAction}
                />
              ) : (
                <ul className="announcement-list">
                  {sorted.map((item) => (
                    <li
                      key={item.id}
                      className={`announcement-list-item ${unreadIds.has(item.id) ? "unread" : ""}`}
                      onClick={() => void handleSelect(item)}
                    >
                      <div className="announcement-list-top">
                        <span className={`announcement-tag tag-${item.type || "info"}`}>
                          {item.type || "info"}
                        </span>
                        <span className="announcement-title">{item.title || "(无标题)"}</span>
                        {unreadIds.has(item.id) && (
                          <span className="announcement-dot" aria-hidden />
                        )}
                      </div>
                      {item.summary && (
                        <div className="announcement-summary">{item.summary}</div>
                      )}
                      {item.createdAt && (
                        <div className="announcement-time">
                          {new Date(item.createdAt).toLocaleString()}
                        </div>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        )}
      </div>

      {popup && (
        <div className="announcement-modal-mask" onClick={() => setPopup(null)}>
          <div className="announcement-modal" onClick={(e) => e.stopPropagation()}>
            <div className="announcement-modal-header">
              <span className={`announcement-tag tag-${popup.type || "info"}`}>
                {popup.type || "公告"}
              </span>
              <h3>{popup.title || "(无标题)"}</h3>
              <button
                type="button"
                className="announcement-icon-btn"
                onClick={() => {
                  setPopup(null);
                  void api.announcements.markRead(popup.id).catch(() => undefined);
                }}
                aria-label="关闭"
              >
                <X size={16} />
              </button>
            </div>
            <div className="announcement-modal-body">
              {popup.summary && (
                <div className="announcement-modal-summary">{popup.summary}</div>
              )}
              {popup.content && (
                <div className="announcement-modal-content">{popup.content}</div>
              )}
            </div>
            <div className="announcement-modal-footer">
              {popup.action && (
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={async () => {
                    await handleAction(popup.action);
                    setPopup(null);
                    void api.announcements.markRead(popup.id).catch(() => undefined);
                  }}
                >
                  {popup.action.label || "查看"}
                  <ExternalLink size={12} />
                </button>
              )}
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() => {
                  setPopup(null);
                  void api.announcements.markRead(popup.id).catch(() => undefined);
                }}
              >
                我知道了
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

function AnnouncementDetail({
  announcement,
  onBack,
  onAction,
}: {
  announcement: Announcement;
  onBack: () => void;
  onAction: (action: AnnouncementAction | null | undefined) => void;
}) {
  return (
    <div className="announcement-detail">
      <button className="announcement-text-btn" onClick={onBack}>
        ← 返回列表
      </button>
      <div className="announcement-detail-title">{announcement.title}</div>
      {announcement.summary && (
        <div className="announcement-detail-summary">{announcement.summary}</div>
      )}
      {announcement.content && (
        <div className="announcement-detail-content">{announcement.content}</div>
      )}
      {announcement.action && (
        <button
          type="button"
          className="btn btn-primary"
          onClick={() => onAction(announcement.action)}
        >
          {announcement.action.label || "查看"}
          <ExternalLink size={12} />
        </button>
      )}
      <div className="announcement-time">
        {announcement.createdAt
          ? new Date(announcement.createdAt).toLocaleString()
          : ""}
      </div>
    </div>
  );
}

export default AnnouncementCenter;
