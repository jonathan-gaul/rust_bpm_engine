use async_trait::async_trait;
use crate::{ProcessInstance, ProcessDefinition, ProcessState};

pub mod sqlx;

#[async_trait]
pub trait InstanceStore: Send + Sync {
    async fn save_definition(&self, def: &ProcessDefinition) -> Result<(), String>;
    async fn save_instance(&self, inst: &ProcessInstance) -> Result<(), String>;
    async fn load_instance(&self, instance_id: &str) -> Result<Option<ProcessInstance>, String>;
    async fn update_instance(&self, inst: &ProcessInstance) -> Result<(), String>;
}
