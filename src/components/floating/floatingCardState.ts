import type { FloatingAccountCard } from "../../lib/api";

export function toTimestampMs(value?: string | null): number {
  if (!value) return 0;
  const time = Date.parse(value);
  return Number.isNaN(time) ? 0 : time;
}

export function mergeFloatingCardByUpdatedAt(
  cards: FloatingAccountCard[],
  incoming: FloatingAccountCard
): FloatingAccountCard[] {
  const index = cards.findIndex((item) => item.id === incoming.id);
  if (index < 0) {
    return [...cards, incoming];
  }
  const current = cards[index];
  if (toTimestampMs(incoming.updated_at) < toTimestampMs(current.updated_at)) {
    return cards;
  }
  const next = [...cards];
  next[index] = incoming;
  return next;
}

export function sortFloatingCardsForRender(cards: FloatingAccountCard[]): FloatingAccountCard[] {
  return [...cards].sort((a, b) => {
    if (a.always_on_top !== b.always_on_top) {
      return a.always_on_top ? -1 : 1;
    }
    return toTimestampMs(b.updated_at) - toTimestampMs(a.updated_at);
  });
}

export function isFloatingCardConflictError(error: unknown): boolean {
  return String(error).includes("floating_card_conflict");
}
