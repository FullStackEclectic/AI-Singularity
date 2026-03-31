export default function ModelsPage() {
  return (
    <div>
      <div className="page-header">
        <div>
          <h1 className="page-title">模型目录</h1>
          <p className="page-subtitle">主流平台模型能力矩阵与价格对比</p>
        </div>
      </div>
      <div className="empty-state" style={{ padding: "var(--space-12)" }}>
        <div className="empty-state-icon">🤖</div>
        <h3 style={{ color: "var(--color-text-secondary)" }}>模型目录</h3>
        <p>Phase 1 完成后将展示各平台模型列表与价格对比</p>
      </div>
    </div>
  );
}
