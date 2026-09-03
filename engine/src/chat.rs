use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthorRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TurnMode {
    Manual,
    #[default]
    Natural,
    Random,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub character_id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub mute: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub avatar_data_url: Option<String>,
    #[serde(default)]
    pub members: Vec<GroupMember>,
    #[serde(default)]
    pub turn_mode: TurnMode,
    #[serde(default)]
    pub allow_self_responses: bool,
    #[serde(default = "default_true")]
    pub auto_mode: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Group {
    pub fn new(name: String, members: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: String::new(),
            avatar_data_url: None,
            members: members
                .into_iter()
                .map(|character_id| GroupMember {
                    character_id,
                    enabled: true,
                    mute: false,
                })
                .collect(),
            turn_mode: TurnMode::Natural,
            allow_self_responses: false,
            auto_mode: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn enabled_members(&self) -> Vec<&GroupMember> {
        self.members
            .iter()
            .filter(|m| m.enabled && !m.mute)
            .collect()
    }
}

/// Resolves which character speaks next in a group based on the turn mode and recent speaker history.
pub fn resolve_next_speaker(group: &Group, last_speaker_id: Option<&str>) -> Option<String> {
    let enabled = group.enabled_members();
    if enabled.is_empty() {
        return None;
    }

    match group.turn_mode {
        TurnMode::Manual | TurnMode::Natural => {
            if let Some(last_id) = last_speaker_id
                && let Some(pos) = enabled.iter().position(|m| m.character_id == last_id)
            {
                let next_pos = (pos + 1) % enabled.len();
                return Some(enabled[next_pos].character_id.clone());
            }
            Some(enabled[0].character_id.clone())
        }
        TurnMode::Random => {
            if enabled.len() == 1 {
                return Some(enabled[0].character_id.clone());
            }

            let candidates: Vec<&GroupMember> = if !group.allow_self_responses {
                if let Some(last_id) = last_speaker_id {
                    let filtered: Vec<&GroupMember> = enabled
                        .iter()
                        .copied()
                        .filter(|m| m.character_id != last_id)
                        .collect();
                    if !filtered.is_empty() {
                        filtered
                    } else {
                        enabled
                    }
                } else {
                    enabled
                }
            } else {
                enabled
            };

            let seed = Utc::now().timestamp_nanos_opt().unwrap_or(0) as usize;
            let idx = seed % candidates.len();
            Some(candidates[idx].character_id.clone())
        }
    }
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
    #[serde(default)]
    pub character_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
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
            character_id: None,
            name: None,
        }
    }

    pub fn with_author(
        role: AuthorRole,
        content: String,
        parent_id: Option<Uuid>,
        character_id: Option<String>,
        name: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_id,
            children_ids: Vec::new(),
            role,
            content,
            created_at: Utc::now(),
            selected_child_index: 0,
            character_id,
            name,
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
    #[serde(default)]
    pub character_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
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
    #[serde(default)]
    pub group_id: Option<String>,
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
            group_id: None,
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
            group_id: None,
        }
    }

    pub fn new_group(group_id: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            character_id: format!("group:{group_id}"),
            title,
            created_at: now,
            updated_at: now,
            root_message_ids: Vec::new(),
            nodes: HashMap::new(),
            active_root_index: 0,
            group_id: Some(group_id),
        }
    }

    /// Appends a new message as a child of the specified parent, or as a new root.
    pub fn append_message(
        &mut self,
        role: AuthorRole,
        content: String,
        parent_id: Option<Uuid>,
    ) -> Uuid {
        self.append_message_with_author(role, content, parent_id, None, None)
    }

    /// Appends a new message with explicit author metadata (used for group chats).
    pub fn append_message_with_author(
        &mut self,
        role: AuthorRole,
        content: String,
        parent_id: Option<Uuid>,
        character_id: Option<String>,
        name: Option<String>,
    ) -> Uuid {
        self.updated_at = Utc::now();
        let node = MessageNode::with_author(role, content, parent_id, character_id, name);
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
        self.add_swipe_with_author(parent_id, content, None, None)
    }

    /// Adds an alternate generation (swipe) with explicit author metadata.
    pub fn add_swipe_with_author(
        &mut self,
        parent_id: Option<Uuid>,
        content: String,
        character_id: Option<String>,
        name: Option<String>,
    ) -> Option<Uuid> {
        self.updated_at = Utc::now();
        let node = MessageNode::with_author(
            AuthorRole::Assistant,
            content,
            parent_id,
            character_id,
            name,
        );
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

    /// Updates the speaker attribution for a message.
    pub fn set_message_author(
        &mut self,
        id: Uuid,
        character_id: Option<String>,
        name: Option<String>,
    ) -> Result<(), String> {
        self.updated_at = Utc::now();
        if let Some(node) = self.nodes.get_mut(&id) {
            node.character_id = character_id;
            node.name = name;
            Ok(())
        } else {
            Err("Message not found".to_string())
        }
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

    /// Appends additional text to an existing message (used for continuation of generation).
    pub fn append_to_message(&mut self, id: Uuid, additional_content: &str) -> Result<(), String> {
        self.updated_at = Utc::now();
        if let Some(node) = self.nodes.get_mut(&id) {
            node.content.push_str(additional_content);
            Ok(())
        } else {
            Err("Message not found".to_string())
        }
    }

    /// Cycles active branch (left/right swipe) at a given parent.
    pub fn switch_branch(&mut self, parent_id: Option<Uuid>, index: usize) -> bool {
        self.updated_at = Utc::now();
        if let Some(pid) = parent_id {
            if let Some(parent) = self.nodes.get_mut(&pid)
                && index < parent.children_ids.len()
            {
                parent.selected_child_index = index;
                return true;
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
                character_id: node.character_id.clone(),
                name: node.name.clone(),
            });
        }

        result
    }

    pub fn get_last_node_id(&self) -> Option<Uuid> {
        self.get_active_path().last().map(|n| n.id)
    }
}
/// Helper function to parse diverse SillyTavern timestamp formats.
pub fn parse_sillytavern_date(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(ndt, Utc));
    }
    if let Ok(ts) = s.parse::<i64>() {
        if ts > 100_000_000_000 {
            return DateTime::from_timestamp_millis(ts);
        } else {
            return DateTime::from_timestamp(ts, 0);
        }
    }
    // Check SillyTavern custom pattern: "YYYY-MM-DD @HHh MMm SSs [XXXms]"
    if let Some((date_part, time_part)) = s.split_once('@') {
        let date_part = date_part.trim();
        let time_part = time_part.trim();
        let date_tokens: Vec<&str> = date_part.split('-').collect();
        if date_tokens.len() == 3 {
            let year: i32 = date_tokens[0].parse().ok()?;
            let month: u32 = date_tokens[1].parse().ok()?;
            let day: u32 = date_tokens[2].parse().ok()?;

            let mut hour = 0u32;
            let mut min = 0u32;
            let mut sec = 0u32;
            let mut millis = 0u32;

            for chunk in time_part.split_whitespace() {
                if let Some(h) = chunk.strip_suffix('h').or_else(|| chunk.strip_suffix('H')) {
                    hour = h.parse().unwrap_or(0);
                } else if let Some(m) = chunk
                    .strip_suffix("ms")
                    .or_else(|| chunk.strip_suffix("MS"))
                {
                    millis = m.parse().unwrap_or(0);
                } else if let Some(m) = chunk.strip_suffix('m').or_else(|| chunk.strip_suffix('M'))
                {
                    min = m.parse().unwrap_or(0);
                } else if let Some(sec_str) =
                    chunk.strip_suffix('s').or_else(|| chunk.strip_suffix('S'))
                {
                    sec = sec_str.parse().unwrap_or(0);
                }
            }

            let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
            let time = chrono::NaiveTime::from_hms_milli_opt(hour, min, sec, millis)?;
            let ndt = chrono::NaiveDateTime::new(date, time);
            return Some(DateTime::from_naive_utc_and_offset(ndt, Utc));
        }
    }
    None
}

/// Helper function to format DateTime into SillyTavern humanized timestamp format.
pub fn format_sillytavern_date(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d @%Hh %Mm %Ss %3fms").to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SillyTavernChatHeader {
    pub user_name: String,
    pub character_name: String,
    pub create_date: String,
    #[serde(default)]
    pub chat_metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SillyTavernExportMessage {
    pub name: String,
    pub is_user: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_system: Option<bool>,
    pub is_name: bool,
    pub send_date: String,
    pub mes: String,
    pub extra: serde_json::Value,
    pub force_avatar: String,
    pub swipes: Vec<String>,
    pub swipe_id: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_id: Option<String>,
}

/// Exports a ChatTree into SillyTavern line-delimited JSON (JSONL) format.
/// Line 0 contains the chat session metadata header.
/// Lines 1..N contain the conversation history turns along the active branch with swipe alternatives preserved.
pub fn export_sillytavern_chat_jsonl(
    chat: &ChatTree,
    user_name: &str,
    character_name: &str,
) -> Result<String, String> {
    let mut lines = Vec::new();

    // 1. Line 0: Header
    let header = SillyTavernChatHeader {
        user_name: user_name.to_string(),
        character_name: character_name.to_string(),
        create_date: format_sillytavern_date(&chat.created_at),
        chat_metadata: serde_json::json!({}),
    };
    let header_json = serde_json::to_string(&header).map_err(|e| e.to_string())?;
    lines.push(header_json);

    // 2. Active Path traversal
    let active_path = chat.get_active_path();
    for node in active_path {
        // Collect swipes for this turn
        let (swipes, swipe_id) = if let Some(pid) = node.parent_id {
            if let Some(parent) = chat.nodes.get(&pid) {
                let mut swipe_texts = Vec::new();
                let mut active_idx = 0;
                for (idx, &child_id) in parent.children_ids.iter().enumerate() {
                    if let Some(child_node) = chat.nodes.get(&child_id) {
                        swipe_texts.push(child_node.content.clone());
                        if child_id == node.id {
                            active_idx = idx;
                        }
                    }
                }
                if swipe_texts.is_empty() {
                    (vec![node.content.clone()], 0)
                } else {
                    (swipe_texts, active_idx)
                }
            } else {
                (vec![node.content.clone()], 0)
            }
        } else {
            // Root level
            let mut swipe_texts = Vec::new();
            let mut active_idx = 0;
            for (idx, &root_id) in chat.root_message_ids.iter().enumerate() {
                if let Some(root_node) = chat.nodes.get(&root_id) {
                    swipe_texts.push(root_node.content.clone());
                    if root_id == node.id {
                        active_idx = idx;
                    }
                }
            }
            if swipe_texts.is_empty() {
                (vec![node.content.clone()], 0)
            } else {
                (swipe_texts, active_idx)
            }
        };

        let (is_user, is_system, name) = match node.role {
            AuthorRole::User => (true, None, user_name.to_string()),
            AuthorRole::Assistant => {
                let speaker_name = node.name.as_deref().unwrap_or(character_name);
                (false, None, speaker_name.to_string())
            }
            AuthorRole::System => (false, Some(true), "System".to_string()),
        };

        let msg_obj = SillyTavernExportMessage {
            name,
            is_user,
            is_system,
            is_name: true,
            send_date: format_sillytavern_date(&node.created_at),
            mes: node.content.clone(),
            extra: serde_json::json!({}),
            force_avatar: "".to_string(),
            swipes,
            swipe_id,
            character_id: node.character_id.clone(),
        };

        let msg_json = serde_json::to_string(&msg_obj).map_err(|e| e.to_string())?;
        lines.push(msg_json);
    }

    Ok(lines.join("\n"))
}

/// Imports a SillyTavern JSONL (or JSON array) chat log into a ChatTree.
/// Accurately preserves all swipes, active selections, author roles, timestamps, and metadata.
pub fn import_sillytavern_chat_jsonl(
    input: &str,
    character_id: &str,
    title: Option<&str>,
) -> Result<ChatTree, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Input is empty".to_string());
    }

    // Support both newline-delimited JSON (JSONL) and JSON array of objects
    let json_values: Vec<serde_json::Value> = if trimmed.starts_with('[') {
        serde_json::from_str(trimmed).map_err(|e| format!("Invalid JSON array: {e}"))?
    } else {
        let mut values = Vec::new();
        for line in trimmed.lines() {
            let line_trim = line.trim();
            if !line_trim.is_empty() {
                let val: serde_json::Value = serde_json::from_str(line_trim)
                    .map_err(|e| format!("Invalid JSON line: {e}"))?;
                values.push(val);
            }
        }
        values
    };

    if json_values.is_empty() {
        return Err("No JSON records found in input".to_string());
    }

    let first_val = &json_values[0];
    let is_header = {
        let has_header_keys = first_val.get("user_name").is_some()
            || first_val.get("character_name").is_some()
            || first_val.get("chat_metadata").is_some();
        let has_no_msg_fields = first_val.get("is_user").is_none()
            && first_val.get("is_system").is_none()
            && (first_val.get("mes").is_none()
                || first_val
                    .get("mes")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .is_empty());
        has_header_keys && has_no_msg_fields
    };

    let (header_char_name, header_create_date, start_index) = if is_header {
        let char_name = first_val
            .get("character_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Character");
        let create_date = first_val
            .get("create_date")
            .and_then(|v| v.as_str())
            .and_then(parse_sillytavern_date)
            .unwrap_or_else(Utc::now);
        (char_name.to_string(), create_date, 1)
    } else {
        ("Character".to_string(), Utc::now(), 0)
    };

    let chat_title = if let Some(t) = title {
        if !t.trim().is_empty() {
            t.trim().to_string()
        } else {
            format!("Chat with {header_char_name}")
        }
    } else {
        format!("Chat with {header_char_name}")
    };

    let mut tree = ChatTree::new(character_id.to_string(), chat_title);
    tree.created_at = header_create_date;
    tree.updated_at = header_create_date;

    let mut current_parent_id: Option<Uuid> = None;

    for val in json_values.iter().skip(start_index) {
        let mes = val
            .get("mes")
            .or_else(|| val.get("text"))
            .or_else(|| val.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let is_user = val
            .get("is_user")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let is_system = val
            .get("is_system")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let send_date = val
            .get("send_date")
            .and_then(|v| v.as_str())
            .and_then(parse_sillytavern_date)
            .unwrap_or_else(Utc::now);

        let swipes_list: Vec<String> =
            if let Some(arr) = val.get("swipes").and_then(|v| v.as_array()) {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            } else {
                Vec::new()
            };

        let mut final_swipes = if swipes_list.is_empty() {
            vec![mes.to_string()]
        } else {
            swipes_list
        };

        // Ensure that if mes is not empty and somehow not in swipes, it is included
        if !mes.is_empty() && !final_swipes.iter().any(|s| s == mes) {
            final_swipes.insert(0, mes.to_string());
        }

        let swipe_id = val
            .get("swipe_id")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(0);
        let active_swipe_idx = swipe_id.min(final_swipes.len().saturating_sub(1));

        let role = if is_system {
            AuthorRole::System
        } else if is_user {
            AuthorRole::User
        } else {
            AuthorRole::Assistant
        };

        let custom_name = val
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let custom_char_id = val
            .get("character_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if current_parent_id.is_none() {
            // Root level
            let mut turn_node_ids = Vec::new();
            for swipe_content in &final_swipes {
                let mut node = MessageNode::with_author(
                    role.clone(),
                    swipe_content.clone(),
                    None,
                    custom_char_id.clone(),
                    custom_name.clone(),
                );
                node.created_at = send_date;
                let node_id = node.id;
                tree.nodes.insert(node_id, node);
                tree.root_message_ids.push(node_id);
                turn_node_ids.push(node_id);
            }
            tree.active_root_index = active_swipe_idx;
            current_parent_id = turn_node_ids.get(active_swipe_idx).copied();
        } else {
            let parent_id = current_parent_id.unwrap();
            let mut turn_node_ids = Vec::new();
            for swipe_content in &final_swipes {
                let mut node = MessageNode::with_author(
                    role.clone(),
                    swipe_content.clone(),
                    Some(parent_id),
                    custom_char_id.clone(),
                    custom_name.clone(),
                );
                node.created_at = send_date;
                let node_id = node.id;
                tree.nodes.insert(node_id, node);
                turn_node_ids.push(node_id);
            }

            if let Some(parent) = tree.nodes.get_mut(&parent_id) {
                for &nid in &turn_node_ids {
                    parent.children_ids.push(nid);
                }
                parent.selected_child_index = active_swipe_idx;
            }

            current_parent_id = turn_node_ids.get(active_swipe_idx).copied();
        }
        tree.updated_at = send_date;
    }

    Ok(tree)
}
