use tauri::{AppHandle, Emitter};
use tiny_http::{Server, Response};
use std::thread;
use url::Url;

pub struct OauthManager;

impl OauthManager {
    /// 启动本地监听端口，等待回调
    pub fn start_oauth_flow(app: AppHandle, provider: String) -> Result<String, String> {
        let port = 11223;
        
        // 由于安全原因我们应该避免端口被重复占用，
        // 如果端口被占用，tiny_http 会启动失败，此时可以重试一个随机端口
        let server = match Server::http(format!("127.0.0.1:{}", port)) {
            Ok(s) => s,
            Err(e) => return Err(format!("无法绑定端口: {}", e)),
        };
        
        // 构造云端授权 URL
        // 因为这是一个本地应用跨层授权的演示，此处内置一个 Github 应用的占位参数
        let client_id = if provider.to_lowercase() == "github copilot" {
            "Iv1.0123456789abcdef" // 实际需替换为您申请的 OAuth Client ID
        } else {
            "generic_client_id"
        };
        
        let auth_url = format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri=http://127.0.0.1:{}&scope=read:user",
            client_id, port
        );

        // 我们开启一条独立线程来接收并阻塞等待请求
        thread::spawn(move || {
            // tiny_http 服务器阻塞循环
            for request in server.incoming_requests() {
                let url_str = format!("http://localhost{}", request.url());
                if let Ok(url) = Url::parse(&url_str) {
                    let mut code = None;
                    for (k, v) in url.query_pairs() {
                        if k == "code" {
                            code = Some(v.into_owned());
                        }
                    }

                    if let Some(c) = code {
                        // 发送成功事件到前端的 Tauri Emitter
                        let _ = app.emit("oauth_success", c);
                        
                        let html = r#"
                        <html>
                            <head><meta charset="utf-8"></head>
                            <body>
                                <h2>🎉 授权成功！您可以关闭该标签页并返回 AI Singularity 了。</h2>
                                <script>setTimeout(() => window.close(), 3000);</script>
                            </body>
                        </html>
                        "#;
                        let response = Response::from_string(html)
                            .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=UTF-8"[..]).unwrap());
                        let _ = request.respond(response);
                        
                        // 拿到后即销毁服务
                        break;
                    }
                }
                let response = Response::from_string("等待回调参数 code...");
                let _ = request.respond(response);
            }
        });
        
        // 将产生的安全跳转 URL 发放给前端，由前端呼出浏览器
        Ok(auth_url)
    }
}
