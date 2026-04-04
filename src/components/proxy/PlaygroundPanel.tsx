import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./PlaygroundPanel.css";

interface Message {
  role: "user" | "assistant";
  content: string;
  latency_ms?: number;
  error?: boolean;
}

const MODELS = [
  { value: "gpt-4o", label: "GPT-4o" },
  { value: "gpt-4o-mini", label: "GPT-4o-mini" },
  { value: "claude-3-5-sonnet-20241022", label: "Claude 3.5 Sonnet" },
  { value: "claude-3-haiku-20240307", label: "Claude 3 Haiku" },
  { value: "gemini-2.0-flash", label: "Gemini 2.0 Flash" },
  { value: "deepseek-chat", label: "DeepSeek Chat" },
];

export default function PlaygroundPanel() {
  const [proxyPort, setProxyPort] = useState<string>("3000");
  const [model, setModel] = useState("claude-3-5-sonnet-20241022");
  const [input, setInput] = useState("");
  const [messages, setMessages] = useState<Message[]>([]);
  const [isSending, setIsSending] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // 自动获取当前代理端口
    invoke<number>("get_proxy_status").then((port) => {
      if (port) setProxyPort(String(port));
    }).catch(() => {});
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const sendMessage = async () => {
    const text = input.trim();
    if (!text || isSending) return;
    setInput("");
    setIsSending(true);

    const userMsg: Message = { role: "user", content: text };
    setMessages((prev) => [...prev, userMsg]);

    const start = Date.now();
    try {
      const resp = await fetch(`http://127.0.0.1:${proxyPort}/v1/chat/completions`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          model,
          stream: false,
          messages: [
            ...messages.map((m) => ({ role: m.role, content: m.content })),
            { role: "user", content: text },
          ],
        }),
      });

      const latency_ms = Date.now() - start;

      if (!resp.ok) {
        const err = await resp.text();
        setMessages((prev) => [
          ...prev,
          { role: "assistant", content: `❌ HTTP ${resp.status}：${err}`, error: true, latency_ms },
        ]);
        return;
      }

      const data = await resp.json();
      const content =
        data?.choices?.[0]?.message?.content ??
        data?.error?.message ??
        JSON.stringify(data);

      setMessages((prev) => [
        ...prev,
        { role: "assistant", content, latency_ms },
      ]);
    } catch (e) {
      const latency_ms = Date.now() - start;
      setMessages((prev) => [
        ...prev,
        { role: "assistant", content: `❌ 网络错误：${String(e)}`, error: true, latency_ms },
      ]);
    } finally {
      setIsSending(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  return (
    <div className="playground-panel">
      <div className="playground-toolbar">
        <select
          className="pg-select"
          value={model}
          onChange={(e) => setModel(e.target.value)}
          title="选择模型"
        >
          {MODELS.map((m) => (
            <option key={m.value} value={m.value}>{m.label}</option>
          ))}
        </select>
        <div className="pg-port-row">
          <span className="pg-label">代理端口</span>
          <input
            className="pg-port-input"
            value={proxyPort}
            onChange={(e) => setProxyPort(e.target.value)}
            placeholder="3000"
          />
        </div>
        <button
          className="btn btn-ghost btn-sm"
          onClick={() => setMessages([])}
          disabled={messages.length === 0}
          title="清空对话"
        >
          🗑 清空
        </button>
      </div>

      <div className="playground-messages">
        {messages.length === 0 ? (
          <div className="pg-empty">
            <div className="pg-empty-icon">⚡</div>
            <p>向代理发送一条消息，验证连通性</p>
            <p className="pg-hint">Enter 发送 · Shift+Enter 换行</p>
          </div>
        ) : (
          messages.map((msg, i) => (
            <div key={i} className={`pg-bubble pg-bubble-${msg.role} ${msg.error ? "pg-bubble-error" : ""}`}>
              <div className="pg-bubble-role">{msg.role === "user" ? "你" : "🤖 AI"}</div>
              <div className="pg-bubble-content">{msg.content}</div>
              {msg.latency_ms != null && (
                <div className="pg-bubble-meta">{msg.latency_ms} ms</div>
              )}
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>

      <div className="playground-input-row">
        <textarea
          className="pg-textarea"
          placeholder="输入测试消息... (Enter 发送)"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          rows={2}
          disabled={isSending}
        />
        <button
          className="btn btn-primary pg-send-btn"
          onClick={sendMessage}
          disabled={isSending || !input.trim()}
        >
          {isSending ? <span className="animate-spin">⟳</span> : "发送"}
        </button>
      </div>
    </div>
  );
}
