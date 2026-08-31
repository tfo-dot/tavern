use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthorRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageNode {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub children_ids: Vec<Uuid>,
    pub role: AuthorRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub selected_child_index: usize,
}

impl MessageNode {
    pub fn new(role: AuthorRole, content: String, parent_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_id,
            children_ids: Vec::new(),
            role,
            content,
            created_at: Utc::now(),
            selected_child_index: 0,
        }
    }
}

/// A flattened view node designed for clean rendering and swipe navigation in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageViewNode {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub role: AuthorRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub sibling_index: usize,
    pub sibling_total: usize,
    pub can_swipe_left: bool,
    pub can_swipe_right: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTree {
    pub id: Uuid,
    pub character_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub root_message_ids: Vec<Uuid>,
    pub nodes: HashMap<Uuid, MessageNode>,
    pub active_root_index: usize,
}

impl Default for ChatTree {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            character_id: "default".to_string(),
            title: "New Chat".to_string(),
            created_at: now,
            updated_at: now,
            root_message_ids: Vec::new(),
            nodes: HashMap::new(),
            active_root_index: 0,
        }
    }
}

impl ChatTree {
    pub fn new(character_id: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            character_id,
            title,
            created_at: now,
            updated_at: now,
            root_message_ids: Vec::new(),
            nodes: HashMap::new(),
            active_root_index: 0,
        }
    }

    /// Appends a new message as a child of the specified parent, or as a new root.
    pub fn append_message(&mut self, role: AuthorRole, content: String, parent_id: Option<Uuid>) -> Uuid {
        self.updated_at = Utc::now();
        let node = MessageNode::new(role, content, parent_id);
        let node_id = node.id;

        if let Some(pid) = parent_id {
            if let Some(parent) = self.nodes.get_mut(&pid) {
                parent.children_ids.push(node_id);
                parent.selected_child_index = parent.children_ids.len() - 1;
            }
        } else {
            self.root_message_ids.push(node_id);
            self.active_root_index = self.root_message_ids.len() - 1;
        }

        self.nodes.insert(node_id, node);
        node_id
    }

    /// Adds an alternate generation (swipe) under the same parent node.
    pub fn add_swipe(&mut self, parent_id: Option<Uuid>, content: String) -> Option<Uuid> {
        self.updated_at = Utc::now();
        let node = MessageNode::new(AuthorRole::Assistant, content, parent_id);
        let node_id = node.id;

        if let Some(pid) = parent_id {
            let parent = self.nodes.get_mut(&pid)?;
            parent.children_ids.push(node_id);
            parent.selected_child_index = parent.children_ids.len() - 1;
        } else {
            self.root_message_ids.push(node_id);
            self.active_root_index = self.root_message_ids.len() - 1;
        }

        self.nodes.insert(node_id, node);
        Some(node_id)
    }

    /// Edits an existing message's text content.
    pub fn edit_message(&mut self, id: Uuid, new_content: String) -> Result<(), String> {
        self.updated_at = Utc::now();
        if let Some(node) = self.nodes.get_mut(&id) {
            node.content = new_content;
            Ok(())
        } else {
            Err("Message not found".to_string())
        }
    }

    /// Cycles active branch (left/right swipe) at a given parent.
    pub fn switch_branch(&mut self, parent_id: Option<Uuid>, index: usize) -> bool {
        self.updated_at = Utc::now();
        if let Some(pid) = parent_id {
            if let Some(parent) = self.nodes.get_mut(&pid) {
                if index < parent.children_ids.len() {
                    parent.selected_child_index = index;
                    return true;
                }
            }
        } else if index < self.root_message_ids.len() {
            self.active_root_index = index;
            return true;
        }
        false
    }

    /// Deletes a message node and all its descendants.
    pub fn delete_message(&mut self, id: Uuid) -> Result<(), String> {
        self.updated_at = Utc::now();
        let parent_id = match self.nodes.get(&id) {
            Some(node) => node.parent_id,
            None => return Err("Message not found".to_string()),
        };

        // Remove recursive subtree
        self.remove_subtree(id);

        // Update parent or roots
        if let Some(pid) = parent_id {
            if let Some(parent) = self.nodes.get_mut(&pid) {
                parent.children_ids.retain(|&child_id| child_id != id);
                if parent.selected_child_index >= parent.children_ids.len() {
                    parent.selected_child_index = parent.children_ids.len().saturating_sub(1);
                }
            }
        } else {
            self.root_message_ids.retain(|&root_id| root_id != id);
            if self.active_root_index >= self.root_message_ids.len() {
                self.active_root_index = self.root_message_ids.len().saturating_sub(1);
            }
        }

        Ok(())
    }

    fn remove_subtree(&mut self, id: Uuid) {
        if let Some(node) = self.nodes.remove(&id) {
            for child_id in node.children_ids {
                self.remove_subtree(child_id);
            }
        }
    }

    /// Traverses the selected path from root to current leaf for context assembly.
    pub fn get_active_path(&self) -> Vec<&MessageNode> {
        let mut path = Vec::new();

        if self.root_message_ids.is_empty() {
            return path;
        }

        let root_id = match self.root_message_ids.get(self.active_root_index) {
            Some(id) => *id,
            None => return path,
        };

        let mut current_id = Some(root_id);

        while let Some(id) = current_id {
            if let Some(node) = self.nodes.get(&id) {
                path.push(node);
                if node.children_ids.is_empty() {
                    current_id = None;
                } else {
                    let next_idx = node.selected_child_index.min(node.children_ids.len() - 1);
                    current_id = node.children_ids.get(next_idx).copied();
                }
            } else {
                break;
            }
        }

        path
    }

    /// Returns the active path messages augmented with swipe and sibling branch metadata.
    pub fn get_active_view_nodes(&self) -> Vec<MessageViewNode> {
        let mut result = Vec::new();
        let path = self.get_active_path();

        for node in path {
            let (sibling_index, sibling_total) = if let Some(pid) = node.parent_id {
                if let Some(parent) = self.nodes.get(&pid) {
                    let idx = parent
                        .children_ids
                        .iter()
                        .position(|&id| id == node.id)
                        .unwrap_or(0);
                    (idx, parent.children_ids.len())
                } else {
                    (0, 1)
                }
            } else {
                let idx = self
                    .root_message_ids
                    .iter()
                    .position(|&id| id == node.id)
                    .unwrap_or(0);
                (idx, self.root_message_ids.len())
            };

            result.push(MessageViewNode {
                id: node.id,
                parent_id: node.parent_id,
                role: node.role.clone(),
                content: node.content.clone(),
                created_at: node.created_at,
                sibling_index,
                sibling_total,
                can_swipe_left: sibling_index > 0,
                can_swipe_right: sibling_index + 1 < sibling_total,
            });
        }

        result
    }

    pub fn get_last_node_id(&self) -> Option<Uuid> {
        self.get_active_path().last().map(|n| n.id)
    }
}
