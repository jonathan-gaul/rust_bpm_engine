use crate::persistence::InstanceStore;
use crate::{ProcessInstance, ProcessDefinition, ProcessState};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::collections::HashMap;

pub struct SqlxPostgresStore {
    pool: PgPool,
}

impl SqlxPostgresStore {
    pub async fn new(database_url: &str) -> Result<Self, String> {
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl InstanceStore for SqlxPostgresStore {
    async fn save_definition(&self, def: &ProcessDefinition) -> Result<(), String> {
        let def_json = serde_json::to_value(def).map_err(|e| e.to_string())?;
        sqlx::query(
            "INSERT INTO process_definitions (id, definition) VALUES ($1, $2)
               ON CONFLICT (id) DO UPDATE SET definition = EXCLUDED.definition",
        )
        .bind(&def.id)
        .bind(def_json)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn save_instance(&self, inst: &ProcessInstance) -> Result<(), String> {
        let vars = serde_json::to_value(&inst.variables).map_err(|e| e.to_string())?;
        let current = serde_json::to_value(&inst.current_node_ids).map_err(|e| e.to_string())?;
        let joins = serde_json::to_value(&inst.join_counters).map_err(|e| e.to_string())?;
        let state_str = format!("{:?}", inst.state);

        sqlx::query("INSERT INTO process_instances (id, process_def_id, state, variables, current_node_ids, active_tokens, join_counters, created_at, updated_at)
                             VALUES ($1,$2,$3,$4,$5,$6,$7, now(), now())
                             ON CONFLICT (id) DO UPDATE SET process_def_id = EXCLUDED.process_def_id, state = EXCLUDED.state, variables = EXCLUDED.variables,
                                 current_node_ids = EXCLUDED.current_node_ids, active_tokens = EXCLUDED.active_tokens, join_counters = EXCLUDED.join_counters, updated_at = now()")
                        .bind(&inst.id)
                        .bind(&inst.process_def_id)
                        .bind(&state_str)
                        .bind(vars)
                        .bind(current)
                        .bind(inst.active_tokens as i32)
                        .bind(joins)
                        .execute(&self.pool)
                        .await
                        .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn load_instance(&self, instance_id: &str) -> Result<Option<ProcessInstance>, String> {
        let row = sqlx::query("SELECT id, process_def_id, state, variables, current_node_ids, active_tokens, join_counters FROM process_instances WHERE id = $1")
            .bind(instance_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(r) = row {
            let id: String = r.try_get("id").map_err(|e| e.to_string())?;
            let process_def_id: String = r.try_get("process_def_id").map_err(|e| e.to_string())?;
            let state_str: String = r.try_get("state").map_err(|e| e.to_string())?;
            let state = match state_str.as_str() {
                "Running" => ProcessState::Running,
                "Completed" => ProcessState::Completed,
                _ => ProcessState::Failed,
            };
            let variables_val: serde_json::Value =
                r.try_get("variables").map_err(|e| e.to_string())?;
            let current_val: serde_json::Value =
                r.try_get("current_node_ids").map_err(|e| e.to_string())?;
            let active_tokens_i32: i32 = r.try_get("active_tokens").map_err(|e| e.to_string())?;
            let joins_val: serde_json::Value =
                r.try_get("join_counters").map_err(|e| e.to_string())?;

            let variables: HashMap<String, String> =
                serde_json::from_value(variables_val).map_err(|e| e.to_string())?;
            let current_node_ids: Vec<String> =
                serde_json::from_value(current_val).map_err(|e| e.to_string())?;
            let active_tokens = active_tokens_i32 as usize;
            let join_counters: HashMap<String, usize> =
                serde_json::from_value(joins_val).map_err(|e| e.to_string())?;

            Ok(Some(ProcessInstance {
                id,
                process_def_id,
                current_node_ids,
                state,
                variables,
                active_tokens,
                join_counters,
            }))
        } else {
            Ok(None)
        }
    }

    async fn update_instance(&self, inst: &ProcessInstance) -> Result<(), String> {
        self.save_instance(inst).await
    }
}
