use crate::db::Database;
use crate::error::AppResult;
use crate::models::PromptConfig;
use crate::services::event_bus::EventBus;
use crate::services::prompts::PromptService;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_prompts(db: State<'_, Database>) -> AppResult<Vec<PromptConfig>> {
    let service = PromptService::new(&db);
    service.list_prompts()
}

#[tauri::command]
pub fn save_prompt(db: State<'_, Database>, prompt: PromptConfig, app: AppHandle) -> AppResult<()> {
    let service = PromptService::new(&db);
    service.save_prompt(prompt)?;
    EventBus::emit_data_changed(&app, "prompts", "save", "prompt.save");
    Ok(())
}

#[tauri::command]
pub fn delete_prompt(db: State<'_, Database>, id: String, app: AppHandle) -> AppResult<()> {
    let service = PromptService::new(&db);
    service.delete_prompt(&id)?;
    EventBus::emit_data_changed(&app, "prompts", "delete", "prompt.delete");
    Ok(())
}

#[tauri::command]
pub fn sync_prompt(
    db: State<'_, Database>,
    id: String,
    workspace_dir: String,
    app: AppHandle,
) -> AppResult<()> {
    let service = PromptService::new(&db);
    service.sync_prompt_to_workspace(&id, &workspace_dir)?;
    EventBus::emit_data_changed(&app, "prompts", "sync_workspace", "prompt.sync_workspace");
    Ok(())
}

#[tauri::command]
pub fn sync_prompt_to_tool(
    db: State<'_, Database>,
    id: String,
    app: AppHandle,
) -> AppResult<Vec<String>> {
    let service = PromptService::new(&db);
    let files = service.sync_to_tool_defaults(&id)?;
    EventBus::emit_data_changed(&app, "prompts", "sync_tool", "prompt.sync_tool");
    Ok(files)
}
