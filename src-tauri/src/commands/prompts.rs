use crate::error::AppResult;
use crate::models::PromptConfig;
use crate::services::prompts::PromptService;
use crate::db::Database;
use tauri::State;

#[tauri::command]
pub fn get_prompts(db: State<'_, Database>) -> AppResult<Vec<PromptConfig>> {
    let service = PromptService::new(&db);
    service.list_prompts()
}

#[tauri::command]
pub fn save_prompt(db: State<'_, Database>, prompt: PromptConfig) -> AppResult<()> {
    let service = PromptService::new(&db);
    service.save_prompt(prompt)
}

#[tauri::command]
pub fn delete_prompt(db: State<'_, Database>, id: String) -> AppResult<()> {
    let service = PromptService::new(&db);
    service.delete_prompt(&id)
}

#[tauri::command]
pub fn sync_prompt(db: State<'_, Database>, id: String, workspace_dir: String) -> AppResult<()> {
    let service = PromptService::new(&db);
    service.sync_prompt_to_workspace(&id, &workspace_dir)
}

#[tauri::command]
pub fn sync_prompt_to_tool(db: State<'_, Database>, id: String) -> AppResult<Vec<String>> {
    let service = PromptService::new(&db);
    service.sync_to_tool_defaults(&id)
}
