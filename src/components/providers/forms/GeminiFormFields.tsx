import React from 'react';

export interface PlatformConfigProps {
  value: any;
  onChange: (val: any) => void;
}

export function GeminiFormFields({ value, onChange }: PlatformConfigProps) {
  const update = (partial: any) => onChange({ ...value, ...partial });
  
  return (
    <div style={{ marginTop: 16 }}>
      <div className="form-section-title" style={{ fontSize: 13, marginBottom: 12 }}>Google 环境选项 (Quirks)</div>
      <div className="form-row">
        <label className="form-label">Project ID (GCP 专属)</label>
        <input 
          className="form-input font-mono" 
          placeholder="若使用 Vertex AI 需填写"
          value={value.projectId || ''}
          onChange={e => update({ projectId: e.target.value })}
        />
        <div className="form-hint">官方 AI Studio 直连或中转站请留空。</div>
      </div>
      <div className="form-row" style={{ marginTop: 12 }}>
        <label className="form-label">环境变量兼容策略</label>
        <select 
          className="form-input" 
          value={value.envInjection || 'standard'}
          onChange={e => update({ envInjection: e.target.value })}
        >
          <option value="standard">标准模型变量 (GEMINI_API_KEY)</option>
          <option value="legacy">旧版工具兼容 (包含 GOOGLE_API_KEY)</option>
        </select>
        <div className="form-hint">决定使用哪些环境变量名向工具暴露 Gemini 凭证信息。</div>
      </div>
    </div>
  );
}
