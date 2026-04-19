import { useMemo, useState, type FormEvent } from "react";
import { useProviderStore } from "../../stores/providerStore";
import type { ProviderConfig, Platform, ToolTarget } from "../../types";
import {
  TOOL_TARGET_LABELS,
  parseToolTargets,
} from "../../types";
import {
  filterPresetsByTool,
  groupPresetsByToolAndCategory,
  type ProviderPreset,
  type ProviderCategory,
} from "../../data/providerPresets";
import { api } from "../../lib/api";
import type { ProviderExtraConfig } from "./ProviderAdvancedConfig";
import { ProviderModalAdvancedTab } from "./ProviderModalAdvancedTab";
import { ProviderModalBasicTab } from "./ProviderModalBasicTab";
import { ProviderPresetStep } from "./ProviderPresetStep";
import type { ProviderFormState } from "./providerModalFormTypes";
import "./ProviderModal.css";

export function ProviderModal({
  initialProvider,
  onClose,
  onSuccess,
  fixedTool,
}: {
  initialProvider?: ProviderConfig;
  onClose: () => void;
  onSuccess: () => void;
  fixedTool?: ToolTarget;
}) {
  const { add, update } = useProviderStore();

  const [step, setStep] = useState<"preset" | "form">(initialProvider ? "form" : "preset");
  const [presetSearch, setPresetSearch] = useState("");
  const [selectedCategory, setSelectedCategory] = useState<ProviderCategory | "all">("all");
  const [activeTab, setActiveTab] = useState<"basic" | "advanced">("basic");

  const isEditing = !!initialProvider;

  const [form, setForm] = useState<ProviderFormState>(() => {
    if (initialProvider) {
      return {
        name: initialProvider.name,
        platform: initialProvider.platform,
        base_url: initialProvider.base_url ?? "",
        model_name: initialProvider.model_name,
        api_key_value: "",
        tool_targets: parseToolTargets(initialProvider),
        website_url: initialProvider.website_url ?? "",
        api_key_url: initialProvider.api_key_url ?? "",
        notes: initialProvider.notes ?? "",
        extra_config: initialProvider.extra_config ?? "{}",
      };
    }

    return {
      name: "",
      platform: "custom",
      base_url: "",
      model_name: "",
      api_key_value: "",
      tool_targets: fixedTool ? [fixedTool] : ["claude_code"],
      website_url: "",
      api_key_url: "",
      notes: "",
      extra_config: "{}",
    };
  });

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState("");
  const [isFetchingModels, setIsFetchingModels] = useState(false);
  const [modelOptions, setModelOptions] = useState<string[]>([]);
  const [modelFetchError, setModelFetchError] = useState("");

  const groupedPresets = useMemo(() => groupPresetsByToolAndCategory(fixedTool), [fixedTool]);
  const filteredPresets = useMemo(() => {
    const basePresets = selectedCategory === "all"
      ? filterPresetsByTool(fixedTool)
      : groupedPresets[selectedCategory] ?? [];

    if (!presetSearch.trim()) {
      return basePresets;
    }

    const query = presetSearch.toLowerCase();
    return basePresets.filter((preset) =>
      preset.name.toLowerCase().includes(query)
      || (preset.defaultBaseUrl?.toLowerCase().includes(query) ?? false),
    );
  }, [fixedTool, groupedPresets, presetSearch, selectedCategory]);

  const applyPreset = (preset: ProviderPreset) => {
    setForm((current) => ({
      ...current,
      name: preset.name,
      platform: preset.platform as Platform,
      base_url: preset.defaultBaseUrl ?? "",
      model_name: preset.defaultModel ?? "",
      website_url: preset.websiteUrl ?? "",
      api_key_url: preset.apiKeyUrl ?? "",
      notes: preset.notes ?? "",
      extra_config: preset.settingsConfig ? JSON.stringify(preset.settingsConfig, null, 2) : "{}",
    }));
    setStep("form");
  };

  let advancedCfg: ProviderExtraConfig = {};
  try {
    advancedCfg = JSON.parse(form.extra_config || "{}");
  } catch {}

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    if (!form.name.trim()) {
      setError("名称不能为空");
      return;
    }
    if (form.tool_targets.length === 0) {
      setError("请至少选择一个同步目标");
      return;
    }

    setIsSubmitting(true);
    setError("");
    try {
      let finalApiKeyId = initialProvider?.api_key_id ?? undefined;

      if (form.api_key_value.trim()) {
        const newKey = await api.keys.add({
          name: `${form.name.trim()} (Auto Key)`,
          platform: form.platform,
          secret: form.api_key_value.trim(),
          base_url: form.base_url.trim() || undefined,
        });
        finalApiKeyId = newKey.id;
      }

      const payload: any = {
        id: initialProvider?.id || crypto.randomUUID(),
        name: form.name.trim(),
        platform: form.platform,
        api_key_id: finalApiKeyId,
        base_url: form.base_url.trim() || null,
        model_name: form.model_name.trim(),
        is_active: initialProvider?.is_active ?? false,
        tool_targets: JSON.stringify(form.tool_targets),
        website_url: form.website_url.trim() || null,
        api_key_url: form.api_key_url.trim() || null,
        notes: form.notes.trim() || null,
        extra_config: form.extra_config.trim() || "{}",
        created_at: initialProvider?.created_at || new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      if (isEditing) {
        await update(payload);
      } else {
        await add(payload);
      }
      onSuccess();
    } catch (submitError) {
      setError(String(submitError));
    } finally {
      setIsSubmitting(false);
    }
  };

  const toggleTool = (tool: ToolTarget) => {
    setForm((current) => ({
      ...current,
      tool_targets: current.tool_targets.includes(tool)
        ? current.tool_targets.filter((item) => item !== tool)
        : [...current.tool_targets, tool],
    }));
  };

  const handleFetchModels = async () => {
    setIsFetchingModels(true);
    setModelFetchError("");
    try {
      const models = await api.providers.fetchModels({
        platform: form.platform,
        base_url: form.base_url.trim() || undefined,
        api_key_value: form.api_key_value.trim() || undefined,
        api_key_id: initialProvider?.api_key_id,
      });
      setModelOptions(models);
      if (!form.model_name.trim() && models.length > 0) {
        setForm((current) => ({ ...current, model_name: models[0] }));
      }
    } catch (fetchError) {
      setModelFetchError(String(fetchError));
    } finally {
      setIsFetchingModels(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal modal-wide" onClick={(event) => event.stopPropagation()}>
        <div className="modal-header">
          <h2>
            {isEditing
              ? `编辑 ${fixedTool ? TOOL_TARGET_LABELS[fixedTool] : ""}节点配置`
              : step === "preset"
                ? `${fixedTool ? TOOL_TARGET_LABELS[fixedTool] : "全局"}通道快速模板`
                : `新增 ${fixedTool ? TOOL_TARGET_LABELS[fixedTool] : ""}节点`}
          </h2>
          <button className="btn btn-icon" onClick={onClose}>✕</button>
        </div>

        {step === "preset" && !isEditing && (
          <ProviderPresetStep
            filteredPresets={filteredPresets}
            onApplyPreset={applyPreset}
            onClose={onClose}
            onContinue={() => setStep("form")}
            presetSearch={presetSearch}
            selectedCategory={selectedCategory}
            setPresetSearch={setPresetSearch}
            setSelectedCategory={setSelectedCategory}
          />
        )}

        {step === "form" && (
          <form className="modal-body" onSubmit={handleSubmit}>
            <div className="form-tabs">
              <div
                className={`form-tab ${activeTab === "basic" ? "active" : ""}`}
                onClick={() => setActiveTab("basic")}
              >
                基础设定
              </div>
              <div
                className={`form-tab ${activeTab === "advanced" ? "active" : ""}`}
                onClick={() => setActiveTab("advanced")}
              >
                调优与进阶特性
              </div>
            </div>

            {activeTab === "basic" && (
              <ProviderModalBasicTab
                fixedTool={fixedTool}
                form={form}
                isEditing={isEditing}
                isFetchingModels={isFetchingModels}
                modelFetchError={modelFetchError}
                modelOptions={modelOptions}
                onFetchModels={handleFetchModels}
                setForm={setForm}
                toggleTool={toggleTool}
              />
            )}

            {activeTab === "advanced" && (
              <ProviderModalAdvancedTab
                advancedCfg={advancedCfg}
                fixedTool={fixedTool}
                form={form}
                setForm={setForm}
              />
            )}

            {error && <div className="form-error">{error}</div>}

            <div className="modal-footer">
              {!isEditing && (
                <button type="button" className="btn btn-ghost" onClick={() => setStep("preset")}>
                  ← 重选预设
                </button>
              )}
              <button type="button" className="btn btn-ghost" onClick={onClose}>取消</button>
              <button type="submit" className="btn btn-primary" disabled={isSubmitting}>
                {isSubmitting ? "保存中…" : isEditing ? "保存修改" : "添加 Provider"}
              </button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}
