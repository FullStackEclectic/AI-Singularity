import { useEffect, useState } from "react";
import {
  api,
  type CurrentAccountSnapshot,
  type FloatingAccountCard,
  type GeminiInstanceRecord,
  type LinuxReleaseInfo,
  type OAuthEnvStatusItem,
  type SkillStorageInfo,
  type UpdateRuntimeInfo,
  type UpdateSettings,
  type WebReportStatus,
  type WebSocketStatus,
} from "../../lib/api";
import type { IdeAccount } from "../../types";

export function useSettingsRuntimeData() {
  const [runtimeLoading, setRuntimeLoading] = useState(true);
  const [skillStorage, setSkillStorage] = useState<SkillStorageInfo | null>(null);
  const [oauthEnvStatus, setOauthEnvStatus] = useState<OAuthEnvStatusItem[]>([]);
  const [ideAccounts, setIdeAccounts] = useState<IdeAccount[]>([]);
  const [currentSnapshots, setCurrentSnapshots] = useState<CurrentAccountSnapshot[]>([]);
  const [floatingCards, setFloatingCards] = useState<FloatingAccountCard[]>([]);
  const [currentGeminiAccountId, setCurrentGeminiAccountId] = useState<string | null>(null);
  const [geminiInstances, setGeminiInstances] = useState<GeminiInstanceRecord[]>([]);
  const [defaultGeminiInstance, setDefaultGeminiInstance] = useState<GeminiInstanceRecord | null>(null);
  const [updateRuntimeInfo, setUpdateRuntimeInfo] = useState<UpdateRuntimeInfo | null>(null);
  const [updateSettings, setUpdateSettings] = useState<UpdateSettings | null>(null);
  const [websocketStatus, setWebsocketStatus] = useState<WebSocketStatus | null>(null);
  const [webReportStatus, setWebReportStatus] = useState<WebReportStatus | null>(null);
  const [linuxReleaseInfo, setLinuxReleaseInfo] = useState<LinuxReleaseInfo | null>(null);

  useEffect(() => {
    let cancelled = false;

    const loadRuntimeInfo = async () => {
      setRuntimeLoading(true);
      try {
        const [
          storageInfo,
          oauthInfo,
          ideAccountsList,
          snapshots,
          floatingCardList,
          instanceList,
          defaultInstance,
        ] = await Promise.all([
          api.skills.getStorageInfo(),
          api.oauth.getEnvStatus(),
          api.ideAccounts.list(),
          api.providerCurrent.listSnapshots(),
          api.floatingCards.list().catch(() => []),
          api.geminiInstances.list(),
          api.geminiInstances.getDefault(),
        ]);
        const [runtimeInfo, savedUpdateSettings, wsStatus, reportStatus] = await Promise.all([
          api.update.getRuntimeInfo(),
          api.update.getSettings(),
          api.websocket.getStatus(),
          api.webReport.getStatus().catch(() => null),
        ]);

        if (cancelled) {
          return;
        }

        setSkillStorage(storageInfo);
        setOauthEnvStatus(oauthInfo);
        setIdeAccounts(ideAccountsList);
        setCurrentSnapshots(snapshots);
        setFloatingCards(floatingCardList);
        setCurrentGeminiAccountId(
          snapshots.find((item) => item.platform === "gemini")?.account_id ?? null,
        );
        setGeminiInstances(instanceList);
        setDefaultGeminiInstance(defaultInstance);
        setUpdateRuntimeInfo(runtimeInfo);
        setUpdateSettings(savedUpdateSettings);
        setWebsocketStatus(wsStatus);
        setWebReportStatus(reportStatus);

        if (runtimeInfo.platform === "linux") {
          api.update.getLinuxReleaseInfo().then((releaseInfo) => {
            if (!cancelled) {
              setLinuxReleaseInfo(releaseInfo);
            }
          }).catch((error) => {
            console.warn("Failed to load Linux release info:", error);
          });
        }
      } catch (error) {
        if (!cancelled) {
          console.error("Failed to load runtime info:", error);
        }
      } finally {
        if (!cancelled) {
          setRuntimeLoading(false);
        }
      }
    };

    void loadRuntimeInfo();

    return () => {
      cancelled = true;
    };
  }, []);

  return {
    runtimeLoading,
    skillStorage,
    oauthEnvStatus,
    ideAccounts,
    currentSnapshots,
    floatingCards,
    currentGeminiAccountId,
    geminiInstances,
    defaultGeminiInstance,
    updateRuntimeInfo,
    updateSettings,
    websocketStatus,
    webReportStatus,
    linuxReleaseInfo,
    setIdeAccounts,
    setCurrentSnapshots,
    setFloatingCards,
    setCurrentGeminiAccountId,
    setGeminiInstances,
    setDefaultGeminiInstance,
    setUpdateSettings,
  };
}

export type SettingsRuntimeDataState = ReturnType<typeof useSettingsRuntimeData>;
