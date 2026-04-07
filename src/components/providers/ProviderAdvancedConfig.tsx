import React, { useState } from "react";

export interface ProviderExtraConfig {
  proxyEnabled?: boolean;
  proxyUrl?: string;
  speedTestEnabled?: boolean;
  speedTestUrl?: string;
  pricingOverride?: boolean;
  costMultiplier?: number;
  apiFormat?: string;
  apiKeyField?: string;
}

interface ProviderAdvancedConfigProps {
  value: ProviderExtraConfig;
  onChange: (val: ProviderExtraConfig) => void;
}

export function ProviderAdvancedConfig({ value, onChange }: ProviderAdvancedConfigProps) {
  const [isOpen, setIsOpen] = useState(false);

  const update = (partial: Partial<ProviderExtraConfig>) => {
    onChange({ ...value, ...partial });
  };

  return (
    <div style={{ marginTop: 24, border: "1px solid var(--color-border)", borderRadius: "var(--radius-md)" }}>
      <button
        type="button"
        style={{
          width: "100%", padding: "12px 16px", display: "flex", justifyContent: "space-between", alignItems: "center",
          background: "var(--color-surface)", border: "none", cursor: "pointer",
          borderBottom: isOpen ? "1px solid var(--color-border)" : "none",
          borderTopLeftRadius: "var(--radius-md)", borderTopRightRadius: "var(--radius-md)"
        }}
        onClick={() => setIsOpen(!isOpen)}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, fontWeight: 600 }}>
          <span>🛠️</span>
          <span>高阶网络与计费配置 (Advanced Config)</span>
        </div>
        <span style={{ transform: isOpen ? "rotate(180deg)" : "none", transition: "transform 0.2s" }}>▼</span>
      </button>

      {isOpen && (
        <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 20 }}>
          
          {/* 网络代理 */}
          <div>
            <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, fontWeight: 500, marginBottom: 12 }}>
              <input type="checkbox" checked={!!value.proxyEnabled} onChange={e => update({ proxyEnabled: e.target.checked })} />
              启用自定义代理 (Proxy)
            </label>
            {value.proxyEnabled && (
              <div className="form-row" style={{ marginLeft: 24 }}>
                <label className="form-label">代理地址</label>
                <input
                  className="form-input font-mono"
                  placeholder="http://127.0.0.1:7890"
                  value={value.proxyUrl || ""}
                  onChange={e => update({ proxyUrl: e.target.value })}
                />
                <div className="form-hint">所有该源发送的请求将经过此代理。</div>
              </div>
            )}
          </div>

          {/* 故障测速重定向 */}
          <div>
            <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, fontWeight: 500, marginBottom: 12 }}>
              <input type="checkbox" checked={!!value.speedTestEnabled} onChange={e => update({ speedTestEnabled: e.target.checked })} />
              启用重定向锚点测速 (Speed Test)
            </label>
            {value.speedTestEnabled && (
              <div className="form-row" style={{ marginLeft: 24 }}>
                <label className="form-label">锚点探测 URL</label>
                <input
                  className="form-input font-mono"
                  placeholder="https://api.example.com/v1/models"
                  value={value.speedTestUrl || ""}
                  onChange={e => update({ speedTestUrl: e.target.value })}
                />
                <div className="form-hint">网关调度时将定时对此端点进行探活，若无响应将自动降级。</div>
              </div>
            )}
          </div>

          {/* 倍率覆写 */}
          <div>
            <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, fontWeight: 500, marginBottom: 12 }}>
              <input type="checkbox" checked={!!value.pricingOverride} onChange={e => update({ pricingOverride: e.target.checked })} />
              覆写计费倍率 (Cost Multiplier Override)
            </label>
            {value.pricingOverride && (
              <div className="form-row" style={{ marginLeft: 24 }}>
                <label className="form-label">成本倍数</label>
                <input
                  type="number"
                  step="0.1"
                  className="form-input font-mono"
                  placeholder="1.0"
                  value={value.costMultiplier || ""}
                  onChange={e => update({ costMultiplier: parseFloat(e.target.value) || undefined })}
                />
                <div className="form-hint">用于在 Dashboard 中强制重算此上游的财务消耗。</div>
              </div>
            )}
          </div>

        </div>
      )}
    </div>
  );
}
