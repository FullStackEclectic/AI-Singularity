import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { api, type CurrentAccountSnapshot } from "../../lib/api";
import type { AccountGroup } from "../../types";

export function useUnifiedAccountsQueries() {
  const { data: rawKeys = [], isLoading: keysLoading } = useQuery({
    queryKey: ["keys"],
    queryFn: api.keys.list,
  });
  const keys = useMemo(
    () => rawKeys.filter((key) => !key.name.endsWith("(Auto Key)")),
    [rawKeys]
  );

  const { data: balances = [] } = useQuery({
    queryKey: ["balances"],
    queryFn: api.balance.listAll,
    staleTime: 1000 * 60 * 5,
  });
  const { data: ideAccs = [], isLoading: ideLoading } = useQuery({
    queryKey: ["ideAccounts"],
    queryFn: api.ideAccounts.list,
  });
  const { data: accountGroups = [] } = useQuery<AccountGroup[]>({
    queryKey: ["accountGroups"],
    queryFn: api.ideAccounts.listGroups,
  });
  const { data: currentSnapshots = [] } = useQuery<CurrentAccountSnapshot[]>({
    queryKey: ["providerCurrentSnapshots"],
    queryFn: api.providerCurrent.listSnapshots,
    staleTime: 1000 * 15,
  });

  const currentIdeAccountIds = useMemo(
    () =>
      Object.fromEntries(
        currentSnapshots.map((item) => [item.platform, item.account_id ?? null])
      ) as Record<string, string | null>,
    [currentSnapshots]
  );

  return {
    accountGroups,
    balances,
    currentIdeAccountIds,
    ideAccs,
    isLoading: keysLoading || ideLoading,
    keys,
  };
}

export type UnifiedAccountsQueriesState = ReturnType<typeof useUnifiedAccountsQueries>;
