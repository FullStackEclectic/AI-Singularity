import { useEffect, useState, useRef } from "react";
import { api } from "../../lib/api";
import type { DesktopLogFile, DesktopLogReadResult } from "../../lib/api/types";
import "./LogsPage.css";

export default function LogsPage() {
  const [files, setFiles] = useState<DesktopLogFile[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [result, setResult] = useState<DesktopLogReadResult | null>(null);
  const [query, setQuery] = useState("");
  const [lines, setLines] = useState(500);
  const [loading, setLoading] = useState(false);
  const [listLoading, setListLoading] = useState(true);
  const [error, setError] = useState("");
  const contentRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    loadList();
  }, []);

  async function loadList() {
    setListLoading(true);
    try {
      const data = await api.logs.list();
      setFiles(data);
      if (data.length > 0 && !selected) {
        setSelected(data[0].name);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setListLoading(false);
    }
  }

  useEffect(() => {
    if (selected) loadLog(selected);
  }, [selected]);

  async function loadLog(name: string) {
    setLoading(true);
    setError("");
    try {
      const data = await api.logs.read(name, lines, query || undefined);
      setResult(data);
      // 滚动到底部
      setTimeout(() => {
        if (contentRef.current) {
          contentRef.current.scrollTop = contentRef.current.scrollHeight;
        }
      }, 50);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  function handleSearch(e: React.FormEvent) {
    e.preventDefault();
    if (selected) loadLog(selected);
  }

  function formatSize(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function formatDate(iso?: string | null) {
    if (!iso) return "—";
    return new Date(iso).toLocaleString("zh-CN", {
      month: "2-digit", day: "2-digit",
      hour: "2-digit", minute: "2-digit",
    });
  }

  // 简单高亮：ERROR 红、WARN 黄、INFO 蓝
  function colorLine(line: string) {
    if (/\bERROR\b/i.test(line)) return "log-line-error";
    if (/\bWARN\b/i.test(line)) return "log-line-warn";
    if (/\bINFO\b/i.test(line)) return "log-line-info";
    if (/\bDEBUG\b/i.test(line)) return "log-line-debug";
    return "";
  }

  const logLines = result?.content.split("\n") ?? [];

  return (
    <div className="logs-page animate-fade-in">
      <div className="page-header">
        <div>
          <h1 className="page-title">运行日志</h1>
          <p className="page-subtitle">查看应用运行时产生的日志文件，便于排查问题。</p>
        </div>
        <button className="btn btn-ghost" onClick={loadList} disabled={listLoading}>
          {listLoading ? "加载中..." : "⟳ 刷新列表"}
        </button>
      </div>

      <div className="logs-layout">
        {/* 左侧文件列表 */}
        <div className="logs-sidebar card">
          <div className="logs-sidebar-title">日志文件</div>
          {listLoading ? (
            <div className="logs-empty text-muted">加载中...</div>
          ) : files.length === 0 ? (
            <div className="logs-empty text-muted">暂无日志文件</div>
          ) : (
            <div className="logs-file-list">
              {files.map((f) => (
                <button
                  key={f.name}
                  className={`logs-file-item ${selected === f.name ? "active" : ""}`}
                  onClick={() => setSelected(f.name)}
                >
                  <div className="logs-file-name">{f.name}</div>
                  <div className="logs-file-meta">
                    <span className={`logs-kind-badge logs-kind-${f.kind}`}>{f.kind}</span>
                    <span className="text-muted">{formatSize(f.size)}</span>
                    <span className="text-muted">{formatDate(f.modified_at)}</span>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>

        {/* 右侧内容区 */}
        <div className="logs-content-area">
          {/* 工具栏 */}
          <form className="logs-toolbar card" onSubmit={handleSearch}>
            <input
              className="form-input logs-search"
              placeholder="关键词过滤（支持正则）"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            <select
              className="form-input logs-lines-select"
              value={lines}
              onChange={(e) => setLines(Number(e.target.value))}
            >
              <option value={200}>最近 200 行</option>
              <option value={500}>最近 500 行</option>
              <option value={1000}>最近 1000 行</option>
              <option value={2000}>最近 2000 行</option>
            </select>
            <button type="submit" className="btn btn-primary" disabled={loading || !selected}>
              {loading ? "加载中..." : "查看"}
            </button>
            {result && (
              <span className="text-muted logs-stat">
                共 {result.total_lines} 行
                {query ? `，匹配 ${result.matched_lines} 行` : ""}
              </span>
            )}
          </form>

          {error && (
            <div className="card logs-error">⚠ {error}</div>
          )}

          {/* 日志内容 */}
          <div className="card logs-viewer-card">
            {!selected ? (
              <div className="logs-empty text-muted">← 选择左侧日志文件</div>
            ) : loading ? (
              <div className="logs-empty text-muted animate-pulse">加载中...</div>
            ) : logLines.length === 0 ? (
              <div className="logs-empty text-muted">该文件暂无内容</div>
            ) : (
              <pre className="logs-viewer" ref={contentRef}>
                {logLines.map((line, i) => (
                  <span key={i} className={`log-line ${colorLine(line)}`}>
                    {line}{"\n"}
                  </span>
                ))}
              </pre>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
