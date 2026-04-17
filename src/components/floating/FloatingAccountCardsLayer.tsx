import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { api, type CurrentAccountSnapshot, type FloatingAccountCard } from "../../lib/api";
import type { IdeAccount } from "../../types";
import {
  isFloatingCardConflictError,
  mergeFloatingCardByUpdatedAt,
  sortFloatingCardsForRender,
} from "./floatingCardState";
import "./FloatingAccountCardsLayer.css";

const MIN_WIDTH = 260;
const MIN_HEIGHT = 120;

type CardInteraction =
  | {
      mode: "drag";
      cardId: string;
      startX: number;
      startY: number;
      originX: number;
      originY: number;
      expectedUpdatedAt: string;
    }
  | {
      mode: "resize";
      cardId: string;
      startX: number;
      startY: number;
      originWidth: number;
      originHeight: number;
      expectedUpdatedAt: string;
    };

interface CodexInstanceLite {
  id: string;
  name: string;
}

function normalizePlatform(value: string | undefined | null): string {
  return String(value || "").trim().toLowerCase();
}

export default function FloatingAccountCardsLayer() {
  const [cards, setCards] = useState<FloatingAccountCard[]>([]);
  const [snapshots, setSnapshots] = useState<CurrentAccountSnapshot[]>([]);
  const [ideAccounts, setIdeAccounts] = useState<IdeAccount[]>([]);
  const [codexInstances, setCodexInstances] = useState<CodexInstanceLite[]>([]);
  const [interaction, setInteraction] = useState<CardInteraction | null>(null);
  const [switchingKey, setSwitchingKey] = useState<string | null>(null);

  const cardsRef = useRef<FloatingAccountCard[]>(cards);
  useEffect(() => {
    cardsRef.current = cards;
  }, [cards]);

  const loadAll = useCallback(async () => {
    try {
      const [cardList, snapshotList, accounts, listInstances, defaultInstance] = await Promise.all([
        api.floatingCards.list().catch(() => []),
        api.providerCurrent.listSnapshots().catch(() => []),
        api.ideAccounts.list().catch(() => []),
        invoke<CodexInstanceLite[]>("list_codex_instances").catch(() => []),
        invoke<CodexInstanceLite>("get_default_codex_instance").catch(() => null),
      ]);
      const nextInstances = [
        ...(defaultInstance ? [defaultInstance] : []),
        ...listInstances,
      ];
      setCards(cardList);
      setSnapshots(snapshotList);
      setIdeAccounts(accounts);
      setCodexInstances(nextInstances);
    } catch (error) {
      console.warn("Failed to load floating cards:", error);
    }
  }, []);

  const applyPatch = useCallback(
    async (
      cardId: string,
      patch: Parameters<typeof api.floatingCards.update>[1],
      expectedUpdatedAt: string
    ) => {
      try {
        const updated = await api.floatingCards.update(cardId, patch, expectedUpdatedAt);
        setCards((prev) => mergeFloatingCardByUpdatedAt(prev, updated));
      } catch (error) {
        if (isFloatingCardConflictError(error)) {
          await loadAll();
          return;
        }
        console.warn("Failed to update floating card:", error);
      }
    },
    [loadAll]
  );

  useEffect(() => {
    loadAll();
    const listeners = [
      listen("floating.card.created", () => {
        void loadAll();
      }),
      listen("floating.card.updated", () => {
        void loadAll();
      }),
      listen("floating.card.position_changed", () => {
        void loadAll();
      }),
      listen("floating.card.visibility_changed", () => {
        void loadAll();
      }),
      listen("floating.card.deleted", () => {
        void loadAll();
      }),
      listen("floating.account.changed", () => {
        void loadAll();
      }),
      listen<{ domain?: string }>("data:changed", (event) => {
        const domain = normalizePlatform(event.payload?.domain);
        if (domain === "ide_accounts" || domain === "floating_cards" || domain === "provider_current") {
          void loadAll();
        }
      }),
    ];
    return () => {
      for (const item of listeners) {
        item.then((unlisten) => unlisten());
      }
    };
  }, [loadAll]);

  useEffect(() => {
    if (!interaction) return;

    const handleMouseMove = (event: MouseEvent) => {
      setCards((prev) =>
        prev.map((card) => {
          if (card.id !== interaction.cardId) return card;
          if (interaction.mode === "drag") {
            const deltaX = event.clientX - interaction.startX;
            const deltaY = event.clientY - interaction.startY;
            return {
              ...card,
              x: Math.max(0, interaction.originX + deltaX),
              y: Math.max(0, interaction.originY + deltaY),
            };
          }
          const deltaW = event.clientX - interaction.startX;
          const deltaH = event.clientY - interaction.startY;
          return {
            ...card,
            width: Math.max(MIN_WIDTH, interaction.originWidth + deltaW),
            height: Math.max(MIN_HEIGHT, interaction.originHeight + deltaH),
          };
        })
      );
    };

    const handleMouseUp = () => {
      const card = cardsRef.current.find((item) => item.id === interaction.cardId);
      if (card) {
        if (interaction.mode === "drag") {
          void applyPatch(
            card.id,
            { x: card.x, y: card.y },
            interaction.expectedUpdatedAt
          );
        } else {
          void applyPatch(
            card.id,
            { width: card.width, height: card.height },
            interaction.expectedUpdatedAt
          );
        }
      }
      setInteraction(null);
    };

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [applyPatch, interaction]);

  const snapshotByPlatform = useMemo(() => {
    const map = new Map<string, CurrentAccountSnapshot>();
    for (const snapshot of snapshots) {
      const key = normalizePlatform(snapshot.platform);
      if (key) {
        map.set(key, snapshot);
      }
    }
    return map;
  }, [snapshots]);

  const accountsByPlatform = useMemo(() => {
    const map = new Map<string, IdeAccount[]>();
    for (const account of ideAccounts) {
      const key = normalizePlatform(account.origin_platform);
      if (!key) continue;
      const bucket = map.get(key) || [];
      bucket.push(account);
      map.set(key, bucket);
    }
    return map;
  }, [ideAccounts]);

  const codexInstanceNameById = useMemo(() => {
    const map = new Map<string, string>();
    for (const item of codexInstances) {
      map.set(item.id, item.name);
    }
    return map;
  }, [codexInstances]);

  const visibleCards = useMemo(
    () => sortFloatingCardsForRender(cards.filter((card) => card.visible)),
    [cards]
  );

  const handleSwitchAccount = async (platform: string, accountId: string) => {
    if (!accountId) return;
    const key = `${platform}:${accountId}`;
    setSwitchingKey(key);
    try {
      await api.ideAccounts.forceInject(accountId);
      await loadAll();
    } catch (error) {
      console.warn("Failed to switch account from floating card:", error);
    } finally {
      setSwitchingKey(null);
    }
  };

  if (visibleCards.length === 0) {
    return null;
  }

  return (
    <div className="floating-cards-layer">
      {visibleCards.map((card) => {
        const platforms = (card.bound_platforms || []).length > 0 ? card.bound_platforms : ["codex", "gemini"];
        const instanceName = card.instance_id ? codexInstanceNameById.get(card.instance_id) : null;
        const instanceMissing = card.scope === "instance" && !!card.instance_id && !instanceName;
        return (
          <section
            key={card.id}
            className={`floating-card ${card.collapsed ? "collapsed" : ""} ${card.always_on_top ? "always-on-top" : ""}`}
            style={{
              left: `${card.x}px`,
              top: `${card.y}px`,
              width: `${card.width}px`,
              height: card.collapsed ? "auto" : `${card.height}px`,
              zIndex: card.always_on_top ? 1800 : 1500,
            }}
          >
            <header
              className="floating-card-header"
              onMouseDown={(event) => {
                const target = event.target as HTMLElement;
                if (target.closest("button,select,input,.floating-card-resize-handle")) {
                  return;
                }
                event.preventDefault();
                setInteraction({
                  mode: "drag",
                  cardId: card.id,
                  startX: event.clientX,
                  startY: event.clientY,
                  originX: card.x,
                  originY: card.y,
                  expectedUpdatedAt: card.updated_at,
                });
              }}
            >
              <div className="floating-card-title-wrap">
                <div className="floating-card-title">{card.title}</div>
                <div className="floating-card-subtitle">
                  {card.scope === "instance"
                    ? `实例绑定 · ${instanceName || card.instance_id || "未知实例"}`
                    : "全局浮窗"}
                </div>
              </div>
              <div className="floating-card-header-actions">
                <button
                  type="button"
                  className="btn btn-ghost btn-xs"
                  onClick={() =>
                    void applyPatch(
                      card.id,
                      { always_on_top: !card.always_on_top },
                      card.updated_at
                    )
                  }
                >
                  {card.always_on_top ? "取消置顶" : "置顶"}
                </button>
                <button
                  type="button"
                  className="btn btn-ghost btn-xs"
                  onClick={() =>
                    void applyPatch(
                      card.id,
                      { collapsed: !card.collapsed },
                      card.updated_at
                    )
                  }
                >
                  {card.collapsed ? "展开" : "折叠"}
                </button>
                <button
                  type="button"
                  className="btn btn-ghost btn-xs"
                  onClick={() =>
                    void applyPatch(
                      card.id,
                      { visible: false },
                      card.updated_at
                    )
                  }
                >
                  隐藏
                </button>
              </div>
            </header>

            {!card.collapsed && (
              <div className="floating-card-body">
                {instanceMissing && (
                  <div className="floating-card-warning">
                    绑定实例已不存在，后端会在下一次同步时自动降级为全局浮窗。
                  </div>
                )}
                {platforms.map((platform) => {
                  const normalizedPlatform = normalizePlatform(platform);
                  const snapshot = snapshotByPlatform.get(normalizedPlatform);
                  const platformAccounts = accountsByPlatform.get(normalizedPlatform) || [];
                  const hasCurrentSnapshotOption = !!snapshot?.account_id &&
                    platformAccounts.some((account) => account.id === snapshot.account_id);
                  return (
                    <div key={`${card.id}-${normalizedPlatform}`} className="floating-card-platform-row">
                      <div className="floating-card-platform-meta">
                        <div className="floating-card-platform-name">{normalizedPlatform || "unknown"}</div>
                        <div className="floating-card-platform-account">
                          {snapshot?.label || snapshot?.email || "未解析到当前账号"}
                        </div>
                      </div>
                      <select
                        className="floating-card-switch-select"
                        value={snapshot?.account_id || ""}
                        onChange={(event) =>
                          void handleSwitchAccount(normalizedPlatform, event.target.value)
                        }
                        disabled={platformAccounts.length === 0 || switchingKey !== null}
                      >
                        {!snapshot?.account_id && <option value="">当前未解析</option>}
                        {!!snapshot?.account_id && !hasCurrentSnapshotOption && (
                          <option value={snapshot.account_id}>
                            {snapshot.label || snapshot.email || snapshot.account_id}
                          </option>
                        )}
                        {platformAccounts.map((account) => (
                          <option key={account.id} value={account.id}>
                            {(account.label?.trim() || account.email)}
                          </option>
                        ))}
                      </select>
                    </div>
                  );
                })}
              </div>
            )}

            {!card.collapsed && (
              <div
                className="floating-card-resize-handle"
                onMouseDown={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  setInteraction({
                    mode: "resize",
                    cardId: card.id,
                    startX: event.clientX,
                    startY: event.clientY,
                    originWidth: card.width,
                    originHeight: card.height,
                    expectedUpdatedAt: card.updated_at,
                  });
                }}
              />
            )}
          </section>
        );
      })}
    </div>
  );
}
