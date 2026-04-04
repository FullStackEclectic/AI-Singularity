import { useState, useEffect } from "react";
import AddIdeAccountDialog from "./AddIdeAccountDialog";
import { api } from "../../lib/api";
import type { IdeAccount } from "../../types";
import "./IdeAccountsPage.css";

export default function IdeAccountsPage() {
  const [accounts, setAccounts] = useState<IdeAccount[]>([]);
  const [loading, setLoading] = useState(true);
  const [dragActive, setDragActive] = useState(false);

  const fetchAccounts = async () => {
    try {
      const data = await api.ideAccounts.list();
      setAccounts(data);
    } catch (e) {
      console.error("Failed to fetch IDE accounts", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchAccounts();
  }, []);

  const handleDrag = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.type === "dragenter" || e.type === "dragover") {
      setDragActive(true);
    } else if (e.type === "dragleave") {
      setDragActive(false);
    }
  };

  const parseAndImport = async (text: string) => {
    try {
      const rawData = JSON.parse(text);
      let parsedAccounts: any[] = [];
      
      // Allow raw arrays or object maps
      if (Array.isArray(rawData)) {
        parsedAccounts = rawData;
      } else if (rawData && typeof rawData === "object") {
        // e.g. mapping of ID -> account
        parsedAccounts = Object.values(rawData);
      }

      const formattedAccounts = parsedAccounts.map((v: any, i: number) => {
        // Safe transform structure to match IdeAccount Rust type
        return {
          id: v.id || `virtual-${Date.now()}-${i}`,
          email: v.email || `unknown-${i}@spoofed.local`,
          origin_platform: v.origin_platform || "claude_code",
          token: {
            access_token: v.access_token || v.token?.access_token || "dummy_acc_token",
            refresh_token: v.refresh_token || v.token?.refresh_token || "dummy_ref_token",
            expires_in: v.expires_in || v.token?.expires_in || 3600,
            token_type: v.token_type || v.token?.token_type || "Bearer",
            updated_at: new Date().toISOString(),
          },
          status: "active",
          is_proxy_disabled: false,
          device_profile: v.device_profile || v.machine_id ? {
            machine_id: v.machine_id || v.device_profile?.machine_id || `sys-${Math.random().toString(36).substring(7)}`,
            mac_machine_id: v.mac_machine_id || v.device_profile?.mac_machine_id || `mac-${Math.random().toString(36).substring(7)}`,
            dev_device_id: v.dev_device_id || v.device_profile?.dev_device_id || crypto.randomUUID(),
            sqm_id: v.sqm_id || v.device_profile?.sqm_id || `{${crypto.randomUUID()}}`,
          } : undefined,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          last_used: new Date().toISOString(),
        };
      });

      if (formattedAccounts.length > 0) {
        const count = await api.ideAccounts.import(formattedAccounts);
        const { message } = await import("@tauri-apps/plugin-dialog");
        await message(`成功列装 ${count} 个核武级特权账号入列！`, { kind: "info", title: "降维打击矩阵上线" });
        fetchAccounts();
      }
    } catch (e) {
      console.error(e);
      const { message } = await import("@tauri-apps/plugin-dialog");
      await message("无法解析 JSON 文件结构，请提供标准的 accounts 备份数组格式。", { kind: "error", title: "解析溃散" });
    }
  };

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);

    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      const file = e.dataTransfer.files[0];
      const text = await file.text();
      await parseAndImport(text);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await api.ideAccounts.delete(id);
      fetchAccounts();
    } catch (e) {
      console.error("Delete failed", e);
    }
  };

  return (
    <div className="page-container page-ide-accounts">
      <div className="page-header">
        <div className="page-title-row">
          <h1>☢️ 代理兵工厂矩阵</h1>
          <p className="page-subtitle">构建免死金牌账号池，进行多指纹高维伪装欺骗与请求动态负载均衡。</p>
        </div>
        <div className="header-actions">
           <AddIdeAccountDialog onSuccess={fetchAccounts} />
        </div>
      </div>

      <div className="sandbox-launcher-panel">
        <div className="sandbox-header">
           <span className="sandbox-icon">🚀</span>
           <div>
             <h3>重装沙盒启动器 (Zero-Pollution Sandbox)</h3>
             <p>无需修改物理机环境变量。强制开启一个代理隔离舱终端，并在底层喂入虚拟 OAuth，点击即刻空投打击。</p>
           </div>
        </div>
        <div className="sandbox-actions">
           <button 
             className="btn btn-outline"
             onClick={async () => {
               try {
                 await api.ideAccounts.launchSandbox("claude", 8080);
               } catch (e: any) {
                 alert("启动大炮失败：" + e.toString());
               }
             }}
           >
             🪖 唤醒 Claude Code 隔离舱
           </button>
           <button 
             className="btn btn-outline"
             onClick={async () => {
               try {
                 await api.ideAccounts.launchSandbox("aider", 8080);
               } catch (e: any) {
                 alert("启动大炮失败：" + e.toString());
               }
             }}
           >
             ⚔️ 唤醒 Aider 隔离舱
           </button>
        </div>
      </div>

      <div 
        className={`massive-dropzone ${dragActive ? "active" : ""}`}
        onDragEnter={handleDrag}
        onDragLeave={handleDrag}
        onDragOver={handleDrag}
        onDrop={handleDrop}
      >
        <div className="dropzone-content">
          <div className="dropzone-icon">📥</div>
          <h3>部署指纹协议包</h3>
          <p>将包含 `OAuth` 令牌及 `DeviceProfile` 原声硬件码的 JSON 兵器库拖拽至此以激活强效池化伪装。</p>
        </div>
      </div>

      <div className="grid-summary">
        <div className="summary-card">
          <label>服役在列</label>
          <div className="val text-success">{accounts.filter(a => a.status === "active").length}</div>
        </div>
        <div className="summary-card">
          <label>被击毁 (403)</label>
          <div className="val text-error">{accounts.filter(a => a.status === "forbidden").length}</div>
        </div>
        <div className="summary-card">
          <label>休眠排队</label>
          <div className="val" style={{color: 'var(--text-muted)'}}>{accounts.filter(a => a.status === "rate_limited").length}</div>
        </div>
      </div>

      <div className="accounts-grid">
        {loading ? (
           <div className="loading-state">载入矩阵参数...</div>
        ) : accounts.length === 0 ? (
           <div className="empty-state">雷达空片：当前未捕获处于战备状态的代理凭证池。请空投靶装。</div>
        ) : (
          accounts.map(acc => (
            <div key={acc.id} className={`account-card status-${acc.status}`}>
              <div className="account-card-header">
                <div>
                  <div className="acc-email">{acc.email}</div>
                  <div className="acc-platform badge">{acc.origin_platform}</div>
                </div>
                <div className={`status-indicator ${acc.status}`}></div>
              </div>
              
              <div className="account-card-body">
                <div className="body-row">
                  <span className="label">状态信号:</span> 
                  <span className={`val status-string`}>{acc.status.toUpperCase()}</span>
                </div>
                {acc.disabled_reason && (
                  <div className="body-row error-reason">
                    <span className="label">战损诊断:</span>
                    <span className="val">{acc.disabled_reason}</span>
                  </div>
                )}
                <div className="body-row">
                  <span className="label">指纹拟态:</span> 
                  <span className="val">{acc.device_profile ? "已挂载护城河" : "裸奔(极危)"}</span>
                </div>
                {acc.device_profile && (
                  <div className="fingerprint-mask">
                    ID: {acc.device_profile.machine_id.substring(0,8)}...***
                  </div>
                )}
              </div>

              <div className="account-card-footer">
                <span className="last-used">心跳: {new Date(acc.last_used).toLocaleString()}</span>
                <div style={{ display: "flex", gap: "8px" }}>
                  <button 
                    className="btn btn-outline"
                    style={{ padding: "4px 8px", fontSize: "11px", borderColor: "#ff4d4f", color: "#ff4d4f" }}
                    onClick={async () => {
                      if(window.confirm(`确认要杀死正在运行中的 IDE，并强制将此账号 [${acc.email}] 从物理底层注入吗？`)) {
                        try {
                           await api.ideAccounts.forceInject(acc.id);
                           alert("🩸 突刺植入完成！IDE重置中...");
                        } catch(e: any) {
                           alert("注入失败：" + e.toString());
                        }
                      }
                    }}
                  >
                    💉 强注靶机
                  </button>
                  <button className="btn-icon" onClick={() => handleDelete(acc.id)}>🗑️</button>
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
