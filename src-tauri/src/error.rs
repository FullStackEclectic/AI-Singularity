use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("加密存储错误: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("HTTP 请求错误: {0}")]
    Http(#[from] reqwest::Error),

    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("平台不支持: {0}")]
    UnsupportedPlatform(String),

    #[error("API 密钥无效")]
    InvalidApiKey,

    #[error("账号被封禁 (403)")]
    AccountBanned,

    #[error("未知错误: {0}")]
    Other(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

// Tauri 命令要求错误类型实现 Serialize
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
