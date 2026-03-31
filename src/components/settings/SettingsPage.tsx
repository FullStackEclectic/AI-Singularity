export default function SettingsPage() {
  return (
    <div>
      <div className="page-header">
        <div>
          <h1 className="page-title">设置</h1>
          <p className="page-subtitle">应用配置、云端同步、告警通知</p>
        </div>
      </div>
      <div className="empty-state" style={{ padding: "var(--space-12)" }}>
        <div className="empty-state-icon">⚙️</div>
        <h3 style={{ color: "var(--color-text-secondary)" }}>设置</h3>
        <p>后续版本将支持云端同步、通知配置等功能</p>
      </div>
    </div>
  );
}
