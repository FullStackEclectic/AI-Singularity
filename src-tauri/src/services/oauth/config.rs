pub const ANTIGRAVITY_CLIENT_ID: &str =
    "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com";
pub const ANTIGRAVITY_CLIENT_SECRET_ENV: &str = "AIS_ANTIGRAVITY_CLIENT_SECRET";

pub const GEMINI_CLIENT_ID: &str =
    "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";
pub const GEMINI_CLIENT_SECRET_ENV: &str = "AIS_GEMINI_CLIENT_SECRET";

pub const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";
pub const GOOGLE_OAUTH_CALLBACK_PATH: &str = "/oauth2callback";
pub const GOOGLE_SCOPES: &str = "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile";

pub const CURSOR_LOGIN_URL: &str = "https://cursor.com/loginDeepControl";
pub const CURSOR_POLL_URL: &str = "https://api2.cursor.sh/auth/poll";

pub const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
pub const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
pub const GITHUB_CLIENT_ID: &str = "01ab8ac9400c4e429b23";
pub const GITHUB_SCOPE: &str = "read:user user:email";

pub const WINDSURF_CLIENT_ID: &str = "3GUryQ7ldAeKEuD2obYnppsnmj58eP5u";
pub const WINDSURF_AUTH_BASE_URL: &str = "https://www.windsurf.com";
pub const WINDSURF_REGISTER_API_URL: &str = "https://register.windsurf.com";
pub const WINDSURF_CALLBACK_PATH: &str = "/windsurf-auth-callback";

pub const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const CODEX_AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
pub const CODEX_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
pub const CODEX_SCOPES: &str = "openid profile email offline_access";
pub const CODEX_CALLBACK_PATH: &str = "/auth/callback";
pub const CODEX_CALLBACK_PORT: u16 = 1455;

pub const KIRO_AUTH_URL: &str = "https://app.kiro.dev/signin";
pub const KIRO_TOKEN_URL: &str = "https://prod.us-east-1.auth.desktop.kiro.dev/oauth/token";
pub const KIRO_CALLBACK_PATH: &str = "/oauth/callback";

pub const TRAE_AUTH_CLIENT_ID: &str = "ono9krqynydwx5";
pub const TRAE_CALLBACK_PATH: &str = "/authorize";
pub const TRAE_LOGIN_GUIDANCE_URL: &str =
    "https://api.marscode.com/cloudide/api/v3/trae/GetLoginGuidance";

pub const QODER_LOGIN_URL: &str = "https://qoder.com/device/selectAccounts";
pub const QODER_OPENAPI_URL: &str = "https://openapi.qoder.sh";
pub const QODER_CLIENT_ID: &str = "e883ade2-e6e3-4d6d-adf7-f92ceff5fdcb";
pub const QODER_POLL_PATH: &str = "/api/v1/deviceToken/poll";
pub const QODER_USERINFO_PATH: &str = "/api/v1/userinfo";

pub const CODEBUDDY_API_URL: &str = "https://www.codebuddy.ai";
pub const CODEBUDDY_API_PREFIX: &str = "/v2/plugin";

pub const ZED_SIGNIN_URL: &str = "https://zed.dev/native_app_signin";
pub const ZED_CALLBACK_PATH: &str = "/zed-auth-callback";

pub const OAUTH_TIMEOUT_SECS: u64 = 300;

pub fn is_localhost_redirect_provider(provider: &str) -> bool {
    matches!(
        provider,
        "antigravity" | "gemini" | "windsurf" | "zed" | "codex" | "kiro" | "trae"
    )
}

pub fn is_device_flow_provider(provider: &str) -> bool {
    matches!(provider, "github_copilot")
}

pub fn is_cursor_provider(provider: &str) -> bool {
    matches!(provider, "cursor")
}

pub fn is_server_poll_provider(provider: &str) -> bool {
    matches!(provider, "qoder" | "codebuddy")
}

pub fn is_import_only_provider(provider: &str) -> bool {
    matches!(
        provider,
        "claude_code" | "claude_desktop" | "vscode" | "opencode" | "generic_ide"
    )
}
