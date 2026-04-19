import { useMutation } from "@tanstack/react-query";
import { useState, type Dispatch, type SetStateAction } from "react";
import { api } from "../../lib/api";
import type { Platform } from "../../types";
import type { Status } from "./addAccountWizardTypes";

type UseAddAccountWizardApiKeyParams = {
  onSuccess: () => void;
  setStatus: Dispatch<SetStateAction<Status>>;
  setMessage: Dispatch<SetStateAction<string>>;
};

export function useAddAccountWizardApiKey({
  onSuccess,
  setStatus,
  setMessage,
}: UseAddAccountWizardApiKeyParams) {
  const [platform, setPlatform] = useState<Platform>("open_ai");
  const [keyName, setKeyName] = useState("");
  const [secret, setSecret] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [notes, setNotes] = useState("");

  const addKeyMut = useMutation({
    mutationFn: api.keys.add,
    onSuccess: () => {
      setStatus("success");
      setMessage("API Key 已保存！");
      setTimeout(() => {
        onSuccess();
      }, 1200);
    },
    onError: (error) => {
      setStatus("error");
      setMessage("保存失败: " + String(error));
    },
  });

  const handleSaveApiKey = () => {
    if (!keyName.trim()) {
      setStatus("error");
      setMessage("请填写标识名称");
      return;
    }
    if (!secret.trim()) {
      setStatus("error");
      setMessage("API Key 不能为空");
      return;
    }

    setStatus("loading");
    setMessage("正在保存...");
    addKeyMut.mutate({
      name: keyName.trim(),
      platform,
      secret: secret.trim(),
      base_url: platform === "custom" ? baseUrl.trim() || undefined : undefined,
      notes: notes.trim() || undefined,
    });
  };

  return {
    platform,
    keyName,
    secret,
    baseUrl,
    notes,
    setPlatform,
    setKeyName,
    setSecret,
    setBaseUrl,
    setNotes,
    handleSaveApiKey,
  };
}
