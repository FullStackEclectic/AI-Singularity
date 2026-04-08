import React from 'react';

export interface PlatformConfigProps {
  value: any;
  onChange: (val: any) => void;
}

export function AnthropicFormFields({ value, onChange }: PlatformConfigProps) {
  const update = (partial: any) => onChange({ ...value, ...partial });
  
  return (
    <div style={{ marginTop: 16 }}>
      <div className="form-section-title" style={{ fontSize: 13, marginBottom: 12 }}>Anthropic 高阶特性 (Quirks)</div>
      <div className="form-row">
        <label className="form-label">API 格式 (Format)</label>
        <select 
          className="form-input" 
          value={value.apiFormat || 'anthropic'}
          onChange={e => update({ apiFormat: e.target.value })}
        >
          <option value="anthropic">Anthropic Standard (官方与标准中转)</option>
          <option value="vertex">Google Vertex AI (GCP生态)</option>
          <option value="bedrock">AWS Bedrock (亚马逊生态)</option>
        </select>
        <div className="form-hint">仅当您使用云厂商原厂直连时修改。使用【中转站】或者【官方】请务必保留 Standard。</div>
      </div>
      
      <div className="form-row" style={{ marginTop: 12 }}>
        <label className="form-label">鉴权承载字段 (Auth Field)</label>
        <select 
          className="form-input" 
          value={value.apiKeyField || 'ANTHROPIC_API_KEY'}
          onChange={e => update({ apiKeyField: e.target.value })}
        >
          <option value="ANTHROPIC_API_KEY">ANTHROPIC_API_KEY</option>
          <option value="ANTHROPIC_AUTH_TOKEN">ANTHROPIC_AUTH_TOKEN</option>
        </select>
        <div className="form-hint">控制底层配置以何种环境变量向终端工具传递密钥。</div>
      </div>
    </div>
  );
}
