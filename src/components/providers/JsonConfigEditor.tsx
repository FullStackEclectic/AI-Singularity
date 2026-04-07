import React, { useState, useEffect } from "react";

interface JsonConfigEditorProps {
  value: string;
  onChange: (val: string) => void;
  height?: number;
}

export function JsonConfigEditor({ value, onChange, height = 240 }: JsonConfigEditorProps) {
  const [internalValue, setInternalValue] = useState(value);
  const [errorLine, setErrorLine] = useState<string | null>(null);

  useEffect(() => {
    setInternalValue(value);
  }, [value]);

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    setInternalValue(val);
    
    try {
      if (val.trim()) {
        JSON.parse(val);
      }
      setErrorLine(null);
      onChange(val);
    } catch (err: any) {
      setErrorLine(err.message);
      // Even if invalid, we want to allow typing
      onChange(val);
    }
  };

  const handleFormat = () => {
    try {
      const parsed = JSON.parse(internalValue);
      const formatted = JSON.stringify(parsed, null, 2);
      setInternalValue(formatted);
      onChange(formatted);
      setErrorLine(null);
    } catch {
      // ignore
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
        <label className="form-label">底层引擎配置 (JSON Core Options)</label>
        <button type="button" onClick={handleFormat} className="btn-sm btn-ghost" style={{ fontSize: 11 }}>
          格式化 JSON
        </button>
      </div>
      <div style={{ position: "relative" }}>
        <textarea
          className={`form-input font-mono ${errorLine ? "border-error" : ""}`}
          style={{ 
            height, 
            resize: "vertical", 
            backgroundColor: "#1e1e1e", 
            color: "#d4d4d4",
            padding: 12,
            lineHeight: 1.5,
            borderColor: errorLine ? "var(--color-danger)" : "var(--color-border)"
          }}
          value={internalValue}
          onChange={handleChange}
          spellCheck={false}
          placeholder="{\n  // Enter JSON config for target backend\n}"
        />
        {errorLine && (
          <div style={{ 
            color: "var(--color-danger)", 
            fontSize: 12, 
            marginTop: 4,
            padding: "4px 8px",
            background: "var(--color-danger-dim)",
            borderRadius: 4
          }}>
            ⚠️ 语法错误: {errorLine}
          </div>
        )}
      </div>
    </div>
  );
}
