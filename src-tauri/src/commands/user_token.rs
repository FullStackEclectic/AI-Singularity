use crate::models::{CreateUserTokenReq, UpdateUserTokenReq, UserToken};
use crate::services::user_token::UserTokenService;
use crate::db::Database;
use tauri::State;

#[tauri::command]
pub fn create_user_token(
    db: State<'_, Database>,
    req: CreateUserTokenReq,
) -> Result<UserToken, String> {
    let service = UserTokenService::new(db.inner());
    service.create_token(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_all_user_tokens(db: State<'_, Database>) -> Result<Vec<UserToken>, String> {
    let service = UserTokenService::new(db.inner());
    service.get_all_tokens().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_user_token(
    db: State<'_, Database>,
    req: UpdateUserTokenReq,
) -> Result<(), String> {
    let service = UserTokenService::new(db.inner());
    service.update_token(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_user_token(db: State<'_, Database>, id: String) -> Result<(), String> {
    let service = UserTokenService::new(db.inner());
    service.delete_token(&id).map_err(|e| e.to_string())
}
