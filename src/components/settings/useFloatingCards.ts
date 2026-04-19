import type { Dispatch, SetStateAction } from "react";
import { api, type FloatingAccountCard } from "../../lib/api";

type UseFloatingCardsParams = {
  setFloatingCards: Dispatch<SetStateAction<FloatingAccountCard[]>>;
  setFloatingCardMsg: Dispatch<SetStateAction<string>>;
};

export function useFloatingCards({
  setFloatingCards,
  setFloatingCardMsg,
}: UseFloatingCardsParams) {
  const reloadFloatingCards = async () => {
    const cards = await api.floatingCards.list().catch(() => []);
    setFloatingCards(cards);
  };

  const handleFloatingCardError = async (
    error: unknown,
    fallback = "浮窗操作失败"
  ) => {
    if (String(error).includes("floating_card_conflict")) {
      setFloatingCardMsg("浮窗已在其他窗口更新，已刷新到最新状态");
      await reloadFloatingCards();
      return;
    }
    setFloatingCardMsg(`${fallback}: ${error}`);
  };

  const handleCreateGlobalFloatingCard = async () => {
    try {
      await api.floatingCards.create({
        scope: "global",
        title: "全局账号浮窗",
        bound_platforms: ["codex", "gemini"],
        window_label: "main",
      });
      setFloatingCardMsg("已创建全局浮窗");
      await reloadFloatingCards();
    } catch (error) {
      await handleFloatingCardError(error, "创建全局浮窗失败");
    }
  };

  const handleToggleFloatingCardVisible = async (card: FloatingAccountCard) => {
    try {
      await api.floatingCards.update(card.id, { visible: !card.visible }, card.updated_at);
      await reloadFloatingCards();
    } catch (error) {
      await handleFloatingCardError(error, "更新浮窗可见状态失败");
    }
  };

  const handleToggleFloatingCardTop = async (card: FloatingAccountCard) => {
    try {
      await api.floatingCards.update(
        card.id,
        { always_on_top: !card.always_on_top },
        card.updated_at
      );
      await reloadFloatingCards();
    } catch (error) {
      await handleFloatingCardError(error, "更新浮窗置顶状态失败");
    }
  };

  const handleDeleteFloatingCard = async (card: FloatingAccountCard) => {
    try {
      await api.floatingCards.delete(card.id);
      setFloatingCardMsg("浮窗已删除");
      await reloadFloatingCards();
    } catch (error) {
      await handleFloatingCardError(error, "删除浮窗失败");
    }
  };

  return {
    reloadFloatingCards,
    handleCreateGlobalFloatingCard,
    handleToggleFloatingCardVisible,
    handleToggleFloatingCardTop,
    handleDeleteFloatingCard,
  };
}

export type FloatingCardsState = ReturnType<typeof useFloatingCards>;
