use std::process::Command;
use crate::error::AppResult;

pub struct SandboxManager;

impl SandboxManager {
    /// 拉起一个完全隔离在本地代理环境中的终极子网络沙盒靶场，目标工具启动时只会认 AI Singularity。
    pub fn launch_tool_sandboxed(
        target_tool: &str,
        proxy_port: u16,
    ) -> AppResult<()> {
        let proxy_url = format!("http://127.0.0.1:{}", proxy_port);

        // 构建 Windows 原生 CMD 启动令
        let mut cmd = Command::new("cmd");
        cmd.arg("/c").arg("start").arg("cmd").arg("/k");

        // 给沙盒配置骇客级接管环境变量，由于跨平台我们先把 Windows 做了。
        let init_script = format!(
            "title AI Singularity [Sandbox - {}] && \
             set HTTP_PROXY={} && \
             set HTTPS_PROXY={} && \
             set ALL_PROXY={} && \
             set ANTHROPIC_API_KEY=sk-ant-api03-sandbox-dummy-bypass-key-12345 && \
             set OPENAI_API_KEY=sk-proj-sandbox-dummy-bypass-key-12345 && \
             echo ============================================================ && \
             echo [AI Singularity] 战地重装机甲正在掩护... && \
             echo [AI Singularity] 环境变量已锁定。所有流量被强行导向代理舱。 && \
             echo [AI Singularity] 自动凭证已挂载 (sk-sandbox-dummy...级) && \
             echo ============================================================ && \
             {}",
            target_tool, proxy_url, proxy_url, proxy_url, target_tool
        );
        cmd.arg(&init_script);

        tracing::info!("🚀 正在呼叫近地轨道打击，开启原生态沙盒舱：{}", target_tool);
        
        let status = cmd.spawn()?;
        tracing::debug!("沙盒宿主进程号启动 PID: {}", status.id());

        Ok(())
    }
}
