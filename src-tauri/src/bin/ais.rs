//! AI Singularity CLI — 与桌面端共享 Core Engine。
//!
//! 设计原则：
//!   - 复用 ai_singularity_lib 的 services 与 db 层（不引入 Tauri 运行时）
//!   - 数据目录通过 dirs::data_dir() + bundle id 自行解析，与桌面端 tauri 默认一致
//!   - Keychain 通过 keyring 跨进程访问

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use ai_singularity_lib::db::Database;
use ai_singularity_lib::models::{McpServer, Platform};
use ai_singularity_lib::proxy::server::ProxyServer;
use ai_singularity_lib::services::balance_tracker::BalanceTracker;
use ai_singularity_lib::services::mcp::McpService;
use ai_singularity_lib::services::model_catalog::ModelCatalogService;
use ai_singularity_lib::services::model_mapping::ModelMappingService;
use ai_singularity_lib::services::provider::ProviderService;
use ai_singularity_lib::services::speedtest::SpeedTestService;
use ai_singularity_lib::services::validator;
use ai_singularity_lib::store::SecureStore;
use clap::{Parser, Subcommand};
use uuid::Uuid;

const BUNDLE_ID: &str = "com.ai-singularity.app";
const DEFAULT_PROXY_PORT: u16 = 8765;

#[derive(Parser)]
#[command(
    name = "ais",
    version,
    about = "AI Singularity CLI — AI 资源统一管理控制台",
    long_about = "复用桌面端的本地数据库与 Keychain，可在终端中直接管理 Key/Provider/Proxy/MCP/Model。"
)]
struct Cli {
    /// 显式指定数据目录（默认与桌面端共享 OS 标准路径）
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,

    /// 内部参数：以代理守护进程模式运行（由 proxy start 自动传入，勿手动使用）
    #[arg(long, hide = true)]
    proxy_daemon: bool,

    /// 内部参数：代理守护进程监听端口
    #[arg(long, hide = true, default_value_t = DEFAULT_PROXY_PORT)]
    port: u16,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// API Key 管理
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },
    /// Provider（AI 编码工具配置）管理
    Provider {
        #[command(subcommand)]
        action: ProviderAction,
    },
    /// 本地代理管理
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },
    /// 余额汇总查询
    Balance {
        /// 只显示指定平台的余额（如 open_ai、anthropic、deep_seek 等）
        #[arg(long)]
        platform: Option<String>,
    },
    /// 模型路由映射管理
    Route {
        #[command(subcommand)]
        action: RouteAction,
    },
    /// MCP Server 管理
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
    /// 模型目录管理
    Model {
        #[command(subcommand)]
        action: ModelAction,
    },
}

// ── Key ──────────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
enum KeyAction {
    /// 列出所有已录入的 Key
    List,
    /// 检测某个 Key 当前的可用性
    Check {
        /// Key ID（来自 `ais key list`）
        id: String,
    },
    /// 交互式添加新 API Key
    Add,
}

// ── Provider ─────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
enum ProviderAction {
    /// 列出所有 Provider
    List,
    /// 切换激活某个 Provider（同时触发同步到 Claude Code/Codex 等工具）
    Switch {
        /// Provider ID
        id: String,
    },
}

// ── Proxy ────────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
enum ProxyAction {
    /// 探活本地代理端口（默认 8765）
    Status {
        #[arg(long, default_value_t = DEFAULT_PROXY_PORT)]
        port: u16,
    },
    /// 在后台启动代理守护进程，并写入 PID 文件
    Start {
        #[arg(long, default_value_t = DEFAULT_PROXY_PORT)]
        port: u16,
    },
    /// 读取 PID 文件并终止代理守护进程
    Stop,
}

// ── Route ────────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
enum RouteAction {
    /// 列出所有模型路由映射规则
    List,
}

// ── MCP ──────────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
enum McpAction {
    /// 列出所有 MCP Server
    List,
    /// 添加新 MCP Server
    Add {
        /// Server 名称
        #[arg(long)]
        name: String,
        /// 启动命令（如 npx、python 等）
        #[arg(long)]
        command: String,
        /// 命令参数（JSON 数组字符串，如 '["arg1","arg2"]'）
        #[arg(long)]
        args: Option<String>,
        /// 环境变量（JSON 对象字符串，如 '{"KEY":"VALUE"}'）
        #[arg(long)]
        env: Option<String>,
    },
}

// ── Model ────────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
enum ModelAction {
    /// 列出模型目录（可按平台过滤）
    List {
        /// 平台过滤（如 open_ai、anthropic、gemini 等）
        #[arg(long)]
        platform: Option<String>,
    },
    /// 对比两个模型的价格与能力
    Compare {
        /// 第一个模型 ID
        model1: String,
        /// 第二个模型 ID
        model2: String,
    },
    /// 对各平台 endpoint 发送探测请求并测量延迟
    Speedtest {
        /// 只测试指定平台（如 open_ai、anthropic 等）
        #[arg(long)]
        platform: Option<String>,
    },
}

// ════════════════════════════════════════════════════════════════════════════
// main / run
// ════════════════════════════════════════════════════════════════════════════

fn main() -> ExitCode {
    let cli = Cli::parse();

    // 内部守护进程模式：直接运行代理逻辑（由 proxy start 子进程调用）
    if cli.proxy_daemon {
        return run_proxy_daemon(cli.port);
    }

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("ais: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let data_dir = cli.data_dir.clone().unwrap_or_else(default_data_dir);
    if !data_dir.exists() {
        anyhow::bail!(
            "数据目录不存在：{}\n请先启动一次 AI Singularity 桌面端，或通过 --data-dir 指定路径。",
            data_dir.display()
        );
    }

    let command = cli.command.ok_or_else(|| {
        anyhow::anyhow!("请指定子命令。使用 --help 查看可用命令。")
    })?;

    match command {
        Command::Key { action } => run_key(action, &data_dir),
        Command::Provider { action } => run_provider(action, &data_dir),
        Command::Proxy { action } => run_proxy(action, &data_dir),
        Command::Balance { platform } => run_balance(&data_dir, platform.as_deref()),
        Command::Route { action } => run_route(action, &data_dir),
        Command::Mcp { action } => run_mcp(action, &data_dir),
        Command::Model { action } => run_model(action, &data_dir),
    }
}

/// 模拟 Tauri 默认的 app_data_dir 解析：dirs::data_dir() / bundle_id
fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(BUNDLE_ID)
}

fn open_db(data_dir: &PathBuf) -> anyhow::Result<Database> {
    let db_path = data_dir.join("data.db");
    Database::new(&db_path)
        .map_err(|e| anyhow::anyhow!("打开数据库失败 ({}): {e}", db_path.display()))
}

/// 从 stdin 读取一行，去除首尾空白
fn read_line(prompt: &str) -> anyhow::Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

/// 读取一行，若为空则返回 None
fn read_optional(prompt: &str) -> anyhow::Result<Option<String>> {
    let s = read_line(prompt)?;
    Ok(if s.is_empty() { None } else { Some(s) })
}

// ════════════════════════════════════════════════════════════════════════════
// key
// ════════════════════════════════════════════════════════════════════════════

fn run_key(action: KeyAction, data_dir: &PathBuf) -> anyhow::Result<()> {
    let db = open_db(data_dir)?;
    match action {
        KeyAction::List => list_keys(&db),
        KeyAction::Check { id } => check_key(&db, &id),
        KeyAction::Add => add_key_interactive(&db),
    }
}

fn list_keys(db: &Database) -> anyhow::Result<()> {
    let rows: Vec<(String, String, String, String, String, Option<String>)> = db
        .query_rows(
            "SELECT id, name, platform, status, key_preview, last_checked_at \
             FROM api_keys ORDER BY priority DESC, created_at DESC",
            &[],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .map_err(|e| anyhow::anyhow!("读取 api_keys 失败: {e}"))?;

    if rows.is_empty() {
        println!("(没有任何 Key，可在桌面端或通过 `ais key add` 添加)");
        return Ok(());
    }

    println!(
        "{:<38}  {:<20}  {:<14}  {:<10}  {:<20}  {}",
        "ID", "Name", "Platform", "Status", "LastChecked", "Preview"
    );
    println!("{}", "-".repeat(120));
    for (id, name, platform, status, preview, last_checked) in rows {
        println!(
            "{:<38}  {:<20.20}  {:<14}  {:<10}  {:<20}  {}",
            id,
            name,
            platform,
            status,
            last_checked.as_deref().unwrap_or("-"),
            preview
        );
    }
    Ok(())
}

fn check_key(db: &Database, id: &str) -> anyhow::Result<()> {
    let secret = SecureStore::get_key(id)
        .map_err(|e| anyhow::anyhow!("从 Keychain 读取 Key 失败: {e}"))?;

    let (platform_str, base_url): (String, Option<String>) = db
        .query_one(
            "SELECT platform, base_url FROM api_keys WHERE id = ?1",
            &[&id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| anyhow::anyhow!("Key ID 不存在或读取失败: {e}"))?;

    let platform = serde_json::from_str::<Platform>(&format!("\"{platform_str}\""))
        .unwrap_or(Platform::Custom);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let status =
        runtime.block_on(validator::check_key_validity(&platform, &secret, base_url.as_deref()));

    let status_str = serde_json::to_string(&status)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    db.execute(
        "UPDATE api_keys SET status = ?1, last_checked_at = ?2 WHERE id = ?3",
        &[
            &status_str,
            &chrono::Utc::now().to_rfc3339(),
            &id,
        ],
    )
    .map_err(|e| anyhow::anyhow!("写回 Key 状态失败: {e}"))?;

    println!("Key {id}: {status_str}");
    Ok(())
}

fn add_key_interactive(db: &Database) -> anyhow::Result<()> {
    println!("=== 添加新 API Key ===");
    println!("（可选字段直接回车跳过）\n");

    let name = loop {
        let v = read_line("名称 (name): ")?;
        if !v.is_empty() {
            break v;
        }
        eprintln!("名称不能为空，请重新输入。");
    };

    // 列出可用平台供参考
    println!("\n可用平台：open_ai / anthropic / gemini / deep_seek / aliyun / bytedance /");
    println!("          moonshot / zhipu / mini_max / step_fun / aws_bedrock / nvidia_nim /");
    println!("          azure_open_a_i / silicon_flow / open_router / groq / mistral /");
    println!("          x_ai / cohere / perplexity / together_ai / ollama / hugging_face /");
    println!("          replicate / copilot / auth0_i_d_e / custom\n");

    let platform_str = loop {
        let v = read_line("平台 (platform): ")?;
        if !v.is_empty() {
            break v;
        }
        eprintln!("平台不能为空，请重新输入。");
    };

    let platform = serde_json::from_str::<Platform>(&format!("\"{platform_str}\""))
        .unwrap_or_else(|_| {
            eprintln!("警告：未识别的平台 \"{platform_str}\"，将使用 custom。");
            Platform::Custom
        });

    let secret = loop {
        let v = read_line("API Key (secret): ")?;
        if !v.is_empty() {
            break v;
        }
        eprintln!("API Key 不能为空，请重新输入。");
    };

    let base_url = read_optional("Base URL（可选，回车跳过）: ")?;
    let notes = read_optional("备注 (notes，可选，回车跳过）: ")?;

    // 写入 DB + Keychain
    let id = Uuid::new_v4().to_string();
    let preview = SecureStore::key_preview(&secret);
    let now = chrono::Utc::now();

    SecureStore::store_key(&id, &secret)
        .map_err(|e| anyhow::anyhow!("写入 Keychain 失败: {e}"))?;

    let platform_key = serde_json::to_string(&platform)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    db.execute(
        "INSERT INTO api_keys (id, name, platform, base_url, key_hash, key_preview, status, notes, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'unknown', ?7, ?8)",
        &[
            &id,
            &name,
            &platform_key,
            &base_url as &dyn rusqlite::ToSql,
            &"placeholder",
            &preview,
            &notes as &dyn rusqlite::ToSql,
            &now.to_rfc3339(),
        ],
    )
    .map_err(|e| anyhow::anyhow!("写入数据库失败: {e}"))?;

    println!("\n✓ Key 已添加");
    println!("  ID      : {id}");
    println!("  名称    : {name}");
    println!("  平台    : {platform_key}");
    println!("  预览    : {preview}");
    if let Some(ref url) = base_url {
        println!("  Base URL: {url}");
    }
    println!("\n提示：可运行 `ais key check {id}` 立即验证 Key 有效性。");
    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════
// provider
// ════════════════════════════════════════════════════════════════════════════

fn run_provider(action: ProviderAction, data_dir: &PathBuf) -> anyhow::Result<()> {
    let db = open_db(data_dir)?;
    match action {
        ProviderAction::List => list_providers(&db),
        ProviderAction::Switch { id } => switch_provider(&db, &id),
    }
}

fn list_providers(db: &Database) -> anyhow::Result<()> {
    let providers = ProviderService::new(db)
        .list_providers()
        .map_err(|e| anyhow::anyhow!("读取 providers 失败: {e}"))?;

    if providers.is_empty() {
        println!("(没有任何 Provider)");
        return Ok(());
    }

    println!(
        "{:<38}  {:<6}  {:<24}  {:<12}  {}",
        "ID", "Active", "Name", "Platform", "Targets"
    );
    println!("{}", "-".repeat(110));
    for p in providers {
        let platform_str = serde_json::to_string(&p.platform)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();
        println!(
            "{:<38}  {:<6}  {:<24.24}  {:<12}  {}",
            p.id,
            if p.is_active { "✓" } else { "" },
            p.name,
            platform_str,
            p.tool_targets.as_deref().unwrap_or("-"),
        );
    }
    Ok(())
}

fn switch_provider(db: &Database, id: &str) -> anyhow::Result<()> {
    ProviderService::new(db)
        .switch_provider(id)
        .map_err(|e| anyhow::anyhow!("切换失败: {e}"))?;
    println!("Provider {id} 已激活并完成跨工具同步。");
    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════
// proxy
// ════════════════════════════════════════════════════════════════════════════

fn run_proxy(action: ProxyAction, data_dir: &PathBuf) -> anyhow::Result<()> {
    match action {
        ProxyAction::Status { port } => proxy_status(port),
        ProxyAction::Start { port } => proxy_start(port, data_dir),
        ProxyAction::Stop => proxy_stop(data_dir),
    }
}

fn proxy_status(port: u16) -> anyhow::Result<()> {
    use std::net::TcpStream;
    use std::time::Duration;

    let addr = format!("127.0.0.1:{port}");
    let probe = TcpStream::connect_timeout(
        &addr.parse().map_err(|e| anyhow::anyhow!("无效地址: {e}"))?,
        Duration::from_millis(500),
    );
    match probe {
        Ok(_) => println!("代理正在运行 — http://{addr}/v1"),
        Err(_) => println!("代理未在 127.0.0.1:{port} 监听（请先运行 `ais proxy start`）"),
    }
    Ok(())
}

fn pid_file_path(data_dir: &PathBuf) -> PathBuf {
    data_dir.join("proxy.pid")
}

fn proxy_start(port: u16, data_dir: &PathBuf) -> anyhow::Result<()> {
    let pid_path = pid_file_path(data_dir);

    // 检查是否已有运行中的进程
    if pid_path.exists() {
        if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
            let pid_str = pid_str.trim();
            if !pid_str.is_empty() {
                println!("警告：检测到已有 PID 文件（PID={pid_str}），若代理已停止请先运行 `ais proxy stop` 清理。");
            }
        }
    }

    // 获取当前可执行文件路径，以子进程方式启动守护进程
    let exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("无法获取当前可执行文件路径: {e}"))?;

    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("--proxy-daemon")
        .arg("--port")
        .arg(port.to_string());

    // 传递 data_dir（若非默认）
    if let Some(ref dir) = None::<PathBuf> {
        // data_dir 已在 run() 中解析，此处守护进程使用相同默认路径
        cmd.arg("--data-dir").arg(dir);
    }

    // 在 Windows 上使用 CREATE_NO_WINDOW 标志，避免弹出控制台窗口
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    // 将 stdout/stderr 重定向到 null，使其真正后台运行
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null());

    let child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("启动代理守护进程失败: {e}"))?;

    let pid = child.id();

    // 写入 PID 文件
    std::fs::write(&pid_path, pid.to_string())
        .map_err(|e| anyhow::anyhow!("写入 PID 文件失败 ({}): {e}", pid_path.display()))?;

    println!("代理守护进程已启动 (PID={pid})，监听端口 {port}。");
    println!("PID 文件：{}", pid_path.display());
    println!("使用 `ais proxy status --port {port}` 确认代理已就绪。");
    Ok(())
}

fn proxy_stop(data_dir: &PathBuf) -> anyhow::Result<()> {
    let pid_path = pid_file_path(data_dir);

    if !pid_path.exists() {
        anyhow::bail!(
            "未找到 PID 文件 ({})，代理可能未在运行。",
            pid_path.display()
        );
    }

    let pid_str = std::fs::read_to_string(&pid_path)
        .map_err(|e| anyhow::anyhow!("读取 PID 文件失败: {e}"))?;
    let pid_str = pid_str.trim();

    if pid_str.is_empty() {
        std::fs::remove_file(&pid_path).ok();
        anyhow::bail!("PID 文件为空，已清理。");
    }

    let pid: u32 = pid_str
        .parse()
        .map_err(|_| anyhow::anyhow!("PID 文件内容无效: \"{pid_str}\""))?;

    // Windows：使用 taskkill /F /PID
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output()
            .map_err(|e| anyhow::anyhow!("执行 taskkill 失败: {e}"))?;

        if output.status.success() {
            println!("代理进程 (PID={pid}) 已终止。");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("taskkill 返回错误：{stderr}");
            eprintln!("（进程可能已不存在，继续清理 PID 文件）");
        }
    }

    // Unix：使用 kill -TERM
    #[cfg(not(target_os = "windows"))]
    {
        let output = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output()
            .map_err(|e| anyhow::anyhow!("执行 kill 失败: {e}"))?;

        if output.status.success() {
            println!("代理进程 (PID={pid}) 已发送 SIGTERM。");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("kill 返回错误：{stderr}");
            eprintln!("（进程可能已不存在，继续清理 PID 文件）");
        }
    }

    std::fs::remove_file(&pid_path)
        .map_err(|e| anyhow::anyhow!("删除 PID 文件失败: {e}"))?;
    println!("PID 文件已清理：{}", pid_path.display());
    Ok(())
}

/// 代理守护进程主循环（由 proxy start 以子进程方式调用）
fn run_proxy_daemon(port: u16) -> ExitCode {
    let data_dir = default_data_dir();
    let db_path = data_dir.join("data.db");

    let db = match Database::new(&db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("[proxy-daemon] 打开数据库失败 ({}): {e}", db_path.display());
            return ExitCode::FAILURE;
        }
    };

    let db_arc = std::sync::Arc::new(db);
    let server = ProxyServer::new(db_arc, port);

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("[proxy-daemon] 创建 Tokio 运行时失败: {e}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = runtime.block_on(server.start()) {
        eprintln!("[proxy-daemon] 代理服务异常退出: {e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

// ════════════════════════════════════════════════════════════════════════════
// balance
// ════════════════════════════════════════════════════════════════════════════

fn run_balance(data_dir: &PathBuf, platform_filter: Option<&str>) -> anyhow::Result<()> {
    let db = open_db(data_dir)?;
    let tracker = BalanceTracker::new(&db);
    let summaries = tracker
        .get_summaries()
        .map_err(|e| anyhow::anyhow!("读取余额失败: {e}"))?;

    let summaries: Vec<_> = if let Some(pf) = platform_filter {
        let pf_lower = pf.to_lowercase();
        summaries
            .into_iter()
            .filter(|s| s.platform.to_lowercase().contains(&pf_lower)
                || s.provider_name.to_lowercase().contains(&pf_lower))
            .collect()
    } else {
        summaries
    };

    if summaries.is_empty() {
        println!("(暂无余额记录，可在桌面端刷新余额后再查询)");
        return Ok(());
    }

    println!(
        "{:<38}  {:<24}  {:<12}  {:>12}  {:>12}  {}",
        "ProviderID", "Name", "Platform", "USD", "CNY", "LastUpdated"
    );
    println!("{}", "-".repeat(130));

    for s in &summaries {
        let usd = s
            .latest_balance_usd
            .map(|v| format!("${:.4}", v))
            .unwrap_or_else(|| "-".to_string());
        let cny = s
            .latest_balance_cny
            .map(|v| format!("¥{:.4}", v))
            .unwrap_or_else(|| "-".to_string());
        let updated = s
            .last_updated
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:<38}  {:<24.24}  {:<12}  {:>12}  {:>12}  {}",
            s.provider_id, s.provider_name, s.platform, usd, cny, updated
        );
    }

    // 汇总行
    let total_usd: f64 = summaries.iter().filter_map(|s| s.latest_balance_usd).sum();
    let total_cny: f64 = summaries.iter().filter_map(|s| s.latest_balance_cny).sum();
    println!("{}", "-".repeat(130));
    println!(
        "{:<38}  {:<24}  {:<12}  {:>12}  {:>12}",
        "合计",
        "",
        "",
        if total_usd > 0.0 { format!("${:.4}", total_usd) } else { "-".to_string() },
        if total_cny > 0.0 { format!("¥{:.4}", total_cny) } else { "-".to_string() },
    );
    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════
// route
// ════════════════════════════════════════════════════════════════════════════

fn run_route(action: RouteAction, data_dir: &PathBuf) -> anyhow::Result<()> {
    let db = open_db(data_dir)?;
    match action {
        RouteAction::List => list_routes(&db),
    }
}

fn list_routes(db: &Database) -> anyhow::Result<()> {
    let service = ModelMappingService::new(db);
    let mappings = service
        .get_all()
        .map_err(|e| anyhow::anyhow!("读取 model_mappings 失败: {e}"))?;

    if mappings.is_empty() {
        println!("(没有任何模型路由映射规则)");
        return Ok(());
    }

    println!(
        "{:<38}  {:<6}  {:<36}  {}",
        "ID", "Active", "SourceModel", "TargetModel"
    );
    println!("{}", "-".repeat(110));
    for m in mappings {
        println!(
            "{:<38}  {:<6}  {:<36.36}  {}",
            m.id,
            if m.is_active { "✓" } else { "" },
            m.source_model,
            m.target_model,
        );
    }
    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════
// mcp
// ════════════════════════════════════════════════════════════════════════════

fn run_mcp(action: McpAction, data_dir: &PathBuf) -> anyhow::Result<()> {
    let db = open_db(data_dir)?;
    match action {
        McpAction::List => list_mcps(&db),
        McpAction::Add { name, command, args, env } => add_mcp(&db, name, command, args, env),
    }
}

fn list_mcps(db: &Database) -> anyhow::Result<()> {
    let service = McpService::new(db);
    let mcps = service
        .list_mcps()
        .map_err(|e| anyhow::anyhow!("读取 mcp_servers 失败: {e}"))?;

    if mcps.is_empty() {
        println!("(没有任何 MCP Server)");
        return Ok(());
    }

    println!(
        "{:<38}  {:<6}  {:<24}  {:<20}  {:<20}  {}",
        "ID", "Active", "Name", "Command", "Args", "Env"
    );
    println!("{}", "-".repeat(130));
    for m in mcps {
        println!(
            "{:<38}  {:<6}  {:<24.24}  {:<20.20}  {:<20.20}  {}",
            m.id,
            if m.is_active { "✓" } else { "" },
            m.name,
            m.command,
            m.args.as_deref().unwrap_or("-"),
            m.env.as_deref().unwrap_or("-"),
        );
    }
    Ok(())
}

fn add_mcp(
    db: &Database,
    name: String,
    command: String,
    args: Option<String>,
    env: Option<String>,
) -> anyhow::Result<()> {
    // 验证 args/env 是合法 JSON（若提供）
    if let Some(ref a) = args {
        serde_json::from_str::<serde_json::Value>(a)
            .map_err(|e| anyhow::anyhow!("--args 不是合法 JSON: {e}"))?;
    }
    if let Some(ref e) = env {
        serde_json::from_str::<serde_json::Value>(e)
            .map_err(|e| anyhow::anyhow!("--env 不是合法 JSON: {e}"))?;
    }

    let now = chrono::Utc::now();
    let mcp = McpServer {
        id: Uuid::new_v4().to_string(),
        name: name.clone(),
        command: command.clone(),
        args: args.clone(),
        env: env.clone(),
        description: None,
        is_active: true,
        tool_targets: None,
        created_at: now,
        updated_at: now,
    };

    let id = mcp.id.clone();
    McpService::new(db)
        .add_mcp(mcp)
        .map_err(|e| anyhow::anyhow!("添加 MCP Server 失败: {e}"))?;

    println!("✓ MCP Server 已添加");
    println!("  ID     : {id}");
    println!("  名称   : {name}");
    println!("  命令   : {command}");
    if let Some(ref a) = args {
        println!("  Args   : {a}");
    }
    if let Some(ref e) = env {
        println!("  Env    : {e}");
    }
    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════
// model
// ════════════════════════════════════════════════════════════════════════════

fn run_model(action: ModelAction, data_dir: &PathBuf) -> anyhow::Result<()> {
    let db = open_db(data_dir)?;
    match action {
        ModelAction::List { platform } => list_models(&db, platform.as_deref()),
        ModelAction::Compare { model1, model2 } => compare_models(&db, &model1, &model2),
        ModelAction::Speedtest { platform } => speedtest_models(platform.as_deref()),
    }
}

fn list_models(db: &Database, platform_filter: Option<&str>) -> anyhow::Result<()> {
    let service = ModelCatalogService::new(db);

    let models = if let Some(pf) = platform_filter {
        let platform = serde_json::from_str::<Platform>(&format!("\"{pf}\""))
            .map_err(|_| anyhow::anyhow!("未识别的平台：\"{pf}\"，请使用 snake_case 格式（如 open_ai）"))?;
        service
            .get_platform_models(&platform)
            .map_err(|e| anyhow::anyhow!("读取模型目录失败: {e}"))?
    } else {
        service
            .list_models()
            .map_err(|e| anyhow::anyhow!("读取模型目录失败: {e}"))?
    };

    if models.is_empty() {
        println!("(没有找到匹配的模型)");
        return Ok(());
    }

    println!(
        "{:<36}  {:<14}  {:>8}  {:>8}  {:>8}  {:>8}  {:>8}  {:>8}  {}",
        "ModelID", "Platform", "Ctx(K)", "Vision", "Tools",
        "In$/1M", "Out$/1M", "Currency", "Source"
    );
    println!("{}", "-".repeat(140));

    for m in &models {
        let platform_str = serde_json::to_string(&m.platform)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();
        let ctx = m
            .context_length
            .map(|c| format!("{}", c / 1000))
            .unwrap_or_else(|| "-".to_string());
        let input = m
            .input_price_per_1m
            .map(|v| format!("{:.3}", v))
            .unwrap_or_else(|| "-".to_string());
        let output = m
            .output_price_per_1m
            .map(|v| format!("{:.3}", v))
            .unwrap_or_else(|| "-".to_string());
        let currency = m.pricing_currency.as_deref().unwrap_or("-");
        let source = m.pricing_source.as_deref().unwrap_or("-");

        println!(
            "{:<36.36}  {:<14}  {:>8}  {:>8}  {:>8}  {:>8}  {:>8}  {:>8}  {}",
            m.id,
            platform_str,
            ctx,
            if m.supports_vision { "✓" } else { "" },
            if m.supports_tools { "✓" } else { "" },
            input,
            output,
            currency,
            source,
        );
    }
    println!("\n共 {} 个模型", models.len());
    Ok(())
}

fn compare_models(db: &Database, model1_id: &str, model2_id: &str) -> anyhow::Result<()> {
    let service = ModelCatalogService::new(db);
    let all = service
        .list_models()
        .map_err(|e| anyhow::anyhow!("读取模型目录失败: {e}"))?;

    let m1_id_lower = model1_id.to_lowercase();
    let m2_id_lower = model2_id.to_lowercase();

    let m1 = all
        .iter()
        .find(|m| m.id.to_lowercase() == m1_id_lower || m.name.to_lowercase() == m1_id_lower)
        .ok_or_else(|| anyhow::anyhow!("未找到模型：{model1_id}"))?;
    let m2 = all
        .iter()
        .find(|m| m.id.to_lowercase() == m2_id_lower || m.name.to_lowercase() == m2_id_lower)
        .ok_or_else(|| anyhow::anyhow!("未找到模型：{model2_id}"))?;

    let p1 = serde_json::to_string(&m1.platform)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();
    let p2 = serde_json::to_string(&m2.platform)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    println!("\n{:<28}  {:<28}  {:<28}", "属性", &m1.id, &m2.id);
    println!("{}", "-".repeat(90));

    let row = |label: &str, v1: &str, v2: &str| {
        println!("{:<28}  {:<28}  {:<28}", label, v1, v2);
    };

    row("平台", &p1, &p2);
    row(
        "上下文长度",
        &m1.context_length.map(|c| format!("{} tokens", c)).unwrap_or_else(|| "-".to_string()),
        &m2.context_length.map(|c| format!("{} tokens", c)).unwrap_or_else(|| "-".to_string()),
    );
    row(
        "支持视觉",
        if m1.supports_vision { "✓" } else { "✗" },
        if m2.supports_vision { "✓" } else { "✗" },
    );
    row(
        "支持工具调用",
        if m1.supports_tools { "✓" } else { "✗" },
        if m2.supports_tools { "✓" } else { "✗" },
    );

    let fmt_price = |p: Option<f64>, currency: Option<&str>| {
        p.map(|v| format!("{:.4} {}", v, currency.unwrap_or("USD")))
            .unwrap_or_else(|| "-".to_string())
    };

    row(
        "输入价格 /1M tokens",
        &fmt_price(m1.input_price_per_1m, m1.pricing_currency.as_deref()),
        &fmt_price(m2.input_price_per_1m, m2.pricing_currency.as_deref()),
    );
    row(
        "输出价格 /1M tokens",
        &fmt_price(m1.output_price_per_1m, m1.pricing_currency.as_deref()),
        &fmt_price(m2.output_price_per_1m, m2.pricing_currency.as_deref()),
    );
    row(
        "固定价格 /请求",
        &fmt_price(m1.fixed_price, m1.pricing_currency.as_deref()),
        &fmt_price(m2.fixed_price, m2.pricing_currency.as_deref()),
    );
    row(
        "价格来源",
        m1.pricing_source.as_deref().unwrap_or("-"),
        m2.pricing_source.as_deref().unwrap_or("-"),
    );
    if let (Some(n1), Some(n2)) = (&m1.pricing_note, &m2.pricing_note) {
        row("价格备注", n1, n2);
    } else if m1.pricing_note.is_some() || m2.pricing_note.is_some() {
        row(
            "价格备注",
            m1.pricing_note.as_deref().unwrap_or("-"),
            m2.pricing_note.as_deref().unwrap_or("-"),
        );
    }

    // 简单性价比对比（仅当两者均有输入价格时）
    if let (Some(i1), Some(i2)) = (m1.input_price_per_1m, m2.input_price_per_1m) {
        println!("\n── 性价比参考 ──");
        if i1 < i2 {
            println!(
                "  {} 输入价格更低（{:.4} vs {:.4}，便宜 {:.1}%）",
                m1.id,
                i1,
                i2,
                (i2 - i1) / i2 * 100.0
            );
        } else if i2 < i1 {
            println!(
                "  {} 输入价格更低（{:.4} vs {:.4}，便宜 {:.1}%）",
                m2.id,
                i2,
                i1,
                (i1 - i2) / i1 * 100.0
            );
        } else {
            println!("  两者输入价格相同。");
        }
    }

    Ok(())
}

fn speedtest_models(platform_filter: Option<&str>) -> anyhow::Result<()> {
    use ai_singularity_lib::services::speedtest::KNOWN_ENDPOINTS;

    let endpoints: Vec<_> = if let Some(pf) = platform_filter {
        let pf_lower = pf.to_lowercase();
        KNOWN_ENDPOINTS
            .iter()
            .filter(|(key, name, _)| {
                key.to_lowercase().contains(&pf_lower)
                    || name.to_lowercase().contains(&pf_lower)
            })
            .collect()
    } else {
        KNOWN_ENDPOINTS.iter().collect()
    };

    if endpoints.is_empty() {
        anyhow::bail!("未找到匹配平台 \"{platform_filter:?}\" 的 endpoint。");
    }

    println!("正在测速 {} 个 endpoint，请稍候...\n", endpoints.len());

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let results = runtime.block_on(async {
        let futures: Vec<_> = endpoints
            .iter()
            .map(|(key, name, endpoint)| {
                SpeedTestService::test_endpoint(key, name, endpoint)
            })
            .collect();
        futures::future::join_all(futures).await
    });

    println!(
        "{:<14}  {:<28}  {:>10}  {}",
        "Platform", "Endpoint", "Latency", "Status"
    );
    println!("{}", "-".repeat(80));

    let mut sorted = results;
    sorted.sort_by(|a, b| {
        match (a.latency_ms, b.latency_ms) {
            (Some(la), Some(lb)) => la.cmp(&lb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    for r in &sorted {
        let latency = r
            .latency_ms
            .map(|ms| format!("{ms}ms"))
            .unwrap_or_else(|| "-".to_string());
        let endpoint_short = if r.endpoint.len() > 40 {
            format!("{}...", &r.endpoint[..37])
        } else {
            r.endpoint.clone()
        };
        println!(
            "{:<14}  {:<40.40}  {:>10}  {}",
            r.platform, endpoint_short, latency, r.status
        );
    }

    // 最快 endpoint 提示
    if let Some(fastest) = sorted.iter().find(|r| r.latency_ms.is_some()) {
        println!(
            "\n最快：{} ({}ms) — {}",
            fastest.platform,
            fastest.latency_ms.unwrap_or(0),
            fastest.endpoint
        );
    }

    Ok(())
}
