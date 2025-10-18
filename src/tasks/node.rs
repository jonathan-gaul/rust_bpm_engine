
use std::collections::HashMap;

use crate::tasks::config::TaskConfig;
use crate::tasks::handlers::TaskHandlerType;

// Node types in our workflow
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum NodeType {
    Start,
    Task { 
        name: String,
        handler_type: TaskHandlerType,
        config: TaskConfig,
    },
    ExclusiveGateway, // XOR - one path based on condition
    ParallelGateway,  // AND - fork or join
    End,
}

// A node in the process definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub id: String,
    pub node_type: NodeType,
    pub outgoing: Vec<String>, // IDs of next nodes
    pub conditions: HashMap<String, String>, // For exclusive gateways: edge_id -> condition expression
}
