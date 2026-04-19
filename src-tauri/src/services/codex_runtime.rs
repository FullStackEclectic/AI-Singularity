use std::path::Path;
use std::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

pub fn is_pid_running(pid: u32) -> bool {
    let mut system = sysinfo::System::new_all();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    system.process(sysinfo::Pid::from_u32(pid)).is_some()
}

#[cfg(target_os = "windows")]
fn ps_single_quote(input: &str) -> String {
    input.replace('\'', "''")
}

#[cfg(target_os = "windows")]
fn parse_extra_args(raw: &str) -> Vec<String> {
    raw.split_whitespace()
        .map(|item| item.to_string())
        .collect()
}

#[cfg(target_os = "windows")]
fn resolve_codex_executable() -> Result<String, String> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-Command codex -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Source)",
        ])
        .output()
        .map_err(|e| format!("探测 Codex 可执行文件失败: {}", e))?;

    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !value.is_empty() {
            return Ok(value);
        }
    }

    let where_output = Command::new("where.exe")
        .arg("codex")
        .output()
        .map_err(|e| format!("where codex 执行失败: {}", e))?;
    if where_output.status.success() {
        let first = String::from_utf8_lossy(&where_output.stdout)
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string());
        if let Some(first) = first {
            return Ok(first);
        }
    }

    Err("未找到 Codex 可执行文件。请确认 codex 已安装并已加入 PATH。".to_string())
}

#[cfg(target_os = "windows")]
pub fn start_codex_instance(user_data_dir: &str, extra_args: &str) -> Result<u32, String> {
    let exe = resolve_codex_executable()?;
    let args = parse_extra_args(extra_args);
    let args_ps = if args.is_empty() {
        "@()".to_string()
    } else {
        format!(
            "@({})",
            args.iter()
                .map(|item| format!("'{}'", ps_single_quote(item)))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let script = format!(
        "$ErrorActionPreference='Stop'; \
         $env:CODEX_HOME='{code_home}'; \
         $p = Start-Process -FilePath '{exe}' -WorkingDirectory '{cwd}' -ArgumentList {args} -PassThru; \
         [Console]::Out.Write($p.Id)",
        code_home = ps_single_quote(user_data_dir),
        exe = ps_single_quote(&exe),
        cwd = ps_single_quote(user_data_dir),
        args = args_ps,
    );

    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("启动 Codex 实例失败: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "启动 Codex 实例失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let pid = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .map_err(|e| format!("解析 Codex 进程 PID 失败: {}", e))?;

    Ok(pid)
}

#[cfg(not(target_os = "windows"))]
pub fn start_codex_instance(_user_data_dir: &str, _extra_args: &str) -> Result<u32, String> {
    Err("当前版本仅在 Windows 上支持 Codex 实例运行时控制".to_string())
}

#[cfg(target_os = "windows")]
pub fn stop_pid(pid: u32) -> Result<(), String> {
    let output = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("停止 Codex 进程失败: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "停止 Codex 进程失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn stop_pid(_pid: u32) -> Result<(), String> {
    Err("当前版本仅在 Windows 上支持 Codex 实例运行时控制".to_string())
}

#[cfg(target_os = "windows")]
pub fn focus_pid(pid: u32) -> Result<(), String> {
    let script = format!(
        "$wshell = New-Object -ComObject WScript.Shell; \
         if ($wshell.AppActivate({pid})) {{ [Console]::Out.Write('ok') }} else {{ exit 1 }}",
        pid = pid
    );
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("定位 Codex 窗口失败: {}", e))?;

    if !output.status.success() {
        return Err("无法将 Codex 实例窗口切到前台，可能当前实例未运行。".to_string());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn focus_pid(_pid: u32) -> Result<(), String> {
    Err("当前版本仅在 Windows 上支持 Codex 实例运行时控制".to_string())
}

pub fn validate_user_data_dir(path: &str) -> Result<(), String> {
    let target = Path::new(path);
    if !target.exists() {
        return Err(format!("实例目录不存在: {}", target.display()));
    }
    if !target.join("state_5.sqlite").exists() {
        return Err(format!("实例目录缺少 state_5.sqlite: {}", target.display()));
    }
    Ok(())
}
