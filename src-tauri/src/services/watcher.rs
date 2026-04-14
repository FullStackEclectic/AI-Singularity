use dirs::home_dir;
use notify::event::ModifyKind;
use notify::{Event as NotifyEvent, EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

pub struct WatcherService;

impl WatcherService {
    /// 开启全局文件监听防篡改热更雷达
    pub fn start_watching(app_handle: AppHandle) {
        let (tx, mut rx) = mpsc::channel::<PathBuf>(100);

        // Notify Watcher 属于阻塞模型，抛入独立的 OS 线程死循环保活
        std::thread::spawn(move || {
            let mut watcher =
                notify::recommended_watcher(move |res: notify::Result<NotifyEvent>| {
                    if let Ok(event) = res {
                        // 我们只捕获实质性的「数据修改」动作，忽略 Access 等杂音
                        if matches!(event.kind, EventKind::Modify(ModifyKind::Data(_))) {
                            for path in event.paths {
                                let _ = tx.blocking_send(path);
                            }
                        }
                    }
                })
                .expect("初始化文件系统雷达引擎失败");

            let mut paths_to_watch = vec![];
            if let Some(home) = home_dir() {
                paths_to_watch.push(home.join(".claude.json")); // Claude Code 资源档
                paths_to_watch.push(home.join(".aider.conf.yml")); // Aider 资源档
                paths_to_watch.push(home.join(".gemini/settings.json")); // Gemini
            }

            for p in &paths_to_watch {
                // 如果用户连这个工具都没装文件还没生成，先替他 Touch 植入空架子以确立监听锚点
                if !p.exists() {
                    // 对于需要父目录的先创建嵌套夹
                    if let Some(parent) = p.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::File::create(p);
                }

                if let Err(e) = watcher.watch(p, RecursiveMode::NonRecursive) {
                    tracing::warn!("无法锁定监听目标 {:?} - {}", p, e);
                } else {
                    tracing::info!("🎯 锁定目标工具配置: {:?}", p);
                }
            }

            // 锁死当前系统线程，充当永远不死的系统守望者服务
            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        });

        // Tokio 轻量级协程捕捉事件并处理【防抖风暴过滤】
        tauri::async_runtime::spawn(async move {
            let mut last_processed = std::time::Instant::now() - Duration::from_secs(10);
            while let Some(changed_path) = rx.recv().await {
                let now = std::time::Instant::now();
                // 暴力连击打断：500 毫秒内的多次连续写入只认第一枪（或者使用更严密的 debounce 库也可，这里采取剥离式硬防抖）
                if now.duration_since(last_processed) < Duration::from_millis(500) {
                    continue;
                }
                last_processed = now;

                tracing::info!("⚡ 警报：核心生态配置被不明外力篡改: {:?}", changed_path);

                // 通过高速 IPC Hub 即刻打向所有的 React 渲染进程
                let _ = app_handle.emit(
                    "external_config_changed",
                    changed_path.to_string_lossy().to_string(),
                );
            }
        });
    }
}
