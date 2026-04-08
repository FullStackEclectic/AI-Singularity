import { useState, useEffect } from "react";
import { SlidersHorizontal, ChevronDown, ChevronUp, Network, ShieldAlert, Cpu } from "lucide-react";
import { api } from "../../lib/api";

interface EngineConfig {
  scheduling: {
    mode: "Balance" | "Priority" | "Latency";
    maxWaitSecs: number;
  };
  circuitBreaker: {
    enabled: boolean;
    backoffSteps: number[];
  };
  advancedThinking: {
    enabled: boolean;
    compressionThreshold: number;
    budgetLimit: number;
  };
}

const DEFAULT_CONFIG: EngineConfig = {
  scheduling: {
    mode: "Balance",
    maxWaitSecs: 60,
  },
  circuitBreaker: {
    enabled: true,
    backoffSteps: [60, 120, 300, 600],
  },
  advancedThinking: {
    enabled: false,
    compressionThreshold: 0.65,
    budgetLimit: 4096,
  },
};

export default function ProxyEngineSettingsPanel() {
  const [config, setConfig] = useState<EngineConfig>(DEFAULT_CONFIG);
  const [expandedSection, setExpandedSection] = useState<string | null>("scheduling");
  const [isLoaded, setIsLoaded] = useState(false);

  useEffect(() => {
    try {
      const saved = localStorage.getItem("proxy_engine_prefs");
      let initialConfig = DEFAULT_CONFIG;
      if (saved) {
        initialConfig = JSON.parse(saved);
        setConfig(initialConfig);
      }
      // 初始化时，向后端同步一次配置
      api.proxy.syncEngineConfig(initialConfig).catch(console.warn);
    } catch {}
    setIsLoaded(true);
  }, []);

  const updateConfig = (updater: (prev: EngineConfig) => EngineConfig) => {
    setConfig(prev => {
      const next = updater(prev);
      localStorage.setItem("proxy_engine_prefs", JSON.stringify(next));
      // 实时同步后端的内存运行时控制
      api.proxy.syncEngineConfig(next).catch(console.warn);
      return next;
    });
  };

  if (!isLoaded) return null;

  const toggleSection = (id: string) => {
    setExpandedSection(prev => prev === id ? null : id);
  };

  const updateBackoffStep = (index: number, val: string) => {
    let num = parseInt(val, 10);
    if (isNaN(num)) num = 0;
    updateConfig(c => {
      const next = { ...c };
      next.circuitBreaker.backoffSteps = [...next.circuitBreaker.backoffSteps];
      next.circuitBreaker.backoffSteps[index] = num;
      return next;
    });
  };

  const addBackoffStep = () => {
    updateConfig(c => {
      const next = { ...c };
      const steps = next.circuitBreaker.backoffSteps;
      const last = steps[steps.length - 1] || 60;
      next.circuitBreaker.backoffSteps = [...steps, last * 2];
      return next;
    });
  };

  const removeBackoffStep = (index: number) => {
    updateConfig(c => {
      const next = { ...c };
      if (next.circuitBreaker.backoffSteps.length > 1) {
        next.circuitBreaker.backoffSteps = next.circuitBreaker.backoffSteps.filter((_, i) => i !== index);
      }
      return next;
    });
  };

  return (
    <div className="proxy-card" style={{ marginTop: 24, padding: "20px 24px" }}>
      <div className="proxy-card-header" style={{ marginBottom: 16 }}>
        <SlidersHorizontal size={18} className="proxy-card-header-icon" />
        Advanced Engine Parameters (高级引擎参数)
      </div>
      <p style={{ margin: "0 0 20px 0", fontSize: 13, color: "var(--color-text-secondary)" }}>
        精细化调控网关引擎的并发调度、熔断避险与模型思维压缩策略，为企业级流量分发提供支持。
      </p>

      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        {/* Section 1: Scheduling */}
        <div style={{ border: "1px solid var(--color-border)", borderRadius: 12, overflow: "hidden" }}>
          <div 
            onClick={() => toggleSection("scheduling")}
            style={{ 
              padding: "12px 16px", background: "var(--color-bg-primary)", cursor: "pointer", 
              display: "flex", justifyContent: "space-between", alignItems: "center"
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 8, fontWeight: 600, fontSize: 14 }}>
              <Network size={16} className="text-primary" />
              流调度与负载均衡 (Scheduling Strategy)
            </div>
            {expandedSection === "scheduling" ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
          </div>
          {expandedSection === "scheduling" && (
            <div style={{ padding: 16, borderTop: "1px solid var(--color-border)" }}>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 20 }}>
                <div>
                  <label style={{ display: "block", fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)", marginBottom: 8 }}>
                    节点分发算法 (Algorithm)
                  </label>
                  <select 
                    className="form-select" 
                    style={{ width: "100%" }}
                    value={config.scheduling.mode}
                    onChange={(e) => updateConfig(c => ({...c, scheduling: {...c.scheduling, mode: e.target.value as any}}))}
                  >
                    <option value="Balance">均衡轮询 (Round Robin Balance)</option>
                    <option value="Priority">资产权重优先 (Strict Priority)</option>
                    <option value="Latency">最低延迟优先 (Low Latency First)</option>
                  </select>
                </div>
                <div>
                  <label style={{ display: "block", fontSize: 12, fontWeight: 600, color: "var(--color-text-secondary)", marginBottom: 8 }}>
                    单点穿透等待阈值 (Max Wait Secs)
                  </label>
                  <div style={{ position: "relative" }}>
                    <input 
                      type="number"
                      className="form-input" 
                      style={{ width: "100%" }}
                      value={config.scheduling.maxWaitSecs}
                      onChange={(e) => updateConfig(c => ({...c, scheduling: {...c.scheduling, maxWaitSecs: Number(e.target.value)}}))}
                    />
                    <span style={{ position: "absolute", right: 12, top: "50%", transform: "translateY(-50%)", fontSize: 12, color: "var(--color-text-muted)" }}>秒</span>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Section 2: Circuit Breaker */}
        <div style={{ border: "1px solid var(--color-border)", borderRadius: 12, overflow: "hidden" }}>
          <div 
            onClick={() => toggleSection("circuitBreaker")}
            style={{ 
              padding: "12px 16px", background: "var(--color-bg-primary)", cursor: "pointer", 
              display: "flex", justifyContent: "space-between", alignItems: "center"
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 8, fontWeight: 600, fontSize: 14 }}>
              <ShieldAlert size={16} className={config.circuitBreaker.enabled ? "text-success" : "text-muted"} />
              梯级熔断保护器 (Adaptive Circuit Breaker)
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
              <span style={{ fontSize: 12, color: config.circuitBreaker.enabled ? "var(--color-success)" : "var(--color-text-muted)", fontWeight: 600 }}>
                {config.circuitBreaker.enabled ? "ACTIVE" : "DISABLED"}
              </span>
              {expandedSection === "circuitBreaker" ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
            </div>
          </div>
          {expandedSection === "circuitBreaker" && (
            <div style={{ padding: 16, borderTop: "1px solid var(--color-border)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
                <span style={{ fontSize: 13, fontWeight: 600, color: "var(--color-text)" }}>触发异常后的逐步冻结策略 (退避秒数)</span>
                <div style={{ display: "flex", gap: 8 }}>
                  <button className="btn btn-secondary btn-sm" onClick={() => updateConfig(c => ({...c, circuitBreaker: {...c.circuitBreaker, enabled: !c.circuitBreaker.enabled}}))}>
                    {config.circuitBreaker.enabled ? "关闭熔断" : "开启熔断"}
                  </button>
                  <button className="btn btn-secondary btn-sm" onClick={addBackoffStep} disabled={!config.circuitBreaker.enabled}>新增阶梯</button>
                </div>
              </div>
              
              <div style={{ display: "flex", gap: 12, flexWrap: "wrap", opacity: config.circuitBreaker.enabled ? 1 : 0.5, pointerEvents: config.circuitBreaker.enabled ? "auto" : "none" }}>
                {config.circuitBreaker.backoffSteps.map((step, idx) => (
                  <div key={idx} style={{ 
                      minWidth: 100, flex: 1, padding: "8px 12px", borderRadius: 8, 
                      border: "1px solid var(--color-border)", background: "rgba(15, 23, 42, 0.02)",
                      display: "flex", flexDirection: "column", gap: 4
                    }}>
                    <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, fontWeight: 600, color: "var(--color-text-muted)" }}>
                      <span>LEVEL {idx + 1}</span>
                      {config.circuitBreaker.backoffSteps.length > 1 && (
                        <span style={{ cursor: "pointer", color: "var(--color-error)" }} onClick={() => removeBackoffStep(idx)}>✕</span>
                      )}
                    </div>
                    <div style={{ position: "relative" }}>
                      <input 
                        type="number"
                        className="form-input" 
                        style={{ width: "100%", padding: "4px 8px", fontSize: 13 }}
                        value={step}
                        onChange={(e) => updateBackoffStep(idx, e.target.value)}
                      />
                      <span style={{ position: "absolute", right: 8, top: "50%", transform: "translateY(-50%)", fontSize: 11, color: "var(--color-text-muted)", pointerEvents: "none" }}>s</span>
                    </div>
                  </div>
                ))}
              </div>
              <p style={{ marginTop: 12, fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 0 }}>
                注：当渠道接口连续遇到 429 或 502 时，将依此阶梯时间冻结对该渠道的心跳探测，防止并发雪崩。
              </p>
            </div>
          )}
        </div>

        {/* Section 3: Advanced Thinking */}
        <div style={{ border: "1px solid var(--color-border)", borderRadius: 12, overflow: "hidden" }}>
          <div 
            onClick={() => toggleSection("advancedThinking")}
            style={{ 
              padding: "12px 16px", background: "var(--color-bg-primary)", cursor: "pointer", 
              display: "flex", justifyContent: "space-between", alignItems: "center"
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 8, fontWeight: 600, fontSize: 14 }}>
              <Cpu size={16} className={config.advancedThinking.enabled ? "text-primary" : "text-muted"} />
              思维层与记忆降维压缩 (Cognitive Scaling)
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
              <span style={{ fontSize: 12, color: config.advancedThinking.enabled ? "var(--color-primary)" : "var(--color-text-muted)", fontWeight: 600 }}>
                {config.advancedThinking.enabled ? "ACTIVE" : "DISABLED"}
              </span>
              {expandedSection === "advancedThinking" ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
            </div>
          </div>
          {expandedSection === "advancedThinking" && (
            <div style={{ padding: 16, borderTop: "1px solid var(--color-border)", opacity: config.advancedThinking.enabled ? 1 : 0.8 }}>
               <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
                <span style={{ fontSize: 13, color: "var(--color-text-secondary)" }}>智能压缩巨量会话历史，在保留意图的同时节约流转成本。</span>
                <button className={`btn btn-sm ${config.advancedThinking.enabled ? "btn-secondary" : "btn-primary"}`} onClick={() => updateConfig(c => ({...c, advancedThinking: {...c.advancedThinking, enabled: !c.advancedThinking.enabled}}))}>
                  {config.advancedThinking.enabled ? "停用压缩" : "激活压缩算法"}
                </button>
              </div>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 24, pointerEvents: config.advancedThinking.enabled ? "auto" : "none" }}>
                <div>
                  <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 8 }}>
                    <label style={{ fontSize: 12, fontWeight: 600, color: "var(--color-text)" }}>触发压缩阙值 (Threshold)</label>
                    <span style={{ fontSize: 12, color: "var(--color-primary)", fontWeight: 600 }}>{Math.round(config.advancedThinking.compressionThreshold * 100)}%</span>
                  </div>
                  <input 
                    type="range" 
                    min="0.3" max="0.95" step="0.05"
                    style={{ width: "100%", accentColor: "var(--color-primary)" }}
                    value={config.advancedThinking.compressionThreshold}
                    onChange={(e) => updateConfig(c => ({...c, advancedThinking: {...c.advancedThinking, compressionThreshold: Number(e.target.value)}}))}
                  />
                  <p style={{ margin: "4px 0 0 0", fontSize: 11, color: "var(--color-text-muted)" }}>当历史消息长度超过目标池设定最大 Tokens 的百分比时引发拦截折叠。</p>
                </div>
                <div>
                    <label style={{ display: "block", fontSize: 12, fontWeight: 600, color: "var(--color-text)", marginBottom: 8 }}>
                       Thinking Token 预算墙 (Budget Limit)
                    </label>
                    <div style={{ position: "relative" }}>
                      <input 
                        type="number"
                        className="form-input" 
                        style={{ width: "100%" }}
                        value={config.advancedThinking.budgetLimit}
                        onChange={(e) => updateConfig(c => ({...c, advancedThinking: {...c.advancedThinking, budgetLimit: Number(e.target.value)}}))}
                      />
                      <span style={{ position: "absolute", right: 12, top: "50%", transform: "translateY(-50%)", fontSize: 12, color: "var(--color-text-muted)", pointerEvents: "none" }}>tokens</span>
                    </div>
                </div>
              </div>
            </div>
          )}
        </div>

      </div>
    </div>
  );
}
