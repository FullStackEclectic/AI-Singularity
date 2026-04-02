export interface McpPreset {
  id: string;
  name: string;
  category: 'System' | 'Web' | 'Code' | 'Database' | 'Utility';
  command: string;
  args: string[];
  description: string;
  icon: string;
  repoUrl: string;
  recommended: boolean;
  notes?: string;
}

export const MCP_PRESETS: McpPreset[] = [
  // ── 浏览器与 Web 自动化 ──────────────────────────────────────────────────
  {
    id: "puppeteer",
    name: "Puppeteer Browser",
    category: "Web",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-puppeteer"],
    description: "让 AI 拥有完整的浏览器控制力，进行网页抓取、交互和截图验证。",
    icon: "🌐",
    repoUrl: "https://github.com/modelcontextprotocol/servers/tree/main/src/puppeteer",
    recommended: true,
  },
  {
    id: "brave-search",
    name: "Brave Search",
    category: "Web",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-brave-search"],
    description: "使用 Brave Search API 进行无广告追踪的网络搜索。",
    icon: "🔍",
    repoUrl: "https://github.com/modelcontextprotocol/servers/tree/main/src/brave-search",
    recommended: true,
    notes: "需要在环境变量中设置 BRAVE_API_KEY",
  },
  {
    id: "fetch",
    name: "Web Fetch",
    category: "Web",
    command: "uvx",
    args: ["mcp-server-fetch"],
    description: "更轻量级的网页抓取工具，用于提取网站 Markdown / HTML。",
    icon: "📥",
    repoUrl: "https://github.com/modelcontextprotocol/servers/tree/main/src/fetch",
    recommended: false,
  },

  // ── 文件与系统调用 ────────────────────────────────────────────────────────
  {
    id: "filesystem",
    name: "Filesystem",
    category: "System",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-filesystem", "{替换为你允许AI访问的安全目录绝对路径}"],
    description: "允许 AI 可控地读写你指定的本地文件和文件夹。",
    icon: "📂",
    repoUrl: "https://github.com/modelcontextprotocol/servers/tree/main/src/filesystem",
    recommended: true,
    notes: "请务必将 Args 中的目录替换为你的真实开发路径",
  },
  {
    id: "memory",
    name: "Memory Graph",
    category: "System",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-memory"],
    description: "构建持久化的知识图谱，让 AI '记住' 你之前讨论过的上下文。",
    icon: "🧠",
    repoUrl: "https://github.com/modelcontextprotocol/servers/tree/main/src/memory",
    recommended: true,
  },
  {
    id: "sequential-thinking",
    name: "Sequential Thinking",
    category: "Utility",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-sequential-thinking"],
    description: "赋予基础大模型（如 Claude 4o）深度推理链的能力，解决复杂逻辑问题。",
    icon: "🪜",
    repoUrl: "https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking",
    recommended: true,
  },

  // ── Code 开发相关 ────────────────────────────────────────────────────────
  {
    id: "github",
    name: "GitHub",
    category: "Code",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-github"],
    description: "直接与你的 GitHub 仓库交互（创建 Issue、提交 PR、代码审查）。",
    icon: "🐙",
    repoUrl: "https://github.com/modelcontextprotocol/servers/tree/main/src/github",
    recommended: true,
    notes: "需要在环境变量中提供 GITHUB_PERSONAL_ACCESS_TOKEN",
  },
  {
    id: "gitlab",
    name: "GitLab",
    category: "Code",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-gitlab"],
    description: "GitLab 交互服务，支持内部项目管理。",
    icon: "🦊",
    repoUrl: "https://github.com/modelcontextprotocol/servers/tree/main/src/gitlab",
    recommended: false,
    notes: "需要在环境变量中提供 GITLAB_PERSONAL_ACCESS_TOKEN 和 GITLAB_API_URL（私有部署）",
  },

  // ── Database 数据库 ──────────────────────────────────────────────────────
  {
    id: "sqlite",
    name: "SQLite DB",
    category: "Database",
    command: "uvx",
    args: ["mcp-server-sqlite", "--db-path", "{你的 sqlite 文件绝对路径}"],
    description: "让 AI 分析本地 SQLite 数据库，安全执行只读查询或智能表结构探索。",
    icon: "🗄️",
    repoUrl: "https://github.com/modelcontextprotocol/servers/tree/main/src/sqlite",
    recommended: true,
  },
  {
    id: "postgres",
    name: "PostgreSQL",
    category: "Database",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://user:pass@localhost/dbname"],
    description: "赋予 AI 全生命周期管理你的 PG 数据库能力（包含创建表、查询分析）。",
    icon: "🐘",
    repoUrl: "https://github.com/modelcontextprotocol/servers/tree/main/src/postgres",
    recommended: false,
  }
];

export const MCP_CATEGORIES = ["System", "Web", "Code", "Database", "Utility"] as const; 
