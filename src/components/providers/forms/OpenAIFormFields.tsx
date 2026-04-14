export interface PlatformConfigProps {
  value: any;
  onChange: (val: any) => void;
}

export function OpenAIFormFields({ value, onChange }: PlatformConfigProps) {
  const update = (partial: any) => onChange({ ...value, ...partial });
  
  return (
    <div style={{ marginTop: 16 }}>
      <div className="form-section-title" style={{ fontSize: 13, marginBottom: 12 }}>OpenAI 高阶选项 (Quirks)</div>
      <div className="form-row">
        <label className="form-label">API 路由类型</label>
        <select 
          className="form-input" 
          value={value.apiType || 'openai'}
          onChange={e => update({ apiType: e.target.value })}
        >
          <option value="openai">OpenAI 统一标准 (官方与兼容中转)</option>
          <option value="azure">Azure OpenAI</option>
        </select>
      </div>

      {value.apiType === 'azure' && (
        <div style={{ background: 'var(--color-surface-raised)', padding: 12, borderRadius: 8, marginTop: 8 }}>
          <div className="form-row">
            <label className="form-label">Azure API Version</label>
            <input 
              className="form-input" 
              placeholder="e.g. 2024-02-15-preview"
              value={value.azureApiVersion || ''}
              onChange={e => update({ azureApiVersion: e.target.value })}
            />
          </div>
          <div className="form-row" style={{ marginTop: 12 }}>
            <label className="form-label">Deployment ID 映射</label>
            <input 
              className="form-input" 
              placeholder="gpt-4o:my-gpt4o-deployment"
              value={value.azureDeploymentMapping || ''}
              onChange={e => update({ azureDeploymentMapping: e.target.value })}
            />
            <div className="form-hint">可选。格式: modelName:deploymentId 逗号分隔。</div>
          </div>
        </div>
      )}

      {value.apiType === 'openai' && (
        <div className="form-row" style={{ marginTop: 12 }}>
          <label className="form-label">Organization ID</label>
          <input 
            className="form-input font-mono" 
            placeholder="org-..."
            value={value.organizationId || ''}
            onChange={e => update({ organizationId: e.target.value })}
          />
          <div className="form-hint">若有指定的企业组织ID请填写，否则留空。</div>
        </div>
      )}
    </div>
  );
}
