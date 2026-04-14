use chrono::Local;
use std::backtrace::Backtrace;
use std::fs;
use std::io::Write;
use std::panic::PanicHookInfo;
use std::path::PathBuf;

pub fn set_panic_hook(app_data_dir: PathBuf) {
    // 获取原生框架注册的默认崩溃钩子（比如控制台打印）
    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info: &PanicHookInfo<'_>| {
        // 先调用默认控制台打印
        default_hook(panic_info);

        let logs_dir = app_data_dir.join("logs");
        if !logs_dir.exists() {
            let _ = fs::create_dir_all(&logs_dir);
        }

        let crash_file_path = logs_dir.join(format!(
            "crash_{}.log",
            Local::now().format("%Y%m%d_%H%M%S")
        ));

        let mut crash_log = String::new();
        crash_log.push_str("================ AI Singularity Crash Report ================\n");
        crash_log.push_str(&format!(
            "Time: {}\n",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        ));
        crash_log.push_str(&format!("OS: {}\n", std::env::consts::OS));
        crash_log.push_str(&format!("Arch: {}\n", std::env::consts::ARCH));

        // 强行扒出引起 Panic 闪退的真身消息
        let payload = panic_info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            *s
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.as_str()
        } else {
            "未知的不可恢复错误类型"
        };

        crash_log.push_str(&format!("\n[Error Reason]: {}\n", message));

        if let Some(location) = panic_info.location() {
            crash_log.push_str(&format!(
                "Location: {}:{}\n",
                location.file(),
                location.line()
            ));
        }

        // Rust 的堆栈倒推机制：这里是真正抓捕幽灵 Bug 的神技
        crash_log.push_str("\n--- Backtrace ---\n");
        let bt = Backtrace::force_capture();
        crash_log.push_str(&format!("{:#?}", bt));
        crash_log.push_str("\n============================================================\n");

        if let Ok(mut file) = fs::File::create(&crash_file_path) {
            let _ = file.write_all(crash_log.as_bytes());
            // 确保同步到底层磁盘柱面
            let _ = file.sync_all();
        }
    }));
}
