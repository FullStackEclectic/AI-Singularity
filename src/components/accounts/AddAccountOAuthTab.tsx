import {
  Check,
  Copy,
  Database,
  ExternalLink,
  Globe,
  Key,
  Loader2,
  RotateCw,
  ShieldCheck,
  XCircle,
} from "lucide-react";
import type { DeviceFlowStart, Status } from "./addAccountWizardTypes";
import "./AddAccountOAuthTab.css";

type AddAccountOAuthTabProps = {
  ideOriginLabel: string;
  isImportOnly: boolean;
  isBrowserOAuth: boolean;
  isDeviceFlow: boolean;
  status: Status;
  deviceFlow: DeviceFlowStart | null;
  oauthPreparing: boolean;
  oauthUserCodeCopied: boolean;
  oauthUrlCopied: boolean;
  oauthPolling: boolean;
  oauthTimedOut: boolean;
  onStartDeviceFlow: () => void;
  onGoImportTab: () => void;
  onCopyUserCode: () => void;
  onCopyOAuthUrl: () => void;
  onOpenOAuthUrl: () => void;
};

export function AddAccountOAuthTab({
  ideOriginLabel,
  isImportOnly,
  isBrowserOAuth,
  isDeviceFlow,
  status,
  deviceFlow,
  oauthPreparing,
  oauthUserCodeCopied,
  oauthUrlCopied,
  oauthPolling,
  oauthTimedOut,
  onStartDeviceFlow,
  onGoImportTab,
  onCopyUserCode,
  onCopyOAuthUrl,
  onOpenOAuthUrl,
}: AddAccountOAuthTabProps) {
  return (
    <div className="wiz-tab-content">
      {!deviceFlow && !oauthPreparing && (
        <div className="wiz-oauth-empty">
          {isImportOnly && (
            <>
              <ShieldCheck
                size={40}
                className="wiz-oauth-icon"
                style={{ color: "var(--warning, #f59e0b)" }}
              />
              <p className="wiz-oauth-desc">
                <strong>{ideOriginLabel}</strong> 渠道不支持 OAuth 授权流程。
                <br />
                请切换到「Token 粘贴」或「导入账号」Tab 导入凭证。
              </p>
              <button className="wiz-btn-ghost" onClick={onGoImportTab}>
                <Database size={16} /> 前往导入 Tab
              </button>
            </>
          )}

          {isBrowserOAuth && (
            <>
              <Globe size={40} className="wiz-oauth-icon" />
              <p className="wiz-oauth-desc">
                点击下方按钮，将自动打开浏览器进行 OAuth 授权。
                <br />
                授权完成后浏览器页面会自动关闭，账号将自动导入。
              </p>
              <button
                className="wiz-btn-primary"
                onClick={onStartDeviceFlow}
                disabled={status === "success"}
              >
                <Globe size={16} /> 开启 OAuth 授权
              </button>
            </>
          )}

          {isDeviceFlow && (
            <>
              <Key size={40} className="wiz-oauth-icon" />
              <p className="wiz-oauth-desc">
                点击下方按钮获取授权码，然后在弹出的授权页面中输入验证码完成授权。
              </p>
              <button
                className="wiz-btn-primary"
                onClick={onStartDeviceFlow}
                disabled={status === "success"}
              >
                <Key size={16} /> 获取授权验证码
              </button>
            </>
          )}
        </div>
      )}

      {oauthPreparing && (
        <div className="wiz-oauth-empty">
          <Loader2 size={32} className="spin wiz-oauth-icon" />
          <p className="wiz-oauth-desc">正在获取验证码...</p>
        </div>
      )}

      {deviceFlow && !oauthPreparing && (
        <div className="wiz-device-flow">
          {deviceFlow.user_code && (
            <div className="wiz-user-code-block">
              <p className="wiz-uc-label">在授权页面输入此验证码：</p>
              <div className="wiz-user-code">
                {deviceFlow.user_code}
                <button className="wiz-copy-code-btn" onClick={onCopyUserCode} title="复制验证码">
                  {oauthUserCodeCopied ? <Check size={14} /> : <Copy size={14} />}
                </button>
              </div>
            </div>
          )}

          <div className="wiz-verification-url">
            <p className="wiz-uc-label">
              {deviceFlow.user_code ? "授权链接：" : "正在等待浏览器授权回调..."}
            </p>
            <div className="wiz-url-row">
              <code className="wiz-url-text">{deviceFlow.verification_uri}</code>
              <button className="wiz-icon-btn" onClick={onCopyOAuthUrl} title="复制链接">
                {oauthUrlCopied ? <Check size={13} /> : <Copy size={13} />}
              </button>
              <button className="wiz-icon-btn" onClick={onOpenOAuthUrl} title="在浏览器中打开">
                <ExternalLink size={13} />
              </button>
            </div>
          </div>

          <div className="wiz-poll-status">
            {oauthPolling && !oauthTimedOut && (
              <div className="wiz-poll-row">
                <div className="wiz-poll-pulse" />
                <span>
                  {deviceFlow.user_code
                    ? "等待您在浏览器中输入验证码并完成授权..."
                    : "等待浏览器授权回调，完成后将自动导入..."}
                </span>
              </div>
            )}
            {oauthTimedOut && (
              <div className="wiz-poll-row error">
                <XCircle size={14} />
                <span>授权已超时</span>
                <button className="wiz-link-btn" onClick={onStartDeviceFlow}>
                  <RotateCw size={12} /> 重新发起
                </button>
              </div>
            )}
          </div>

          {!oauthTimedOut && (
            <button
              className="wiz-btn-ghost wiz-retry-btn"
              onClick={onStartDeviceFlow}
              disabled={oauthPreparing}
            >
              <RotateCw size={14} /> 重新发起授权
            </button>
          )}
        </div>
      )}
    </div>
  );
}
