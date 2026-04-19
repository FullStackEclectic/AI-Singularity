import { useEffect, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { AccountViewMode } from "./accountViewUtils";
import type { AccountRenderRow } from "./unifiedAccountsTypes";

type UseUnifiedAccountsVirtualizerParams = {
  accountViewMode: AccountViewMode;
  activeChannelId: string;
  groupByTag: boolean;
  groupedRenderRows: AccountRenderRow[];
};

export function useUnifiedAccountsVirtualizer({
  accountViewMode,
  activeChannelId,
  groupByTag,
  groupedRenderRows,
}: UseUnifiedAccountsVirtualizerParams) {
  const parentRef = useRef<HTMLDivElement>(null);
  const itemRowHeight = accountViewMode === "compact" ? 42 : 52;

  const rowVirtualizer = useVirtualizer({
    count: accountViewMode === "grid" ? 0 : groupedRenderRows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: (index) => {
      const row = groupedRenderRows[index];
      if (!row) return itemRowHeight;
      return row.type === "group" ? 36 : itemRowHeight;
    },
    overscan: accountViewMode === "compact" ? 14 : 10,
  });

  useEffect(() => {
    rowVirtualizer.measure();
  }, [groupedRenderRows, itemRowHeight, rowVirtualizer]);

  useEffect(() => {
    const scrollHost = parentRef.current;
    if (!scrollHost) return;
    scrollHost.scrollTop = 0;
    rowVirtualizer.scrollToOffset(0);
  }, [accountViewMode, activeChannelId, groupByTag, rowVirtualizer]);

  return {
    parentRef,
    rowVirtualizer,
  };
}

export type UnifiedAccountsVirtualizerState = ReturnType<typeof useUnifiedAccountsVirtualizer>;
