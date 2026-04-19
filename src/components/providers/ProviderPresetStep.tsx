import type { Dispatch, SetStateAction } from "react";
import type { ProviderCategory, ProviderPreset } from "../../data/providerPresets";
import { CATEGORY_LABELS } from "../../data/providerPresets";
import "./ProviderPresetStep.css";

const CATEGORY_COLORS: Record<ProviderCategory, string> = {
  relay: "#8B5CF6",
  official: "#3B82F6",
  custom: "#6B7280",
};

export function ProviderPresetStep({
  filteredPresets,
  onApplyPreset,
  onClose,
  onContinue,
  presetSearch,
  selectedCategory,
  setPresetSearch,
  setSelectedCategory,
}: {
  filteredPresets: ProviderPreset[];
  onApplyPreset: (preset: ProviderPreset) => void;
  onClose: () => void;
  onContinue: () => void;
  presetSearch: string;
  selectedCategory: ProviderCategory | "all";
  setPresetSearch: Dispatch<SetStateAction<string>>;
  setSelectedCategory: Dispatch<SetStateAction<ProviderCategory | "all">>;
}) {
  return (
    <div className="modal-body preset-panel">
      <div className="preset-toolbar">
        <input
          className="form-input"
          placeholder="搜索供应商…"
          value={presetSearch}
          onChange={(event) => setPresetSearch(event.target.value)}
          style={{ flex: 1 }}
        />
        <select
          className="form-input"
          value={selectedCategory}
          onChange={(event) => setSelectedCategory(event.target.value as ProviderCategory | "all")}
          style={{ width: 140 }}
        >
          <option value="all">全部分类</option>
          {(Object.keys(CATEGORY_LABELS) as ProviderCategory[]).map((category) => (
            <option key={category} value={category}>
              {CATEGORY_LABELS[category]}
            </option>
          ))}
        </select>
      </div>

      <div className="preset-grid">
        {filteredPresets.map((preset) => {
          const accentColor = CATEGORY_COLORS[preset.category];
          return (
            <button
              key={preset.presetId}
              className={`preset-card ${preset.category === "relay" ? "preset-card-premium" : ""}`}
              style={
                preset.category !== "relay"
                  ? { borderLeftColor: accentColor, borderLeftWidth: 3 }
                  : undefined
              }
              onClick={() => onApplyPreset(preset)}
            >
              <div className="preset-name">
                {preset.category === "relay" && <span style={{ marginRight: 6 }}>👑</span>}
                {preset.name}
              </div>
              <div
                className="preset-meta"
                style={{ color: accentColor, fontWeight: 600, fontSize: 11 }}
              >
                {CATEGORY_LABELS[preset.category]}
              </div>
              {preset.defaultBaseUrl && (
                <div className="preset-url font-mono">{preset.defaultBaseUrl}</div>
              )}
            </button>
          );
        })}
      </div>

      <div className="modal-footer">
        <button className="btn btn-ghost" onClick={onClose}>
          取消
        </button>
        <button className="btn btn-primary-outline" onClick={onContinue}>
          手动填写 →
        </button>
      </div>
    </div>
  );
}
