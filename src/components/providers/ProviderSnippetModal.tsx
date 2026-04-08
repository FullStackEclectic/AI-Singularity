import React from "react";
import SyntaxHighlighter from "react-syntax-highlighter";
import { atomOneDark } from "react-syntax-highlighter/dist/esm/styles/hljs";
import type { ProviderConfig } from "../../types";

interface Props {
  provider: ProviderConfig;
  onClose: () => void;
}

export default function ProviderSnippetModal({ provider, onClose }: Props) {
  // 我们优先使用 provider 自带的 base_url，如果为空，则默认指向本地代理网关。
  const baseUrl = provider.base_url || "http://127.0.0.1:23333/v1";
  const modelName = provider.model_name || "gpt-3.5-turbo";

  // 在没有实际 Key 的情况下展示占位符
  const apiKey = "<YOUR_API_KEY_OR_AIS_TOKEN>";

  const curlCode = `curl ${baseUrl.replace(/\/$/, '')}/chat/completions \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer ${apiKey}" \\
  -d '{
    "model": "${modelName}",
    "messages": [
      {
        "role": "user",
        "content": "Hello!"
      }
    ]
  }'`;

  const pythonCode = `from openai import OpenAI

client = OpenAI(
    api_key="${apiKey}",
    base_url="${baseUrl}"
)

response = client.chat.completions.create(
    model="${modelName}",
    messages=[
        {"role": "user", "content": "Hello!"}
    ]
)

print(response.choices[0].message.content)`;

  const nodeCode = `import OpenAI from "openai";

const openai = new OpenAI({
  apiKey: "${apiKey}",
  baseURL: "${baseUrl}",
});

async function main() {
  const completion = await openai.chat.completions.create({
    messages: [{ role: "user", content: "Hello!" }],
    model: "${modelName}",
  });
  console.log(completion.choices[0].message.content);
}

main();`;

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    // 这里可以加一个简单的通知，暂时忽略
  };

  return (
    <div className="modal-overlay" onClick={onClose} style={{
      position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
      backgroundColor: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 9999
    }}>
      <div className="modal-content" onClick={e => e.stopPropagation()} style={{
        background: "var(--surface-base)", padding: "var(--space-6)", borderRadius: "var(--radius-lg)",
        width: "90%", maxWidth: "700px", maxHeight: "90vh", overflowY: "auto", boxShadow: "0 10px 30px rgba(0,0,0,0.5)"
      }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-4)" }}>
          <h2 style={{ margin: 0, fontSize: "1.25rem", fontWeight: 600 }}>接入集成代码片段 - {provider.name}</h2>
          <button onClick={onClose} style={{ background: "none", border: "none", fontSize: "1.5rem", cursor: "pointer", color: "var(--color-text-secondary)" }}>&times;</button>
        </div>

        <p className="text-muted" style={{ marginBottom: "var(--space-4)", fontSize: "14px" }}>
          使用以下代码将 <strong>{provider.name}</strong> 提供的能力接入到您的代码或客户端中。如果您使用了 AI Singularity 代理，建议 base_url 填写代理网关地址。
        </p>

        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-6)" }}>
          {/* cURL */}
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-2)" }}>
              <span style={{ fontWeight: 600 }}>cURL</span>
              <button className="btn btn-secondary btn-sm" onClick={() => handleCopy(curlCode)}>复制</button>
            </div>
            <SyntaxHighlighter language="bash" style={atomOneDark} customStyle={{ borderRadius: "var(--radius-sm)", margin: 0, fontSize: "13px" }}>
              {curlCode}
            </SyntaxHighlighter>
          </div>

          {/* Python */}
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-2)" }}>
              <span style={{ fontWeight: 600 }}>Python (OpenAI SDK)</span>
              <button className="btn btn-secondary btn-sm" onClick={() => handleCopy(pythonCode)}>复制</button>
            </div>
            <SyntaxHighlighter language="python" style={atomOneDark} customStyle={{ borderRadius: "var(--radius-sm)", margin: 0, fontSize: "13px" }}>
              {pythonCode}
            </SyntaxHighlighter>
          </div>

          {/* Node.js */}
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-2)" }}>
              <span style={{ fontWeight: 600 }}>Node.js (OpenAI SDK)</span>
              <button className="btn btn-secondary btn-sm" onClick={() => handleCopy(nodeCode)}>复制</button>
            </div>
            <SyntaxHighlighter language="javascript" style={atomOneDark} customStyle={{ borderRadius: "var(--radius-sm)", margin: 0, fontSize: "13px" }}>
              {nodeCode}
            </SyntaxHighlighter>
          </div>
        </div>
      </div>
    </div>
  );
}
