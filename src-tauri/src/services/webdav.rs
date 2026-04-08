use reqwest::{Client, Method, header};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct WebDavAuth {
    pub username: String,
    pub password: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WebDavConfig {
    pub url: String,
    pub username: String,
    pub password: Option<String>,
}

pub struct WebDavService {
    client: Client,
}

impl WebDavService {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    /// 发送一个轻量 PROPFIND 请求验证连接
    pub async fn test_connection(&self, config: &WebDavConfig) -> AppResult<()> {
        let req = self.client.request(Method::from_bytes(b"PROPFIND").unwrap(), &config.url)
            .header("Depth", "0")
            .basic_auth(&config.username, config.password.as_deref());
        
        let response = req.send().await.map_err(|e| AppError::Other(anyhow::anyhow!("WebDAV 连接失败: {}", e)))?;
        
        if response.status().is_client_error() || response.status().is_server_error() {
            if response.status() == 401 || response.status() == 403 {
                return Err(AppError::Other(anyhow::anyhow!("WebDAV 认证失败，请检查用户名和密码")));
            }
            return Err(AppError::Other(anyhow::anyhow!("WebDAV 错误状态码: {}", response.status())));
        }

        Ok(())
    }

    /// 上传备份文件，覆盖远端
    pub async fn push_backup(&self, config: &WebDavConfig, json_data: &str) -> AppResult<()> {
        // 先确保目录存在（此处简化为假定直接对 url 执行 PUT）
        // 实际可以将 ai_singularity_backup.json 加载 URL 结尾
        let target_url = if config.url.ends_with(".json") {
            config.url.clone()
        } else {
            let mut url = config.url.clone();
            if !url.ends_with('/') {
                url.push('/');
            }
            url.push_str("ai_singularity_backup.json");
            url
        };

        let req = self.client.put(&target_url)
            .basic_auth(&config.username, config.password.as_deref())
            .header(header::CONTENT_TYPE, "application/json")
            .body(json_data.to_string());
        
        let response = req.send().await.map_err(|e| AppError::Other(anyhow::anyhow!("WebDAV 上传失败: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(AppError::Other(anyhow::anyhow!("WebDAV 上传错误状态码: {}", response.status())));
        }

        Ok(())
    }

    /// 下载远端备份文件
    pub async fn pull_backup(&self, config: &WebDavConfig) -> AppResult<String> {
        let target_url = if config.url.ends_with(".json") {
            config.url.clone()
        } else {
            let mut url = config.url.clone();
            if !url.ends_with('/') {
                url.push('/');
            }
            url.push_str("ai_singularity_backup.json");
            url
        };

        let req = self.client.get(&target_url)
            .basic_auth(&config.username, config.password.as_deref());
        
        let response = req.send().await.map_err(|e| AppError::Other(anyhow::anyhow!("WebDAV 下载失败: {}", e)))?;
        
        if !response.status().is_success() {
            if response.status() == 404 {
                return Err(AppError::Other(anyhow::anyhow!("远端未找到备份文件")));
            }
            return Err(AppError::Other(anyhow::anyhow!("WebDAV 下载错误状态码: {}", response.status())));
        }

        let body = response.text().await.map_err(|e| AppError::Other(anyhow::anyhow!("读取 WebDAV 响应流失败: {}", e)))?;
        Ok(body)
    }
}
