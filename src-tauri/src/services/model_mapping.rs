use crate::{
    db::Database,
    error::AppError,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMapping {
    pub id: String,
    pub source_model: String,
    pub target_model: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ModelMappingService {
    db: Database,
}

impl ModelMappingService {
    pub fn new(db: &Database) -> Self {
        Self { db: db.clone() }
    }

    pub fn get_all(&self) -> Result<Vec<ModelMapping>, AppError> {
        let rows = self.db.query_rows(
            "SELECT id, source_model, target_model, is_active, created_at, updated_at 
             FROM model_mappings ORDER BY created_at DESC",
            &[],
            |row| {
                use std::str::FromStr;
                Ok(ModelMapping {
                    id: row.get(0)?,
                    source_model: row.get(1)?,
                    target_model: row.get(2)?,
                    is_active: row.get::<_, i32>(3)? != 0,
                    created_at: DateTime::<Utc>::from_str(&row.get::<_, String>(4)?).unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::<Utc>::from_str(&row.get::<_, String>(5)?).unwrap_or_else(|_| Utc::now()),
                })
            },
        )?;
        Ok(rows)
    }

    pub fn upsert(&self, mapping: &ModelMapping) -> Result<(), AppError> {
        let status_int = if mapping.is_active { 1 } else { 0 };
        self.db.execute(
            "INSERT INTO model_mappings (id, source_model, target_model, is_active, created_at, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET 
             source_model=excluded.source_model, 
             target_model=excluded.target_model, 
             is_active=excluded.is_active, 
             updated_at=excluded.updated_at",
            &[
                &mapping.id,
                &mapping.source_model,
                &mapping.target_model,
                &status_int,
                &mapping.created_at.to_rfc3339(),
                &mapping.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), AppError> {
        self.db.execute("DELETE FROM model_mappings WHERE id = ?1", &[&id])?;
        Ok(())
    }
}

// ---------------- 暴露给端侧的 Commands ----------------

#[derive(Deserialize)]
pub struct UpsertModelMappingRequest {
    pub id: Option<String>,
    pub source_model: String,
    pub target_model: String,
    pub is_active: bool,
}

#[tauri::command]
pub async fn list_model_mappings(db: State<'_, Database>) -> Result<Vec<ModelMapping>, AppError> {
    let service = ModelMappingService::new(&db);
    service.get_all()
}

#[tauri::command]
pub async fn upsert_model_mapping(
    db: State<'_, Database>,
    req: UpsertModelMappingRequest,
) -> Result<ModelMapping, AppError> {
    let service = ModelMappingService::new(&db);
    let now = Utc::now();
    let id = req.id.unwrap_or_else(|| Uuid::new_v4().to_string());
    
    let mapping = ModelMapping {
        id: id.clone(),
        source_model: req.source_model,
        target_model: req.target_model,
        is_active: req.is_active,
        created_at: now,
        updated_at: now,
    };
    
    service.upsert(&mapping)?;
    Ok(mapping)
}

#[tauri::command]
pub async fn delete_model_mapping(
    db: State<'_, Database>,
    id: String,
) -> Result<(), AppError> {
    let service = ModelMappingService::new(&db);
    service.delete(&id)
}
