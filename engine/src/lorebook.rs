use chrono::{DateTime, Utc};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum LorebookError {
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("Invalid lorebook format")]
    InvalidFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LorebookPosition {
    #[default]
    BeforeChar,
    AfterChar,
    BeforeScenario,
    AfterScenario,
    TopSystem,
    BottomSystem,
    AtDepth,
}

impl LorebookPosition {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "before_char" | "before_character" | "before_character_defs" | "0" => Self::BeforeChar,
            "after_char" | "after_character" | "after_character_defs" | "1" => Self::AfterChar,
            "before_scenario" | "before_scenario_defs" | "before_prompt" | "2" => {
                Self::BeforeScenario
            }
            "after_scenario" | "after_scenario_defs" | "after_prompt" | "3" => Self::AfterScenario,
            "top_system" | "top_of_system" | "system_top" | "4" => Self::TopSystem,
            "bottom_system" | "bottom_of_system" | "system_bottom" | "5" => Self::BottomSystem,
            "at_depth" | "depth" | "in_chat" | "chat" | "6" => Self::AtDepth,
            _ => Self::BeforeChar,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectiveLogic {
    #[default]
    AndAny = 0, // Requires at least one secondary key
    NotAny = 1, // Requires NONE of the secondary keys to be present
    AndAll = 2, // Requires ALL secondary keys to be present
    NotAll = 3, // Requires at least one secondary key to NOT be present
}

impl Serialize for SelectiveLogic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for SelectiveLogic {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Number(n) => match n.as_u64() {
                Some(0) => Ok(SelectiveLogic::AndAny),
                Some(1) => Ok(SelectiveLogic::NotAny),
                Some(2) => Ok(SelectiveLogic::AndAll),
                Some(3) => Ok(SelectiveLogic::NotAll),
                _ => Ok(SelectiveLogic::AndAny),
            },
            serde_json::Value::String(s) => match s.to_lowercase().as_str() {
                "and_any" | "andany" | "0" => Ok(SelectiveLogic::AndAny),
                "not_any" | "notany" | "1" => Ok(SelectiveLogic::NotAny),
                "and_all" | "andall" | "2" => Ok(SelectiveLogic::AndAll),
                "not_all" | "notall" | "3" => Ok(SelectiveLogic::NotAll),
                _ => Ok(SelectiveLogic::AndAny),
            },
            _ => Ok(SelectiveLogic::AndAny),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LorebookEntry {
    pub id: String,
    pub keys: Vec<String>,
    pub secondary_keys: Vec<String>,
    pub content: String,
    pub comment: String,
    pub enabled: bool,
    pub constant: bool,
    pub selective: bool,
    pub selective_logic: SelectiveLogic,
    pub position: LorebookPosition,
    pub depth: usize,
    pub order: i32,
    pub case_sensitive: bool,
    pub use_regex: bool,
    pub prevent_recursion: bool,
    pub scan_depth: Option<usize>,
    pub extensions: serde_json::Value,
}

impl<'de> Deserialize<'de> for LorebookEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let val = serde_json::Value::deserialize(deserializer)?;
        parse_single_entry(&val)
            .ok_or_else(|| serde::de::Error::custom("invalid lorebook entry object"))
    }
}
impl Default for LorebookEntry {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            keys: Vec::new(),
            secondary_keys: Vec::new(),
            content: String::new(),
            comment: String::new(),
            enabled: true,
            constant: false,
            selective: false,
            selective_logic: SelectiveLogic::AndAny,
            position: LorebookPosition::BeforeChar,
            depth: 4,
            order: 100,
            case_sensitive: false,
            use_regex: false,
            prevent_recursion: false,
            scan_depth: None,
            extensions: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lorebook {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_scan_depth")]
    pub scan_depth: usize,
    #[serde(default = "default_token_budget")]
    pub token_budget: usize,
    #[serde(default)]
    pub recursive_scanning: bool,
    #[serde(default)]
    pub global: bool,
    #[serde(default)]
    pub entries: Vec<LorebookEntry>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub extensions: serde_json::Value,
}

fn default_scan_depth() -> usize {
    2
}

fn default_token_budget() -> usize {
    2048
}

impl Default for Lorebook {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: "New Lorebook".to_string(),
            description: String::new(),
            scan_depth: 2,
            token_budget: 2048,
            recursive_scanning: false,
            global: false,
            entries: Vec::new(),
            created_at: now,
            updated_at: now,
            extensions: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

impl Lorebook {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            ..Default::default()
        }
    }

    pub fn add_entry(&mut self, mut entry: LorebookEntry) -> String {
        if entry.id.is_empty() {
            entry.id = Uuid::new_v4().to_string();
        }
        let id = entry.id.clone();
        self.entries.push(entry);
        self.updated_at = Utc::now();
        id
    }

    pub fn to_character_book(&self) -> CharacterBook {
        CharacterBook {
            name: Some(self.name.clone()),
            description: Some(self.description.clone()),
            scan_depth: Some(self.scan_depth),
            token_budget: Some(self.token_budget),
            recursive_scanning: Some(self.recursive_scanning),
            entries: self.entries.clone(),
            extensions: self.extensions.clone(),
        }
    }
}

/// Embedded Character Book for Character Card V2
#[derive(Debug, Clone, Serialize, Default)]
pub struct CharacterBook {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub scan_depth: Option<usize>,
    #[serde(default)]
    pub token_budget: Option<usize>,
    #[serde(default)]
    pub recursive_scanning: Option<bool>,
    #[serde(default)]
    pub entries: Vec<LorebookEntry>,
    #[serde(default)]
    pub extensions: serde_json::Value,
}

impl<'de> Deserialize<'de> for CharacterBook {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let val = serde_json::Value::deserialize(deserializer)?;
        let obj = match val.as_object() {
            Some(map) => map,
            None => {
                return Err(serde::de::Error::custom(
                    "expected object for character_book",
                ));
            }
        };

        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let description = obj
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let scan_depth = obj
            .get("scan_depth")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);
        let token_budget = obj
            .get("token_budget")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);
        let recursive_scanning = obj.get("recursive_scanning").and_then(|v| v.as_bool());

        let mut entries = Vec::new();
        if let Some(entries_val) = obj.get("entries") {
            match entries_val {
                serde_json::Value::Array(arr) => {
                    for item in arr {
                        if let Some(entry) = parse_single_entry(item) {
                            entries.push(entry);
                        }
                    }
                }
                serde_json::Value::Object(map) => {
                    let mut sorted_keys: Vec<_> = map.keys().collect();
                    sorted_keys.sort_by_key(|k| k.parse::<i64>().unwrap_or(0));
                    for k in sorted_keys {
                        if let Some(item) = map.get(k)
                            && let Some(entry) = parse_single_entry(item)
                        {
                            entries.push(entry);
                        }
                    }
                }
                _ => {}
            }
        }

        let extensions = obj
            .get("extensions")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

        Ok(CharacterBook {
            name,
            description,
            scan_depth,
            token_budget,
            recursive_scanning,
            entries,
            extensions,
        })
    }
}

impl CharacterBook {
    pub fn to_lorebook(&self) -> Lorebook {
        let now = Utc::now();
        Lorebook {
            id: Uuid::new_v4().to_string(),
            name: self
                .name
                .clone()
                .unwrap_or_else(|| "Character Lorebook".to_string()),
            description: self.description.clone().unwrap_or_default(),
            scan_depth: self.scan_depth.unwrap_or(2),
            token_budget: self.token_budget.unwrap_or(2048),
            recursive_scanning: self.recursive_scanning.unwrap_or(false),
            global: false,
            entries: self.entries.clone(),
            created_at: now,
            updated_at: now,
            extensions: self.extensions.clone(),
        }
    }
}

/// Helper to parse string or array into Vec<String>
fn extract_string_list(val: Option<&serde_json::Value>) -> Vec<String> {
    match val {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect(),
        Some(serde_json::Value::String(s)) => {
            if s.trim().is_empty() {
                Vec::new()
            } else {
                s.split(',')
                    .map(|part| part.trim().to_string())
                    .filter(|part| !part.is_empty())
                    .collect()
            }
        }
        _ => Vec::new(),
    }
}

/// Flexible parser that can ingest standard Lorebook JSON, CharacterCard V2 character_book,
/// SillyTavern World Info JSON export (with array or object entries), or character PNG containing a book.
pub fn parse_lorebook(bytes: &[u8]) -> Result<Lorebook, LorebookError> {
    // If PNG, check if character card has embedded character_book
    if bytes.len() >= 8
        && &bytes[0..8] == b"\x89PNG\r\n\x1a\n"
        && let Ok((card, _)) = crate::parser::parse_character_card(bytes)
        && let Some(book) = card.data.character_book
    {
        let mut lorebook = book.to_lorebook();
        if lorebook.name == "Character Lorebook" && !card.data.name.is_empty() {
            lorebook.name = format!("{} Lorebook", card.data.name);
        }
        return Ok(lorebook);
    }

    let json_str = std::str::from_utf8(bytes)?;
    // First, try standard Lorebook deserialization
    if let Ok(book) = serde_json::from_str::<Lorebook>(json_str)
        && (!book.name.is_empty() || !book.entries.is_empty())
    {
        return Ok(book);
    }

    // Next, check if it's a CharacterBook
    if let Ok(char_book) = serde_json::from_str::<CharacterBook>(json_str)
        && (!char_book.entries.is_empty() || char_book.name.is_some())
    {
        return Ok(char_book.to_lorebook());
    }

    // Generic JSON AST inspection for SillyTavern formats
    let root_val: serde_json::Value = serde_json::from_str(json_str)?;
    let root_obj = match &root_val {
        serde_json::Value::Object(map) => map,
        _ => return Err(LorebookError::InvalidFormat),
    };

    // If wrapped in "character_book" or "world_info"
    let effective_obj = if let Some(serde_json::Value::Object(inner)) = root_obj
        .get("character_book")
        .or_else(|| root_obj.get("world_info"))
    {
        inner
    } else {
        root_obj
    };

    let name = effective_obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Imported Lorebook")
        .to_string();

    let description = effective_obj
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let scan_depth = effective_obj
        .get("scan_depth")
        .and_then(|v| v.as_u64())
        .unwrap_or(2) as usize;

    let token_budget = effective_obj
        .get("token_budget")
        .and_then(|v| v.as_u64())
        .unwrap_or(2048) as usize;

    let recursive_scanning = effective_obj
        .get("recursive_scanning")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let global = effective_obj
        .get("global")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut entries = Vec::new();

    // SillyTavern entries can be either an Array or an Object/Map of entries
    if let Some(entries_val) = effective_obj.get("entries") {
        match entries_val {
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(entry) = parse_single_entry(item) {
                        entries.push(entry);
                    }
                }
            }
            serde_json::Value::Object(map) => {
                // Entries keyed by ID string or index
                let mut sorted_keys: Vec<_> = map.keys().collect();
                sorted_keys.sort_by_key(|k| k.parse::<i64>().unwrap_or(0));
                for k in sorted_keys {
                    if let Some(item) = map.get(k)
                        && let Some(entry) = parse_single_entry(item)
                    {
                        entries.push(entry);
                    }
                }
            }
            _ => {}
        }
    }

    let now = Utc::now();
    Ok(Lorebook {
        id: Uuid::new_v4().to_string(),
        name,
        description,
        scan_depth,
        token_budget,
        recursive_scanning,
        global,
        entries,
        created_at: now,
        updated_at: now,
        extensions: effective_obj
            .get("extensions")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
    })
}

fn parse_single_entry(val: &serde_json::Value) -> Option<LorebookEntry> {
    let obj = val.as_object()?;

    let id = obj
        .get("id")
        .or_else(|| obj.get("uid"))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                Some(s.to_string())
            } else {
                v.as_i64().map(|n| n.to_string())
            }
        })
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // SillyTavern uses "key" or "keys"
    let keys = extract_string_list(obj.get("keys").or_else(|| obj.get("key")));

    // SillyTavern uses "secondary_keys" or "keysecondary"
    let secondary_keys = extract_string_list(
        obj.get("secondary_keys")
            .or_else(|| obj.get("keysecondary")),
    );

    let content = obj
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let comment = obj
        .get("comment")
        .or_else(|| obj.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Enabled: in ST, "disable": true means disabled
    let enabled = if let Some(d) = obj.get("disable").and_then(|v| v.as_bool()) {
        !d
    } else {
        obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true)
    };

    let constant = obj
        .get("constant")
        .or_else(|| obj.get("always_active"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let selective = obj
        .get("selective")
        .and_then(|v| v.as_bool())
        .unwrap_or(!secondary_keys.is_empty());

    let selective_logic = obj
        .get("selective_logic")
        .or_else(|| obj.get("selectiveLogic"))
        .map(|v| match v {
            serde_json::Value::Number(n) => match n.as_u64() {
                Some(1) => SelectiveLogic::NotAny,
                Some(2) => SelectiveLogic::AndAll,
                Some(3) => SelectiveLogic::NotAll,
                _ => SelectiveLogic::AndAny,
            },
            serde_json::Value::String(s) => match s.to_lowercase().as_str() {
                "not_any" | "notany" | "1" => SelectiveLogic::NotAny,
                "and_all" | "andall" | "2" => SelectiveLogic::AndAll,
                "not_all" | "notall" | "3" => SelectiveLogic::NotAll,
                _ => SelectiveLogic::AndAny,
            },
            _ => SelectiveLogic::AndAny,
        })
        .unwrap_or(SelectiveLogic::AndAny);

    let position = if let Some(pos_val) = obj
        .get("position")
        .or_else(|| obj.get("insertion_strategy"))
    {
        if let Some(s) = pos_val.as_str() {
            LorebookPosition::from_str_loose(s)
        } else if let Some(n) = pos_val.as_i64() {
            LorebookPosition::from_str_loose(&n.to_string())
        } else {
            LorebookPosition::BeforeChar
        }
    } else {
        LorebookPosition::BeforeChar
    };

    let depth = obj.get("depth").and_then(|v| v.as_u64()).unwrap_or(4) as usize;

    let order = obj
        .get("order")
        .or_else(|| obj.get("insertion_order"))
        .and_then(|v| v.as_i64())
        .unwrap_or(100) as i32;

    let case_sensitive = obj
        .get("case_sensitive")
        .or_else(|| obj.get("caseSensitive"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let use_regex = obj
        .get("use_regex")
        .or_else(|| obj.get("useRegex"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let prevent_recursion = obj
        .get("prevent_recursion")
        .or_else(|| obj.get("preventRecursion"))
        .or_else(|| obj.get("exclude_recursion"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let scan_depth = obj
        .get("scan_depth")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);

    Some(LorebookEntry {
        id,
        keys,
        secondary_keys,
        content,
        comment,
        enabled,
        constant,
        selective,
        selective_logic,
        position,
        depth,
        order,
        case_sensitive,
        use_regex,
        prevent_recursion,
        scan_depth,
        extensions: obj
            .get("extensions")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
    })
}

// --- Matching and Activation Engine ---

/// Tests whether a single key matches against the scan text.
pub fn is_key_match(key: &str, text: &str, case_sensitive: bool, use_regex: bool) -> bool {
    let trimmed_key = key.trim();
    if trimmed_key.is_empty() || text.is_empty() {
        return false;
    }

    if use_regex {
        let pattern = if case_sensitive {
            Regex::new(trimmed_key)
        } else {
            RegexBuilder::new(trimmed_key)
                .case_insensitive(true)
                .build()
        };

        if let Ok(re) = pattern {
            return re.is_match(text);
        }
        return false;
    }

    // Plain text matching with word boundary / substring semantics.
    // In SillyTavern, standard keyword matching checks for whole word boundary unless regex.
    let escaped_key = regex::escape(trimmed_key);
    // \b word boundary works well for alphanumeric keys; for keys with symbols or spaces, relaxed boundary is used
    let pattern_str = if trimmed_key
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        format!(r"\b{}\b", escaped_key)
    } else {
        escaped_key
    };

    let re_res = if case_sensitive {
        Regex::new(&pattern_str)
    } else {
        RegexBuilder::new(&pattern_str)
            .case_insensitive(true)
            .build()
    };

    if let Ok(re) = re_res {
        re.is_match(text)
    } else {
        if case_sensitive {
            text.contains(trimmed_key)
        } else {
            text.to_lowercase().contains(&trimmed_key.to_lowercase())
        }
    }
}

/// Evaluates whether an entry triggers based on its keys and selective logic against the scan text.
pub fn evaluate_entry(entry: &LorebookEntry, scan_text: &str) -> bool {
    if !entry.enabled {
        return false;
    }

    if entry.constant {
        return true;
    }

    if entry.keys.is_empty() {
        return false;
    }

    // 1. Primary keys check: at least ONE primary key must match
    let primary_matched = entry
        .keys
        .iter()
        .any(|k| is_key_match(k, scan_text, entry.case_sensitive, entry.use_regex));

    if !primary_matched {
        return false;
    }

    // 2. Secondary keys check (if selective is enabled and secondary keys exist)
    if entry.selective && !entry.secondary_keys.is_empty() {
        let matched_count = entry
            .secondary_keys
            .iter()
            .filter(|k| is_key_match(k, scan_text, entry.case_sensitive, entry.use_regex))
            .count();
        let total_count = entry.secondary_keys.len();

        match entry.selective_logic {
            SelectiveLogic::AndAny => {
                if matched_count == 0 {
                    return false;
                }
            }
            SelectiveLogic::NotAny => {
                if matched_count > 0 {
                    return false;
                }
            }
            SelectiveLogic::AndAll => {
                if matched_count != total_count {
                    return false;
                }
            }
            SelectiveLogic::NotAll => {
                if matched_count == total_count {
                    return false;
                }
            }
        }
    }

    true
}

#[derive(Debug, Clone)]
pub struct ActivatedEntry {
    pub id: String,
    pub content: String,
    pub comment: String,
    pub position: LorebookPosition,
    pub depth: usize,
    pub order: i32,
}

/// Evaluates active lorebooks against chat history and returns the list of activated entries.
pub fn scan_lorebooks_for_activation(
    lorebooks: &[&Lorebook],
    chat_messages: &[String], // recent message contents
    user_input: &str,         // current user input if any
) -> Vec<ActivatedEntry> {
    if lorebooks.is_empty() {
        return Vec::new();
    }

    let mut activated_map: HashMap<String, (LorebookEntry, usize)> = HashMap::new(); // id -> (entry, token_budget)
    let mut recursive_enabled = false;
    let mut total_token_budget = 0;

    for book in lorebooks {
        if book.recursive_scanning {
            recursive_enabled = true;
        }
        if book.token_budget > 0 {
            total_token_budget = total_token_budget.max(book.token_budget);
        }

        // Determine scan text for this lorebook
        let messages_to_scan = if book.scan_depth == 0 || book.scan_depth >= chat_messages.len() {
            chat_messages
        } else {
            &chat_messages[chat_messages.len().saturating_sub(book.scan_depth)..]
        };

        let mut scan_parts = Vec::new();
        for msg in messages_to_scan {
            scan_parts.push(msg.as_str());
        }
        if !user_input.trim().is_empty() {
            scan_parts.push(user_input);
        }
        let scan_text = scan_parts.join("\n");

        for entry in &book.entries {
            // Check entry-level scan depth if present
            let effective_scan_text = if let Some(depth) = entry.scan_depth {
                if depth > 0 && depth < chat_messages.len() {
                    let subset = &chat_messages[chat_messages.len().saturating_sub(depth)..];
                    let mut p = Vec::new();
                    for m in subset {
                        p.push(m.as_str());
                    }
                    if !user_input.trim().is_empty() {
                        p.push(user_input);
                    }
                    p.join("\n")
                } else {
                    scan_text.clone()
                }
            } else {
                scan_text.clone()
            };

            if evaluate_entry(entry, &effective_scan_text) {
                activated_map.insert(entry.id.clone(), (entry.clone(), book.token_budget));
            }
        }
    }

    // Recursive scanning: scan newly activated entries against unactivated ones
    if recursive_enabled {
        let max_rounds = 5;
        let mut checked_ids: HashSet<String> = HashSet::new();

        for _ in 0..max_rounds {
            let newly_activated: Vec<LorebookEntry> = activated_map
                .values()
                .filter(|(entry, _)| !entry.prevent_recursion && !checked_ids.contains(&entry.id))
                .map(|(entry, _)| entry.clone())
                .collect();

            if newly_activated.is_empty() {
                break;
            }

            for e in &newly_activated {
                checked_ids.insert(e.id.clone());
            }

            let new_text = newly_activated
                .iter()
                .map(|e| e.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");

            if new_text.trim().is_empty() {
                break;
            }

            let mut found_new = false;
            for book in lorebooks {
                for entry in &book.entries {
                    if !activated_map.contains_key(&entry.id) && evaluate_entry(entry, &new_text) {
                        activated_map.insert(entry.id.clone(), (entry.clone(), book.token_budget));
                        found_new = true;
                    }
                }
            }

            if !found_new {
                break;
            }
        }
    }

    // Convert to ActivatedEntry list and sort by order
    let mut result: Vec<ActivatedEntry> = activated_map
        .into_values()
        .map(|(e, _)| ActivatedEntry {
            id: e.id,
            content: e.content,
            comment: e.comment,
            position: e.position,
            depth: e.depth,
            order: e.order,
        })
        .collect();

    // Sort by insertion order (lower numbers first, or higher numbers first; SillyTavern default is ascending order)
    result.sort_by_key(|e| e.order);

    // Apply token budget if configured
    if total_token_budget > 0 {
        let mut current_tokens = 0;
        let mut budgeted_result = Vec::new();
        for item in result {
            let tokens = (item.content.chars().count() + 3).div_ceil(4);
            if current_tokens + tokens <= total_token_budget || budgeted_result.is_empty() {
                current_tokens += tokens;
                budgeted_result.push(item);
            } else {
                break;
            }
        }
        budgeted_result
    } else {
        result
    }
}
