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

    if enabled.len() == 1 {
        return Some(enabled[0].character_id.clone());
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
            let mut candidates = enabled;

            if !group.allow_self_responses
                && let Some(last_id) = last_speaker_id
            {
                let has_other_speakers = candidates.iter().any(|m| m.character_id != last_id);
                if has_other_speakers {
                    candidates.retain(|m| m.character_id != last_id);
                }
            }

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

fn default_authors_note_depth() -> usize {
    1
}

fn default_authors_note_interval() -> usize {
    1
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
    #[serde(default)]
    pub authors_note: String,
    #[serde(default = "default_authors_note_depth")]
    pub authors_note_depth: usize,
    #[serde(default = "default_authors_note_interval")]
    pub authors_note_interval: usize,
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
            authors_note: String::new(),
            authors_note_depth: 1,
            authors_note_interval: 1,
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
            authors_note: String::new(),
            authors_note_depth: 1,
            authors_note_interval: 1,
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
            authors_note: String::new(),
            authors_note_depth: 1,
            authors_note_interval: 1,
        }
    }
    /// Appends a new message as a child of parent, or as a new root.
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

    /// Adds an alternate generation under the same parent node.
    pub fn add_swipe(&mut self, parent_id: Option<Uuid>, content: String) -> Option<Uuid> {
        self.add_swipe_with_author(parent_id, content, None, None)
    }

    /// Adds an alternate generation with explicit author metadata.
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
            let node = self.nodes.get(&id);

            if node.is_none() {
                break;
            }

            let node = node.unwrap();

            path.push(node);

            if node.children_ids.is_empty() {
                current_id = None;
            } else {
                let next_idx = node.selected_child_index.min(node.children_ids.len() - 1);
                current_id = node.children_ids.get(next_idx).copied();
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
    /// Forks the chat tree from the root down to the given `message_id`, creating a new independent ChatTree.
    pub fn fork_at_message(
        &self,
        message_id: Uuid,
        new_title: Option<String>,
    ) -> Result<ChatTree, String> {
        if !self.nodes.contains_key(&message_id) {
            return Err("Target message node not found in chat".to_string());
        }

        // Trace from message_id backwards to root
        let mut path_ids = Vec::new();
        let mut curr = Some(message_id);
        while let Some(id) = curr {
            path_ids.push(id);
            curr = self.nodes.get(&id).and_then(|n| n.parent_id);
        }
        path_ids.reverse();

        let title = new_title.unwrap_or_else(|| format!("{} (Fork)", self.title));
        let mut forked = match &self.group_id {
            Some(gid) => ChatTree::new_group(gid.clone(), title),
            None => ChatTree::new(self.character_id.clone(), title),
        };
        forked.authors_note = self.authors_note.clone();
        forked.authors_note_depth = self.authors_note_depth;
        forked.authors_note_interval = self.authors_note_interval;

        let mut parent_id = None;
        for id in path_ids {
            if let Some(orig) = self.nodes.get(&id) {
                let new_id = forked.append_message_with_author(
                    orig.role.clone(),
                    orig.content.clone(),
                    parent_id,
                    orig.character_id.clone(),
                    orig.name.clone(),
                );
                parent_id = Some(new_id);
            }
        }

        Ok(forked)
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
        }

        return DateTime::from_timestamp(ts, 0);
    }

    let trimmed = s.trim();

    let parsed = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d @%Hh %Mm %Ss %3fms")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d @%Hh %Mm %Ss"));

    if let Ok(ndt) = parsed {
        return Some(DateTime::from_naive_utc_and_offset(ndt, Utc));
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

/// Exports a ChatTree into SillyTavern JSONL format.
/// First line contains the chat session header, rest is conversation history (with alternatives preserved)
pub fn export_sillytavern_chat_jsonl(
    chat: &ChatTree,
    user_name: &str,
    character_name: &str,
) -> Result<String, String> {
    let mut out = Vec::with_capacity(4096);

    // Header
    let header = SillyTavernChatHeader {
        user_name: user_name.to_string(),
        character_name: character_name.to_string(),
        create_date: format_sillytavern_date(&chat.created_at),
        chat_metadata: serde_json::json!({}),
    };
    serde_json::to_writer(&mut out, &header).map_err(|e| e.to_string())?;
    out.push(b'\n');

    for node in chat.get_active_path() {
        let sibling_ids: &[Uuid] = match node.parent_id {
            Some(pid) => chat.nodes.get(&pid).map_or(&[], |p| &p.children_ids),
            None => &chat.root_message_ids,
        };

        let mut swipes = Vec::new();
        let mut swipe_id = 0;

        for (idx, &sib_id) in sibling_ids.iter().enumerate() {
            if let Some(sibiling_node) = chat.nodes.get(&sib_id) {
                swipes.push(sibiling_node.content.clone());
                if sib_id == node.id {
                    swipe_id = idx;
                }
            }
        }

        if swipes.is_empty() {
            swipes.push(node.content.clone());
            swipe_id = 0;
        }

        let (is_user, is_system, name) = match node.role {
            AuthorRole::User => (true, None, user_name.to_string()),
            AuthorRole::Assistant => {
                let speaker = node.name.as_deref().unwrap_or(character_name);
                (false, None, speaker.to_string())
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
            force_avatar: String::new(),
            swipes,
            swipe_id,
            character_id: node.character_id.clone(),
        };

        serde_json::to_writer(&mut out, &msg_obj).map_err(|e| e.to_string())?;
        out.push(b'\n');
    }

    String::from_utf8(out).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct RawHeader {
    character_name: Option<String>,
    create_date: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawMessage {
    #[serde(alias = "text", alias = "content")]
    mes: Option<String>,
    #[serde(default)]
    is_user: bool,
    #[serde(default)]
    is_system: bool,
    send_date: Option<String>,
    #[serde(default)]
    swipes: Vec<String>,
    #[serde(default)]
    swipe_id: usize,
    name: Option<String>,
    character_id: Option<String>,
}

/// Imports a SillyTavern JSONL (or JSON array) chat log into a ChatTree.
pub fn import_sillytavern_chat_jsonl(
    input: &str,
    character_id: &str,
    title: Option<&str>,
) -> Result<ChatTree, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Input is empty".to_string());
    }

    let json_values: Vec<serde_json::Value> = if trimmed.starts_with('[') {
        serde_json::from_str(trimmed).map_err(|e| format!("Invalid JSON array: {e}"))?
    } else {
        trimmed
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(serde_json::from_str)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Invalid JSON line: {e}"))?
    };

    let first_val = json_values
        .first()
        .ok_or("No JSON records found in input")?;

    let is_header = (first_val.get("user_name").is_some()
        || first_val.get("character_name").is_some()
        || first_val.get("chat_metadata").is_some())
        && first_val.get("is_user").is_none()
        && first_val.get("is_system").is_none()
        && first_val
            .get("mes")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .is_empty();

    let (header_char_name, header_create_date, start_index) = if is_header {
        let header: RawHeader = serde_json::from_value(first_val.clone()).unwrap_or(RawHeader {
            character_name: None,
            create_date: None,
        });
        let date = header
            .create_date
            .as_deref()
            .and_then(parse_sillytavern_date)
            .unwrap_or_else(Utc::now);
        (
            header
                .character_name
                .unwrap_or_else(|| "Character".to_string()),
            date,
            1,
        )
    } else {
        ("Character".to_string(), Utc::now(), 0)
    };

    let chat_title = match title.map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => t.to_string(),
        None => format!("Chat with {header_char_name}"),
    };

    let mut tree = ChatTree::new(character_id.to_string(), chat_title);
    tree.created_at = header_create_date;
    tree.updated_at = header_create_date;

    let mut current_parent_id: Option<Uuid> = None;

    for val in &json_values[start_index..] {
        let msg: RawMessage = serde_json::from_value(val.clone())
            .map_err(|e| format!("Invalid message record: {e}"))?;

        let mes = msg.mes.unwrap_or_default();
        let send_date = msg
            .send_date
            .as_deref()
            .and_then(parse_sillytavern_date)
            .unwrap_or_else(Utc::now);

        let mut final_swipes = if msg.swipes.is_empty() {
            vec![mes.clone()]
        } else {
            msg.swipes
        };

        let mut active_swipe_idx = msg.swipe_id;

        if !mes.is_empty() && !final_swipes.iter().any(|s| s == &mes) {
            final_swipes.push(mes);
        }
        active_swipe_idx = active_swipe_idx.min(final_swipes.len().saturating_sub(1));

        let role = if msg.is_system {
            AuthorRole::System
        } else if msg.is_user {
            AuthorRole::User
        } else {
            AuthorRole::Assistant
        };

        let turn_node_ids: Vec<Uuid> = final_swipes
            .into_iter()
            .map(|content| {
                let mut node = MessageNode::with_author(
                    role.clone(),
                    content,
                    current_parent_id,
                    msg.character_id.clone(),
                    msg.name.clone(),
                );
                node.created_at = send_date;
                let id = node.id;
                tree.nodes.insert(id, node);
                id
            })
            .collect();

        match current_parent_id {
            None => {
                tree.root_message_ids.extend(&turn_node_ids);
                tree.active_root_index = active_swipe_idx;
            }
            Some(parent_id) => {
                if let Some(parent) = tree.nodes.get_mut(&parent_id) {
                    parent.children_ids.extend(&turn_node_ids);
                    parent.selected_child_index = active_swipe_idx;
                }
            }
        }

        current_parent_id = turn_node_ids.get(active_swipe_idx).copied();
        tree.updated_at = send_date;
    }

    Ok(tree)
}
