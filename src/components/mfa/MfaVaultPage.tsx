import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import { Copy, Download, History, KeyRound, ShieldCheck, Star, Trash2, Upload } from "lucide-react";
import "./MfaVaultPage.css";

type MfaRecord = {
  id: string;
  accountName: string;
  secret: string;
  remark?: string;
  createdAt: number;
};

type ParsedCredential = {
  accountName: string;
  secret: string;
  period: number;
  digits: number;
  algorithm: "SHA-1" | "SHA-256" | "SHA-512";
  issuer?: string;
};

type ListTab = "saved" | "history";

const STORAGE_KEY_SAVED = "ais.mfa.saved.v1";
const STORAGE_KEY_HISTORY = "ais.mfa.history.v1";
const MAX_HISTORY = 50;

function createId() {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `mfa-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

function normalizeBase32(raw: string): string | null {
  const cleaned = raw.trim().replace(/[\s-]/g, "").toUpperCase();
  if (!cleaned) return null;
  if (!/^[A-Z2-7]+=*$/.test(cleaned)) return null;
  return cleaned.replace(/=+$/g, "");
}

function decodeBase32(input: string): Uint8Array | null {
  const normalized = normalizeBase32(input);
  if (!normalized) return null;
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
  let bits = 0;
  let value = 0;
  const bytes: number[] = [];

  for (const char of normalized) {
    const idx = alphabet.indexOf(char);
    if (idx < 0) return null;
    value = (value << 5) | idx;
    bits += 5;
    if (bits >= 8) {
      bytes.push((value >>> (bits - 8)) & 0xff);
      bits -= 8;
    }
  }

  return new Uint8Array(bytes);
}

function parseOtpAuthUri(raw: string): ParsedCredential | null {
  try {
    const url = new URL(raw.trim());
    if (url.protocol !== "otpauth:") return null;
    if (!url.hostname || url.hostname.toLowerCase() !== "totp") return null;
    const secret = url.searchParams.get("secret");
    const normalized = secret ? normalizeBase32(secret) : null;
    if (!normalized) return null;
    const label = decodeURIComponent(url.pathname.replace(/^\//, "")).trim();
    const issuer = url.searchParams.get("issuer")?.trim() || undefined;
    const algorithmRaw = (url.searchParams.get("algorithm") || "SHA1").toUpperCase();
    const algorithm =
      algorithmRaw === "SHA256" ? "SHA-256" :
      algorithmRaw === "SHA512" ? "SHA-512" :
      "SHA-1";
    const period = Math.max(1, Number(url.searchParams.get("period") || "30") || 30);
    const digits = Math.max(1, Number(url.searchParams.get("digits") || "6") || 6);
    return {
      accountName: label || issuer || "",
      secret: normalized,
      period,
      digits,
      algorithm,
      issuer,
    };
  } catch {
    return null;
  }
}

function parseCredential(raw: string): ParsedCredential | null {
  const input = raw.trim();
  if (!input) return null;
  if (input.toLowerCase().startsWith("otpauth://")) {
    return parseOtpAuthUri(input);
  }
  const normalized = normalizeBase32(input);
  if (!normalized) return null;
  return {
    accountName: "",
    secret: normalized,
    period: 30,
    digits: 6,
    algorithm: "SHA-1",
  };
}

function loadRecords(key: string): MfaRecord[] {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    const records = parsed
      .map((item): MfaRecord | null => {
        const secret = typeof item.secret === "string" ? normalizeBase32(item.secret) : null;
        if (!secret) return null;
        return {
          id: typeof item.id === "string" && item.id ? item.id : createId(),
          accountName: typeof item.accountName === "string" ? item.accountName : "",
          secret,
          remark: typeof item.remark === "string" ? item.remark : undefined,
          createdAt: Number(item.createdAt) || Date.now(),
        };
      });
    return records.filter((item): item is MfaRecord => item !== null);
  } catch {
    return [];
  }
}

function dedupe(records: MfaRecord[]) {
  const map = new Map<string, MfaRecord>();
  [...records]
    .sort((a, b) => b.createdAt - a.createdAt)
    .forEach((record) => {
      if (!map.has(record.secret)) {
        map.set(record.secret, record);
      }
    });
  return Array.from(map.values());
}

async function generateTotp(parsed: ParsedCredential, timestampMs: number) {
  const secretBytes = decodeBase32(parsed.secret);
  if (!secretBytes) return "";
  const counter = Math.floor(timestampMs / 1000 / parsed.period);
  const buffer = new ArrayBuffer(8);
  const view = new DataView(buffer);
  view.setUint32(0, Math.floor(counter / 2 ** 32));
  view.setUint32(4, counter >>> 0);

  const key = await crypto.subtle.importKey(
    "raw",
    secretBytes,
    { name: "HMAC", hash: parsed.algorithm },
    false,
    ["sign"],
  );
  const signature = new Uint8Array(await crypto.subtle.sign("HMAC", key, buffer));
  const offset = signature[signature.length - 1] & 0x0f;
  const binary =
    ((signature[offset] & 0x7f) << 24) |
    (signature[offset + 1] << 16) |
    (signature[offset + 2] << 8) |
    signature[offset + 3];
  const mod = 10 ** parsed.digits;
  return String(binary % mod).padStart(parsed.digits, "0");
}

export default function MfaVaultPage() {
  const [savedRecords, setSavedRecords] = useState<MfaRecord[]>(() => dedupe(loadRecords(STORAGE_KEY_SAVED)));
  const [historyRecords, setHistoryRecords] = useState<MfaRecord[]>(() => dedupe(loadRecords(STORAGE_KEY_HISTORY)).slice(0, MAX_HISTORY));
  const [activeTab, setActiveTab] = useState<ListTab>("saved");
  const [inputValue, setInputValue] = useState("");
  const [inputError, setInputError] = useState("");
  const [activeCredential, setActiveCredential] = useState<ParsedCredential | null>(null);
  const [generatedCode, setGeneratedCode] = useState("");
  const [remainingSeconds, setRemainingSeconds] = useState(30);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY_SAVED, JSON.stringify(savedRecords));
  }, [savedRecords]);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY_HISTORY, JSON.stringify(historyRecords));
  }, [historyRecords]);

  useEffect(() => {
    const update = async () => {
      const now = Date.now();
      const seconds = Math.floor(now / 1000);
      const remaining = 30 - (seconds % 30);
      setRemainingSeconds(remaining === 0 ? 30 : remaining);
      if (activeCredential) {
        const code = await generateTotp(activeCredential, now);
        setGeneratedCode(code);
      } else {
        setGeneratedCode("");
      }
    };

    update();
    const timer = window.setInterval(update, 1000);
    return () => window.clearInterval(timer);
  }, [activeCredential]);

  const visibleRecords = useMemo(
    () => (activeTab === "saved" ? savedRecords : historyRecords),
    [activeTab, savedRecords, historyRecords]
  );

  const handleQuery = () => {
    const parsed = parseCredential(inputValue);
    if (!parsed) {
      setInputError("请输入有效的 otpauth://totp/... URI 或 Base32 秘钥");
      return;
    }
    setInputError("");
    setActiveCredential(parsed);
    const historyRecord: MfaRecord = {
      id: createId(),
      accountName: parsed.accountName || parsed.issuer || "",
      secret: parsed.secret,
      remark: "",
      createdAt: Date.now(),
    };
    setHistoryRecords((prev) => dedupe([historyRecord, ...prev]).slice(0, MAX_HISTORY));
  };

  const handleSave = () => {
    const parsed = activeCredential || parseCredential(inputValue);
    if (!parsed) {
      setInputError("请先输入有效的 2FA 凭据");
      return;
    }
    const record: MfaRecord = {
      id: createId(),
      accountName: parsed.accountName || parsed.issuer || "",
      secret: parsed.secret,
      remark: "",
      createdAt: Date.now(),
    };
    setSavedRecords((prev) => dedupe([record, ...prev]));
    setMessage("已保存到 2FA 收藏列表");
  };

  const handleDelete = (id: string, tab: ListTab) => {
    if (tab === "saved") {
      setSavedRecords((prev) => prev.filter((item) => item.id !== id));
    } else {
      setHistoryRecords((prev) => prev.filter((item) => item.id !== id));
    }
  };

  const handleCopy = async (id: string, text: string, success: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedId(id);
      setMessage(success);
      setTimeout(() => setCopiedId(null), 1200);
    } catch (e) {
      setMessage("复制失败：" + String(e));
    }
  };

  const handleExport = () => {
    const blob = new Blob([JSON.stringify(savedRecords, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `ai-singularity-mfa-${new Date().toISOString().replace(/[:.]/g, "-")}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    setMessage("2FA 列表已导出");
  };

  const handleImportFile = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;
    try {
      const text = await file.text();
      const parsed = JSON.parse(text);
      const list = Array.isArray(parsed) ? parsed : [parsed];
      const imported = dedupe(
        list
          .map((item): MfaRecord | null => {
            const source = typeof item === "string" ? item : item?.secret || item?.otpauth || "";
            const parsedCredential = parseCredential(String(source || ""));
            if (!parsedCredential) return null;
            return {
              id: createId(),
              accountName: typeof item?.accountName === "string" ? item.accountName : parsedCredential.accountName,
              secret: parsedCredential.secret,
              remark: typeof item?.remark === "string" ? item.remark : undefined,
              createdAt: Date.now(),
            };
          })
          .filter((item): item is MfaRecord => item !== null)
      );
      if (imported.length === 0) {
        setMessage("导入失败：未发现有效的 2FA 记录");
      } else {
        setSavedRecords((prev) => dedupe([...imported, ...prev]));
        setMessage(`已导入 ${imported.length} 条 2FA 记录`);
      }
    } catch (e) {
      setMessage("导入失败：" + String(e));
    } finally {
      event.target.value = "";
    }
  };

  const handleLoadRecord = (record: MfaRecord) => {
    const parsed = parseCredential(record.secret);
    if (!parsed) {
      setMessage("该记录的秘钥无效");
      return;
    }
    setInputValue(record.secret);
    setActiveCredential({
      ...parsed,
      accountName: record.accountName || parsed.accountName,
    });
    setInputError("");
  };

  return (
    <div className="mfa-page">
      <div className="page-header">
        <div>
          <h1 className="page-title"><ShieldCheck size={22} className="text-primary" /> 2FA / MFA 管理</h1>
          <p className="page-subtitle">查询和保存 TOTP 动态码，支持 Base32 秘钥与 otpauth URI。</p>
        </div>
      </div>

      <div className="mfa-layout">
        <div className="card mfa-query-card">
          <div className="mfa-section-header">
            <KeyRound size={18} />
            <span>即时查询</span>
          </div>
          <textarea
            className="form-input mfa-input"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            placeholder="输入 otpauth://totp/... 或 Base32 秘钥"
          />
          <div className="mfa-hint">
            支持直接粘贴 `otpauth://totp/...` 或纯 Base32 秘钥，系统会自动规范化后生成动态码。
          </div>
          {inputError && <div className="mfa-inline-error">{inputError}</div>}

          <div className="mfa-action-row">
            <button className="btn btn-primary" onClick={handleQuery}>生成动态码</button>
            <button className="btn btn-secondary" onClick={handleSave}>保存到收藏</button>
            <button className="btn btn-secondary" onClick={() => fileInputRef.current?.click()}>
              <Upload size={14} /> 导入
            </button>
            <button className="btn btn-secondary" onClick={handleExport}>
              <Download size={14} /> 导出
            </button>
            <input
              ref={fileInputRef}
              type="file"
              accept=".json,application/json"
              style={{ display: "none" }}
              onChange={handleImportFile}
            />
          </div>

          {message && <div className="mfa-message">{message}</div>}

          <div className="mfa-live-panel">
            <div>
              <div className="mfa-live-label">当前动态码</div>
              <div className="mfa-live-code">{generatedCode || "------"}</div>
              <div className="mfa-live-meta">
                {activeCredential?.accountName || activeCredential?.issuer || "未命名凭据"}
              </div>
            </div>
            <div className="mfa-countdown">
              <div className="mfa-countdown-value">{remainingSeconds}s</div>
              <div className="mfa-countdown-bar">
                <div className="mfa-countdown-fill" style={{ width: `${(remainingSeconds / 30) * 100}%` }} />
              </div>
            </div>
          </div>
        </div>

        <div className="card mfa-list-card">
          <div className="mfa-list-toolbar">
            <div className="mfa-section-header">
              {activeTab === "saved" ? <Star size={18} /> : <History size={18} />}
              <span>{activeTab === "saved" ? "收藏凭据" : "查询历史"}</span>
            </div>
            <div className="mfa-tab-switcher">
              <button className={`mfa-tab ${activeTab === "saved" ? "active" : ""}`} onClick={() => setActiveTab("saved")}>
                收藏
              </button>
              <button className={`mfa-tab ${activeTab === "history" ? "active" : ""}`} onClick={() => setActiveTab("history")}>
                历史
              </button>
            </div>
          </div>

          {visibleRecords.length === 0 ? (
            <div className="mfa-empty">当前没有记录</div>
          ) : (
            <div className="mfa-record-list">
              {visibleRecords.map((record) => (
                <div key={record.id} className="mfa-record-item">
                  <div className="mfa-record-main">
                    <div className="mfa-record-title">{record.accountName || "未命名账号"}</div>
                    <div className="mfa-record-secret">{record.secret}</div>
                    <div className="mfa-record-time">{new Date(record.createdAt).toLocaleString()}</div>
                  </div>
                  <div className="mfa-record-actions">
                    <button className="btn btn-ghost btn-sm" onClick={() => handleLoadRecord(record)}>查看</button>
                    <button className="btn btn-ghost btn-sm" onClick={async () => {
                      const parsed = parseCredential(record.secret);
                      if (!parsed) return;
                      const code = await generateTotp(parsed, Date.now());
                      if (code) handleCopy(`code-${record.id}`, code, "动态码已复制");
                    }}>
                      <Copy size={14} />
                      {copiedId === `code-${record.id}` ? "已复制" : "复制动态码"}
                    </button>
                    <button className="btn btn-ghost btn-sm" onClick={() => handleCopy(`secret-${record.id}`, record.secret, "秘钥已复制")}>
                      <KeyRound size={14} />
                      {copiedId === `secret-${record.id}` ? "已复制" : "复制秘钥"}
                    </button>
                    <button className="btn btn-danger-ghost btn-sm" onClick={() => handleDelete(record.id, activeTab)}>
                      <Trash2 size={14} /> 删除
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
