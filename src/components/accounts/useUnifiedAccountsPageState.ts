import { useEffect, useState } from "react";
import { isPrivacyMode, setPrivacyMode } from "../../lib/privacyMode";
import { type AttentionReasonFilter } from "./unifiedAccountsUtils";
import {
  type AccountViewMode,
  persistAccountViewMode,
  persistTagGrouping,
  readPersistedAccountViewMode,
  readPersistedTagGrouping,
} from "./accountViewUtils";
import type { ActionMessage } from "./unifiedAccountsTypes";

export function useUnifiedAccountsPageState() {
  const [privacy, setPrivacy] = useState(isPrivacyMode);
  const [actionMessage, setActionMessage] = useState<ActionMessage | null>(null);
  const [selectedIdeIds, setSelectedIdeIds] = useState<string[]>([]);
  const [showAttentionOnly, setShowAttentionOnly] = useState(false);
  const [attentionReasonFilter, setAttentionReasonFilter] = useState<AttentionReasonFilter | null>(null);
  const [accountGroupFilter, setAccountGroupFilter] = useState<string>("all");
  const [accountViewMode, setAccountViewMode] = useState<AccountViewMode>(() => readPersistedAccountViewMode());
  const [groupByTag, setGroupByTag] = useState<boolean>(() => readPersistedTagGrouping());
  const [searchQuery, setSearchQuery] = useState("");
  const [activeChannelId, setActiveChannelId] = useState<string>("all");

  const togglePrivacy = () => {
    const next = !privacy;
    setPrivacy(next);
    setPrivacyMode(next);
  };

  const toggleIdeSelected = (id: string) => {
    setSelectedIdeIds((prev) => (prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id]));
  };

  useEffect(() => {
    if (activeChannelId.startsWith("api_")) {
      setAccountGroupFilter("all");
      setShowAttentionOnly(false);
      setAttentionReasonFilter(null);
    }
  }, [activeChannelId]);

  useEffect(() => {
    persistAccountViewMode(accountViewMode);
  }, [accountViewMode]);

  useEffect(() => {
    persistTagGrouping(groupByTag);
  }, [groupByTag]);

  return {
    accountGroupFilter,
    accountViewMode,
    actionMessage,
    activeChannelId,
    attentionReasonFilter,
    groupByTag,
    privacy,
    searchQuery,
    selectedIdeIds,
    setAccountGroupFilter,
    setAccountViewMode,
    setActionMessage,
    setActiveChannelId,
    setAttentionReasonFilter,
    setGroupByTag,
    setSearchQuery,
    setSelectedIdeIds,
    setShowAttentionOnly,
    showAttentionOnly,
    toggleIdeSelected,
    togglePrivacy,
  };
}

export type UnifiedAccountsPageState = ReturnType<typeof useUnifiedAccountsPageState>;
