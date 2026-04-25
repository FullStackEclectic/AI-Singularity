//! AI Singularity CLI — 与桌面端共享 Core Engine。
//!
//! 设计原则：
//!   - 复用 ai_singularity_lib 的 services 与 db 层（不引入 Tauri 运行时）
//!   - 数据目录通过 dirs::data_dir() + bundle id 自行解析，与桌面端 tauri 默认一致
//!   - Keychain 通过 keyring 跨进程访问

use std::path::PathBuf;
use std::process::ExitCode;

use ai_singularity_lib::db::Database;
use ai_singularity_lib::models::Platform;
use ai_singularity_lib::services::provider::ProviderService;
use ai_singularity_lib::services::validator;
use ai_singularity_lib::store::SecureStore;
use clap::{Parser, Subcommand};

const BUNDLE_ID: &str = "com.ai-singularity.app";
const DEFAULT_PROXY_PORT: u16 = 8765;

#[derive(Parser)]
#[command(
    name = "ais",
    version,
    about = "AI Singularity CLI — AI 资源统一管理控制台",
    long_about = "复用桌面端的本地数据库与 Keychain，可在终端中直接管理 Key/Provider/Proxy。"
)]
struct Cli {
    /// 显式指定数据目录（默认与桌面端共享 OS 标准路径）
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
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
    /// 本地代理状态查询
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },
}

#[derive(Subcommand)]
enum KeyAction {
    /// 列出所有已录入的 Key
    List,
    /// 检测某个 Key 当前的可用性
    Check {
        /// Key ID（来自 `ais key list`）
        id: String,
    },
}

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

#[derive(Subcommand)]
enum ProxyAction {
    /// 探活本地代理端口（默认 8765）
    Status {
        #[arg(long, default_value_t = DEFAULT_PROXY_PORT)]
        port: u16,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
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

    match cli.command {
        Command::Key { action } => run_key(action, &data_dir),
        Command::Provider { action } => run_provider(action, &data_dir),
        Command::Proxy { action } => run_proxy(action),
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

// ────────────────────────────────────────────────────────────────────────
// key
// ────────────────────────────────────────────────────────────────────────

fn run_key(action: KeyAction, data_dir: &PathBuf) -> anyhow::Result<()> {
    let db = open_db(data_dir)?;
    match action {
        KeyAction::List => list_keys(&db),
        KeyAction::Check { id } => check_key(&db, &id),
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
        println!("(没有任何 Key，可在桌面端添加)");
        return Ok(());
    }

    println!(
        "{:<38}  {:<20}  {:<14}  {:<10}  {:<14}  {}",
        "ID", "Name", "Platform", "Status", "LastChecked", "Preview"
    );
    println!("{}", "-".repeat(120));
    for (id, name, platform, status, preview, last_checked) in rows {
        println!(
            "{:<38}  {:<20.20}  {:<14}  {:<10}  {:<14}  {}",
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

// ────────────────────────────────────────────────────────────────────────
// provider
// ────────────────────────────────────────────────────────────────────────

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

// ────────────────────────────────────────────────────────────────────────
// proxy
// ────────────────────────────────────────────────────────────────────────

fn run_proxy(action: ProxyAction) -> anyhow::Result<()> {
    match action {
        ProxyAction::Status { port } => proxy_status(port),
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
        Err(_) => println!("代理未在 127.0.0.1:{port} 监听（请先在桌面端启动）"),
    }
    Ok(())
}
