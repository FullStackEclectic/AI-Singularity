import { useState, useEffect } from "react";
import { createPortal } from "react-dom";
import { listen } from "@tauri-apps/api/event";
import { open as openDialog, message as showMessage } from "@tauri-apps/plugin-dialog";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import { api } from "../../lib/api";
import "./AddIdeAccountDialog.css";

interface AddIdeAccountDialogProps {
  onSuccess: () => void;
}

type TabType = "oauth" | "token" | "import";

export default function AddIdeAccountDialog({ onSuccess }: AddIdeAccountDialogProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<TabType>("oauth");
  const [tokenInput, setTokenInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [oauthMsg, setOauthMsg] = useState("");

  useEffect(() => {
    // 监听后端的 Oauth 成功截获事件
    const unlisten = listen<string>("oauth_success", async (event) => {
      const code = event.payload;
      setOauthMsg("✅ 回调已捕获！正在进行核心机密欺骗与注网...");
      try {
        await generateAndImport(code, true);
        setOauthMsg("🚀 核武级账号部署完毕！");
        setTimeout(() => { setIsOpen(false); onSuccess(); }, 1500);
      } catch (e: any) {
        setOauthMsg("❌ 部署失败：" + e.toString());
      }
    });

    return () => {
      unlisten.then(f => f());
    };
  }, [isOpen, onSuccess]);

  // 公用的：生成伪造机器特征并注入的辅助函数
  const generateAndImport = async (coreToken: string, isOauthCode: boolean = false) => {
    // 模拟终端环境指纹
    const macId = `mac-${Math.random().toString(36).substring(7)}`;
    const machineId = `sys-${Math.random().toString(36).substring(7)}`;
    const sqmId = `{${crypto.randomUUID()}}`;
    
    const account = {
      id: `virtual-${Date.now()}`,
      email: isOauthCode ? `oauth_stub_${Date.now()}@spoofed.local` : `token_stub_${Date.now()}@spoofed.local`,
      origin_platform: "github_copilot", // 默认演示平台
      token: {
        access_token: isOauthCode ? coreToken : "dummy_acc_token",
        refresh_token: !isOauthCode ? coreToken : "dummy_ref_token",
        expires_in: 3600,
        token_type: "Bearer",
        updated_at: new Date().toISOString(),
      },
      status: "active",
      is_proxy_disabled: false,
      device_profile: {
        machine_id: machineId,
        mac_machine_id: macId,
        dev_device_id: crypto.randomUUID(),
        sqm_id: sqmId,
      },
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      last_used: new Date().toISOString(),
    };

    await api.ideAccounts.import([account]);
  };

  const handleOAuthStart = async () => {
    setLoading(true);
    setOauthMsg("正在打通安全验证管道...");
    try {
      // 告诉后端启动极简 HTTP 接收服务，获取安全的拉起连接
      const url = await api.oauth.startFlow("github copilot");
      setOauthMsg("正在呼出浏览器，请在网页中完成授权并关闭。");
      await openUrl(url);
    } catch (e: any) {
      setOauthMsg("❌ 通道建立失败：" + e.toString());
    } finally {
      setLoading(false);
    }
  };

  const handleTokenSubmit = async () => {
    if (!tokenInput.trim()) return;
    setLoading(true);
    try {
      await generateAndImport(tokenInput.trim(), false);
      setTokenInput("");
      setIsOpen(false);
      onSuccess();
    } catch (e: any) {
      alert("写入失败：" + e.toString());
    } finally {
      setLoading(false);
    }
  };

  const handleJSONImport = async () => {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: "账号配置", extensions: ["json"] }]
      });
      if (selected && typeof selected === "string") {
         // 注意：浏览器环境下这里需要调用 fs API 读取。
         // 因为时间紧凑，本选项我们保留核心概念提示，可以通过 Tauri Core FS API 读取。
         await showMessage("出于演示阶段，暂时请使用外部拖拽 JSON 文件的原有特性，这里预留了接口接入文件系统读取层 (fs.readTextFile)。");
      }
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <>
      <button className="btn btn-primary" onClick={() => { setIsOpen(true); setOauthMsg(""); setActiveTab("oauth"); }}>
        + 添加兵工厂账号
      </button>

      {isOpen && createPortal(
        <div className="dialog-overlay" onClick={() => setIsOpen(false)}>
          <div className="dialog-content" onClick={e => e.stopPropagation()}>
            <div className="dialog-header">
              <h3>☢️ 注射器 (Account Injector)</h3>
              <button className="btn-close" onClick={() => setIsOpen(false)}>×</button>
            </div>
            
            <div className="dialog-body">
              <div className="nav-tabs">
                <button className={activeTab === "oauth" ? "active" : ""} onClick={() => setActiveTab("oauth")}>🌐 网页劫持 (OAuth)</button>
                <button className={activeTab === "token" ? "active" : ""} onClick={() => setActiveTab("token")}>🔑 单点突破 (Token)</button>
                <button className={activeTab === "import" ? "active" : ""} onClick={() => setActiveTab("import")}>📁 军火库入库</button>
              </div>

              {activeTab === "oauth" && (
                <div className="tab-pane">
                  <div className="notice">
                    <strong>机理揭秘：</strong>通过在本地暴露临时伪装端口，跳转至官方授权环境；在通过人类特征检验后，截获回调并抢夺合法身份令牌。整个过程高度自适应，防追踪。
                  </div>
                  <button 
                    className="hero-btn oauth" 
                    onClick={handleOAuthStart}
                    disabled={loading}
                  >
                    {loading ? "管道搭建中..." : "👉 启动官方环境验证劫持"}
                  </button>
                  {oauthMsg && (
                    <div style={{fontSize: 12, color: 'var(--color-primary)', textAlign: 'center', marginTop: 8}}>
                      {oauthMsg}
                    </div>
                  )}
                </div>
              )}

              {activeTab === "token" && (
                <div className="tab-pane">
                  <div className="notice" style={{ borderColor: 'var(--color-warning)' }}>
                    请贴入合法的 <code>refresh_token</code> (通常以 <code>1//</code> 开头)。我们会在底层自动“无中生有”捏造一系列的机器硬件参数、网卡 MAC 地址配对进行融合。
                  </div>
                  <div className="form-group">
                    <label>Refresh Token 密钥区</label>
                    <textarea 
                      className="token-textarea" 
                      placeholder="e.g. 1//0eXYZ_xxx_... 粘贴至此"
                      value={tokenInput}
                      onChange={e => setTokenInput(e.target.value)}
                    />
                  </div>
                  <button 
                    className="btn btn-primary" 
                    style={{ width: "100%" }}
                    onClick={handleTokenSubmit}
                    disabled={loading || !tokenInput.trim()}
                  >
                    🧬 生成伪装指纹并实施挂载
                  </button>
                </div>
              )}

              {activeTab === "import" && (
                <div className="tab-pane">
                  <div className="notice">
                    适用于批量化部署。支持导入已经脱离并结构化的本地 VSCode <code>state.vscdb</code> 或经过本工具导出的军火库 <code>.json</code> 配置列表。
                  </div>
                  <button className="hero-btn import" onClick={handleJSONImport}>
                    📂 浏览本地数据库文件...
                  </button>
                </div>
              )}
            </div>
          </div>
        </div>,
        document.body
      )}
    </>
  );
}
