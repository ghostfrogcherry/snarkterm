use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SplitNode {
    Pane { session_index: usize },
    Horizontal { children: Vec<SplitNode> },
    Vertical { children: Vec<SplitNode> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabModel {
    pub title: String,
    pub root: SplitNode,
}
