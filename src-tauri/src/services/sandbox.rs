use crate::error::AppResult;
use std::process::Command;

pub struct SandboxManager;

impl SandboxManager {
    /// 拉起一个隔离在本地代理环境中的子终端，目标 CLI 工具启动时会强制走 AI Singularity 代理。
    /// 通过设置 HTTP_PROXY/HTTPS_PROXY/ALL_PROXY 与占位凭据，避免真实凭据泄露给目标工具。
    pub fn launch_tool_sandboxed(target_tool: &str, proxy_port: u16) -> AppResult<()> {
        let proxy_url = format!("http://127.0.0.1:{}", proxy_port);

        #[cfg(target_os = "windows")]
        let cmd = build_windows_command(target_tool, &proxy_url);

        #[cfg(target_os = "macos")]
        let cmd = build_macos_command(target_tool, &proxy_url);

        #[cfg(all(unix, not(target_os = "macos")))]
        let cmd = build_linux_command(target_tool, &proxy_url);

        tracing::info!("🚀 启动沙盒终端：{}", target_tool);
        let mut cmd = cmd;
        let child = cmd.spawn()?;
        tracing::debug!("沙盒宿主进程 PID: {}", child.id());

        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn build_windows_command(target_tool: &str, proxy_url: &str) -> Command {
    let init_script = format!(
        "title AI Singularity [Sandbox - {tool}] && \
         set HTTP_PROXY={proxy} && \
         set HTTPS_PROXY={proxy} && \
         set ALL_PROXY={proxy} && \
         set ANTHROPIC_API_KEY=sk-ant-api03-sandbox-dummy-bypass-key-12345 && \
         set OPENAI_API_KEY=sk-proj-sandbox-dummy-bypass-key-12345 && \
         echo ============================================================ && \
         echo [AI Singularity] Sandbox terminal active && \
         echo [AI Singularity] All HTTP traffic forced through {proxy} && \
         echo [AI Singularity] Dummy credentials injected && \
         echo ============================================================ && \
         {tool}",
        tool = target_tool,
        proxy = proxy_url,
    );

    let mut cmd = Command::new("cmd");
    cmd.arg("/c").arg("start").arg("cmd").arg("/k").arg(init_script);
    cmd
}

#[cfg(target_os = "macos")]
fn build_macos_command(target_tool: &str, proxy_url: &str) -> Command {
    // 用 osascript 让 Terminal.app 打开新窗口并执行带环境变量前缀的命令
    let inner = format!(
        "export HTTP_PROXY={proxy}; export HTTPS_PROXY={proxy}; export ALL_PROXY={proxy}; \
         export ANTHROPIC_API_KEY=sk-ant-api03-sandbox-dummy-bypass-key-12345; \
         export OPENAI_API_KEY=sk-proj-sandbox-dummy-bypass-key-12345; \
         echo '[AI Singularity] Sandbox terminal active'; \
         echo '[AI Singularity] All HTTP traffic forced through {proxy}'; \
         {tool}",
        tool = target_tool,
        proxy = proxy_url,
    );
    // osascript 内字符串需要转义双引号
    let escaped = inner.replace('"', "\\\"");
    let osa = format!("tell application \"Terminal\" to do script \"{}\"", escaped);

    let mut cmd = Command::new("osascript");
    cmd.arg("-e").arg(osa);
    cmd
}

#[cfg(all(unix, not(target_os = "macos")))]
fn build_linux_command(target_tool: &str, proxy_url: &str) -> Command {
    // 探测可用终端，按优先级回退
    let inner = format!(
        "export HTTP_PROXY={proxy}; export HTTPS_PROXY={proxy}; export ALL_PROXY={proxy}; \
         export ANTHROPIC_API_KEY=sk-ant-api03-sandbox-dummy-bypass-key-12345; \
         export OPENAI_API_KEY=sk-proj-sandbox-dummy-bypass-key-12345; \
         echo '[AI Singularity] Sandbox terminal active'; \
         echo '[AI Singularity] All HTTP traffic forced through {proxy}'; \
         {tool}; exec $SHELL",
        tool = target_tool,
        proxy = proxy_url,
    );

    let terminal = pick_linux_terminal();
    let mut cmd = Command::new(&terminal);
    match terminal.as_str() {
        "gnome-terminal" => {
            cmd.arg("--").arg("bash").arg("-c").arg(inner);
        }
        "konsole" => {
            cmd.arg("-e").arg("bash").arg("-c").arg(inner);
        }
        "xfce4-terminal" => {
            cmd.arg("--command").arg(format!("bash -c \"{}\"", inner.replace('"', "\\\"")));
        }
        _ => {
            // xterm / 默认回退
            cmd.arg("-e").arg("bash").arg("-c").arg(inner);
        }
    }
    cmd
}

#[cfg(all(unix, not(target_os = "macos")))]
fn pick_linux_terminal() -> String {
    use std::path::Path;
    for candidate in ["gnome-terminal", "konsole", "xfce4-terminal", "xterm"] {
        // 简易 PATH 探测
        if let Ok(path_env) = std::env::var("PATH") {
            for dir in path_env.split(':') {
                if Path::new(dir).join(candidate).exists() {
                    return candidate.to_string();
                }
            }
        }
    }
    "xterm".to_string()
}
