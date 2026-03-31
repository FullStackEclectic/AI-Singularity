use std::path::Path;
use std::io::Write;
use anyhow::Result;

/// 原子写入：先写入临时文件，再 rename 替换原文件
/// 防止写入过程中崩溃导致配置文件损坏
pub fn atomic_write(target: &Path, content: &[u8]) -> Result<()> {
    let parent = target.parent().ok_or_else(|| {
        anyhow::anyhow!("目标文件没有父目录: {:?}", target)
    })?;

    // 创建同目录的临时文件
    let tmp_path = parent.join(format!(
        ".tmp_{}_{}",
        target.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));

    // 写入临时文件
    {
        let mut tmp_file = std::fs::File::create(&tmp_path)?;
        tmp_file.write_all(content)?;
        tmp_file.flush()?;
        // 注意：File 离开作用域时会自动 close/sync
    }

    // 原子替换（同卷 rename 是原子操作）
    std::fs::rename(&tmp_path, target)?;

    Ok(())
}

/// 原子写入 JSON 对象
pub fn atomic_write_json<T: serde::Serialize>(target: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_vec_pretty(value)?;
    atomic_write(target, &json)
}

/// 备份轮换：保留最近 N 份备份
pub fn rotate_backup(target: &Path, keep: usize) -> Result<()> {
    let parent = target.parent().unwrap_or(Path::new("."));
    let filename = target.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let backup_dir = parent.join("backups");

    std::fs::create_dir_all(&backup_dir)?;

    // 生成带时间戳的备份文件名
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_path = backup_dir.join(format!("{}.{}.bak", filename, timestamp));

    if target.exists() {
        std::fs::copy(target, &backup_path)?;
    }

    // 清理超出 keep 数量的旧备份（按文件名排序，删最旧的）
    let mut backups: Vec<_> = std::fs::read_dir(&backup_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.starts_with(filename) && n.ends_with(".bak"))
                .unwrap_or(false)
        })
        .collect();

    backups.sort_by_key(|e| e.file_name());

    // 删除超出保留数量的最旧备份
    if backups.len() > keep {
        for old in &backups[..backups.len() - keep] {
            let _ = std::fs::remove_file(old.path());
        }
    }

    Ok(())
}
