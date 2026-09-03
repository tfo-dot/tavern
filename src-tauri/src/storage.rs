use chrono::{DateTime, Utc};
use engine::character::{Character, CharacterCardV2, UserPersona};
use engine::chat::{export_sillytavern_chat_jsonl, import_sillytavern_chat_jsonl, ChatTree, Group};
use engine::crdt::TavernCrdtDoc;
use engine::lorebook::{parse_lorebook, Lorebook};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;
use uuid::Uuid;

use crate::sync::protocol::SyncStats;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub endpoint: String,
    pub api_key: String,
    pub active_model: String,
    pub temperature: f32,
    pub top_p: f32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
    pub max_tokens: usize,
    pub max_context_tokens: usize,
    pub system_template: String,
    pub stop_sequences: Vec<String>,
    pub active_character_id: Option<String>,
    pub active_chat_id: Option<String>,
    #[serde(default)]
    pub active_persona_id: Option<String>,
    #[serde(default)]
    pub global_lorebook_ids: Vec<String>,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub sync_port: Option<u16>,
    #[serde(default)]
    pub sync_pin: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434/v1".to_string(),
            api_key: "none".to_string(),
            active_model: "".to_string(),
            temperature: 0.7,
            top_p: 0.9,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 800,
            max_context_tokens: 4096,
            system_template: "Write {{char}}'s next reply in a fictional roleplay chat between {{char}} and {{user}}.\nFollow character traits, scenario, and tone strictly. Stay in character.".to_string(),
            stop_sequences: Vec::new(),
            active_character_id: None,
            active_chat_id: None,
            active_persona_id: None,
            global_lorebook_ids: Vec::new(),
            device_name: None,
            sync_port: None,
            sync_pin: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSummary {
    pub id: String,
    pub character_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub last_message_preview: String,
    #[serde(default)]
    pub group_id: Option<String>,
}

pub struct StorageManager {
    base_dir: PathBuf,
}

impl StorageManager {
    pub fn new(base_dir: PathBuf) -> Self {
        let mgr = Self { base_dir };
        mgr.init_directories();
        mgr.ensure_starter_data();
        mgr.ensure_starter_persona();
        mgr
    }

    fn characters_dir(&self) -> PathBuf {
        self.base_dir.join("characters")
    }

    fn chats_dir(&self) -> PathBuf {
        self.base_dir.join("chats")
    }

    fn personas_dir(&self) -> PathBuf {
        self.base_dir.join("personas")
    }

    fn lorebooks_dir(&self) -> PathBuf {
        self.base_dir.join("lorebooks")
    }

    fn groups_dir(&self) -> PathBuf {
        self.base_dir.join("groups")
    }
    fn settings_path(&self) -> PathBuf {
        self.base_dir.join("settings.json")
    }

    fn legacy_user_persona_path(&self) -> PathBuf {
        self.base_dir.join("user_persona.json")
    }

    fn init_directories(&self) {
        let _ = fs::create_dir_all(self.characters_dir());
        let _ = fs::create_dir_all(self.chats_dir());
        let _ = fs::create_dir_all(self.personas_dir());
        let _ = fs::create_dir_all(self.lorebooks_dir());
        let _ = fs::create_dir_all(self.groups_dir());
    }

    fn ensure_starter_data(&self) {
        if let Ok(entries) = fs::read_dir(self.characters_dir()) {
            let count = entries.count();
            if count == 0 {
                let mut card = CharacterCardV2::default();
                card.data.name = "Seraphina".to_string();
                card.data.description = "A warm, enigmatic tavern keeper with silver hair and amethyst eyes. She runs the Starlight Tavern, a mystical refuge between worlds where travelers rest and share their stories.".to_string();
                card.data.personality =
                    "Empathetic, witty, observant, gentle yet possessing ancient arcane knowledge."
                        .to_string();
                card.data.scenario = "{{user}} pushes open the heavy oak doors of the Starlight Tavern on a stormy night, seeking shelter from the bitter cold.".to_string();
                card.data.first_mes = "*Rain lashes against the stained glass windows as the heavy oak door creaks open. Behind the polished mahogany counter, Seraphina looks up from polishing a crystal glass, her amethyst eyes sparkling warmly.*\n\n\"Welcome, traveler! Step inside and warm yourself by the hearth. Come, take a seat. What tale or drink brings you to my tavern tonight?\"".to_string();
                card.data.alternate_greetings = vec![
                    "*The tavern is quiet tonight, soft lute music playing from an unseen corner. Seraphina smiles gently as you step in.*\n\n\"Back again, traveler? I just brewed fresh spiced cider. Pull up a chair and let's catch up.\"".to_string()
                ];
                card.data.tags = vec![
                    "Fantasy".to_string(),
                    "Tavern".to_string(),
                    "Mystical".to_string(),
                    "Friendly".to_string(),
                ];

                let char_obj = Character::from_card(card, None);
                let _ = self.save_character(&char_obj);
            }
        }
    }

    fn ensure_starter_persona(&self) {
        // Check if legacy user_persona.json exists and migrate it
        if self.legacy_user_persona_path().exists() {
            if let Ok(content) = fs::read_to_string(self.legacy_user_persona_path()) {
                if let Ok(persona) = serde_json::from_str::<UserPersona>(&content) {
                    let _ = self.save_user_persona(&persona);
                }
            }
        }

        // If personas dir is empty, create default persona
        if let Ok(entries) = fs::read_dir(self.personas_dir()) {
            if entries.count() == 0 {
                let default_p = UserPersona {
                    id: "default_user".to_string(),
                    name: "User".to_string(),
                    description: "".to_string(),
                    avatar_data_url: None,
                };
                let _ = self.save_user_persona(&default_p);
            }
        }
    }

    // --- Characters ---

    pub fn save_character(&self, character: &Character) -> Result<(), String> {
        let path = self.characters_dir().join(format!("{}.json", character.id));
        let data = serde_json::to_string_pretty(character).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())?;
        if let Ok(doc) = self.load_crdt_doc() {
            let _ = doc.set_character(character);
            let _ = self.save_crdt_doc(&doc);
        }
        Ok(())
    }

    pub fn load_character(&self, id: &str) -> Result<Character, String> {
        let path = self.characters_dir().join(format!("{id}.json"));
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn list_characters(&self) -> Result<Vec<Character>, String> {
        let mut characters = Vec::new();
        if let Ok(entries) = fs::read_dir(self.characters_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(character) = serde_json::from_str::<Character>(&content) {
                            characters.push(character);
                        }
                    }
                }
            }
        }
        characters.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(characters)
    }

    pub fn delete_character(&self, id: &str) -> Result<(), String> {
        let path = self.characters_dir().join(format!("{id}.json"));
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        if let Ok(doc) = self.load_crdt_doc() {
            let _ = doc.delete_character(id);
            let _ = self.save_crdt_doc(&doc);
        }
        if let Ok(chats) = self.list_chats_for_character(id) {
            for chat in chats {
                let _ = self.delete_chat(&chat.id);
            }
        }
        Ok(())
    }

    // --- Chats ---

    pub fn save_chat(&self, chat: &ChatTree) -> Result<(), String> {
        let path = self.chats_dir().join(format!("{}.json", chat.id));
        let data = serde_json::to_string_pretty(chat).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())?;
        if let Ok(doc) = self.load_crdt_doc() {
            let _ = doc.set_chat(chat);
            let _ = self.save_crdt_doc(&doc);
        }
        Ok(())
    }

    pub fn load_chat(&self, id: &str) -> Result<ChatTree, String> {
        let path = self.chats_dir().join(format!("{id}.json"));
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn list_chats_for_character(&self, character_id: &str) -> Result<Vec<ChatSummary>, String> {
        let mut summaries = Vec::new();
        if let Ok(entries) = fs::read_dir(self.chats_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(chat) = serde_json::from_str::<ChatTree>(&content) {
                            if chat.character_id == character_id {
                                let (msg_count, preview) = {
                                    let active_path = chat.get_active_path();
                                    let count = active_path.len();
                                    let prev = active_path
                                        .last()
                                        .map(|m| {
                                            let c = m.content.chars().take(80).collect::<String>();
                                            if m.content.chars().count() > 80 {
                                                format!("{c}...")
                                            } else {
                                                c
                                            }
                                        })
                                        .unwrap_or_else(|| "Empty chat".to_string());
                                    (count, prev)
                                };

                                summaries.push(ChatSummary {
                                    id: chat.id.to_string(),
                                    character_id: chat.character_id,
                                    title: chat.title,
                                    created_at: chat.created_at,
                                    updated_at: chat.updated_at,
                                    message_count: msg_count,
                                    last_message_preview: preview,
                                    group_id: chat.group_id,
                                });
                            }
                        }
                    }
                }
            }
        }
        summaries.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(summaries)
    }

    pub fn list_chats_for_group(&self, group_id: &str) -> Result<Vec<ChatSummary>, String> {
        let mut summaries = Vec::new();
        if let Ok(entries) = fs::read_dir(self.chats_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(chat) = serde_json::from_str::<ChatTree>(&content) {
                            if chat.group_id.as_deref() == Some(group_id) {
                                let (msg_count, preview) = {
                                    let active_path = chat.get_active_path();
                                    let count = active_path.len();
                                    let prev = active_path
                                        .last()
                                        .map(|m| {
                                            let c = m.content.chars().take(80).collect::<String>();
                                            if m.content.chars().count() > 80 {
                                                format!("{c}...")
                                            } else {
                                                c
                                            }
                                        })
                                        .unwrap_or_else(|| "Empty chat".to_string());
                                    (count, prev)
                                };

                                summaries.push(ChatSummary {
                                    id: chat.id.to_string(),
                                    character_id: chat.character_id,
                                    title: chat.title,
                                    created_at: chat.created_at,
                                    updated_at: chat.updated_at,
                                    message_count: msg_count,
                                    last_message_preview: preview,
                                    group_id: chat.group_id,
                                });
                            }
                        }
                    }
                }
            }
        }
        summaries.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(summaries)
    }

    // --- Groups ---

    pub fn save_group(&self, group: &Group) -> Result<(), String> {
        let path = self.groups_dir().join(format!("{}.json", group.id));
        let data = serde_json::to_string_pretty(group).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())?;
        if let Ok(doc) = self.load_crdt_doc() {
            let _ = doc.set_group(group);
            let _ = self.save_crdt_doc(&doc);
        }
        Ok(())
    }

    pub fn load_group(&self, id: &str) -> Result<Group, String> {
        let path = self.groups_dir().join(format!("{id}.json"));
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn list_groups(&self) -> Result<Vec<Group>, String> {
        let mut groups = Vec::new();
        if let Ok(entries) = fs::read_dir(self.groups_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(group) = serde_json::from_str::<Group>(&content) {
                            groups.push(group);
                        }
                    }
                }
            }
        }
        groups.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(groups)
    }

    pub fn delete_group(&self, id: &str) -> Result<(), String> {
        let path = self.groups_dir().join(format!("{id}.json"));
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        if let Ok(doc) = self.load_crdt_doc() {
            let _ = doc.delete_group(id);
            let _ = self.save_crdt_doc(&doc);
        }
        if let Ok(chats) = self.list_chats_for_group(id) {
            for chat in chats {
                let _ = self.delete_chat(&chat.id);
            }
        }
        Ok(())
    }

    pub fn delete_chat(&self, id: &str) -> Result<(), String> {
        let path = self.chats_dir().join(format!("{id}.json"));
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        if let Ok(doc) = self.load_crdt_doc() {
            if let Ok(uuid_val) = Uuid::parse_str(id) {
                let _ = doc.delete_chat(&uuid_val);
                let _ = self.save_crdt_doc(&doc);
            }
        }
        Ok(())
    }
    pub fn import_chat_jsonl(
        &self,
        file_bytes: &[u8],
        target_character_id: Option<&str>,
        title: Option<&str>,
    ) -> Result<ChatTree, String> {
        let content = std::str::from_utf8(file_bytes)
            .map_err(|e| format!("Invalid UTF-8 encoding in chat file: {e}"))?;

        // If target_character_id is not specified, attempt to infer from character name or list
        let char_id = if let Some(id) = target_character_id {
            id.to_string()
        } else {
            let first_line = content.lines().next().unwrap_or("").trim();
            let matched_id = if let Ok(val) = serde_json::from_str::<serde_json::Value>(first_line)
            {
                if let Some(char_name) = val.get("character_name").and_then(|v| v.as_str()) {
                    if let Ok(characters) = self.list_characters() {
                        characters
                            .into_iter()
                            .find(|c| c.card.data.name.eq_ignore_ascii_case(char_name))
                            .map(|c| c.id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            matched_id
                .or_else(|| {
                    self.list_characters()
                        .ok()
                        .and_then(|chars| chars.into_iter().next().map(|c| c.id))
                })
                .unwrap_or_else(|| "default".to_string())
        };

        let tree = import_sillytavern_chat_jsonl(content, &char_id, title)?;
        self.save_chat(&tree)?;
        Ok(tree)
    }

    pub fn export_chat_jsonl(
        &self,
        chat_id: &str,
        user_name: &str,
        character_name: &str,
    ) -> Result<String, String> {
        let chat = self.load_chat(chat_id)?;
        export_sillytavern_chat_jsonl(&chat, user_name, character_name)
    }

    // --- Settings ---

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), String> {
        let data = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        fs::write(self.settings_path(), data).map_err(|e| e.to_string())
    }

    pub fn load_settings(&self) -> AppSettings {
        if let Ok(content) = fs::read_to_string(self.settings_path()) {
            if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                return settings;
            }
        }
        let default_settings = AppSettings::default();
        let _ = self.save_settings(&default_settings);
        default_settings
    }

    // --- User Personas (Multi-Persona) ---

    pub fn save_user_persona(&self, persona: &UserPersona) -> Result<(), String> {
        let path = self.personas_dir().join(format!("{}.json", persona.id));
        let data = serde_json::to_string_pretty(persona).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())?;
        if let Ok(doc) = self.load_crdt_doc() {
            let _ = doc.set_persona(persona);
            let _ = self.save_crdt_doc(&doc);
        }
        Ok(())
    }

    pub fn load_user_persona(&self, id: &str) -> Result<UserPersona, String> {
        let path = self.personas_dir().join(format!("{id}.json"));
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn list_user_personas(&self) -> Result<Vec<UserPersona>, String> {
        let mut personas = Vec::new();
        if let Ok(entries) = fs::read_dir(self.personas_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(persona) = serde_json::from_str::<UserPersona>(&content) {
                            personas.push(persona);
                        }
                    }
                }
            }
        }
        if personas.is_empty() {
            let default_p = UserPersona::default();
            let _ = self.save_user_persona(&default_p);
            personas.push(default_p);
        }
        Ok(personas)
    }

    pub fn delete_user_persona(&self, id: &str) -> Result<(), String> {
        let path = self.personas_dir().join(format!("{id}.json"));
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        if let Ok(doc) = self.load_crdt_doc() {
            let _ = doc.delete_persona(id);
            let _ = self.save_crdt_doc(&doc);
        }
        Ok(())
    }

    // --- Lorebooks (World Info) ---

    pub fn save_lorebook(&self, lorebook: &Lorebook) -> Result<(), String> {
        let path = self.lorebooks_dir().join(format!("{}.json", lorebook.id));
        let data = serde_json::to_string_pretty(lorebook).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())?;
        if let Ok(doc) = self.load_crdt_doc() {
            let _ = doc.set_lorebook(lorebook);
            let _ = self.save_crdt_doc(&doc);
        }
        Ok(())
    }

    pub fn load_lorebook(&self, id: &str) -> Result<Lorebook, String> {
        let path = self.lorebooks_dir().join(format!("{id}.json"));
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn list_lorebooks(&self) -> Result<Vec<Lorebook>, String> {
        let mut lorebooks = Vec::new();
        if let Ok(entries) = fs::read_dir(self.lorebooks_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(book) = serde_json::from_str::<Lorebook>(&content) {
                            lorebooks.push(book);
                        }
                    }
                }
            }
        }
        lorebooks.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(lorebooks)
    }

    pub fn delete_lorebook(&self, id: &str) -> Result<(), String> {
        let path = self.lorebooks_dir().join(format!("{id}.json"));
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        if let Ok(doc) = self.load_crdt_doc() {
            let _ = doc.delete_lorebook(id);
            let _ = self.save_crdt_doc(&doc);
        }
        Ok(())
    }

    pub fn import_lorebook(&self, file_bytes: &[u8]) -> Result<Lorebook, String> {
        let book = parse_lorebook(file_bytes).map_err(|e| e.to_string())?;
        self.save_lorebook(&book)?;
        Ok(book)
    }

    pub fn export_lorebook_json(&self, id: &str) -> Result<String, String> {
        let book = self.load_lorebook(id)?;
        serde_json::to_string_pretty(&book).map_err(|e| e.to_string())
    }

    // --- CRDT Sync Integration ---

    pub fn crdt_doc_path(&self) -> PathBuf {
        self.base_dir.join("crdt_doc.bin")
    }

    pub fn load_crdt_doc(&self) -> Result<TavernCrdtDoc, String> {
        let path = self.crdt_doc_path();
        if path.exists() {
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            TavernCrdtDoc::from_snapshot(&bytes)
        } else {
            let doc = TavernCrdtDoc::new();
            self.populate_crdt_from_disk(&doc)?;
            let _ = self.save_crdt_doc(&doc);
            Ok(doc)
        }
    }

    pub fn save_crdt_doc(&self, doc: &TavernCrdtDoc) -> Result<(), String> {
        let bytes = doc.export_snapshot()?;
        fs::write(self.crdt_doc_path(), bytes).map_err(|e| e.to_string())
    }

    pub fn populate_crdt_from_disk(&self, doc: &TavernCrdtDoc) -> Result<(), String> {
        if let Ok(characters) = self.list_characters() {
            for c in characters {
                let _ = doc.set_character(&c);
            }
        }
        if let Ok(entries) = fs::read_dir(self.chats_dir()) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(chat) = serde_json::from_str::<ChatTree>(&content) {
                            let _ = doc.set_chat(&chat);
                        }
                    }
                }
            }
        }
        if let Ok(personas) = self.list_user_personas() {
            for p in personas {
                let _ = doc.set_persona(&p);
            }
        }
        if let Ok(lorebooks) = self.list_lorebooks() {
            for b in lorebooks {
                let _ = doc.set_lorebook(&b);
            }
        }
        if let Ok(groups) = self.list_groups() {
            for g in groups {
                let _ = doc.set_group(&g);
            }
        }
        doc.commit();
        Ok(())
    }

    pub fn sync_disk_from_crdt(&self, doc: &TavernCrdtDoc) -> Result<SyncStats, String> {
        let mut stats = SyncStats::default();
        let tombstones = doc.get_tombstones().unwrap_or_default();

        // 1. Characters
        if let Ok(characters) = doc.get_characters() {
            for c in characters {
                let tomb_key = format!("character:{}", c.id);
                if let Some(tomb) = tombstones.get(&tomb_key) {
                    if tomb.deleted_at >= c.updated_at {
                        let path = self.characters_dir().join(format!("{}.json", c.id));
                        if path.exists() {
                            let _ = fs::remove_file(path);
                        }
                        continue;
                    }
                }
                let should_write = match self.load_character(&c.id) {
                    Ok(existing) => c.updated_at > existing.updated_at,
                    Err(_) => true,
                };
                if should_write {
                    let path = self.characters_dir().join(format!("{}.json", c.id));
                    if let Ok(data) = serde_json::to_string_pretty(&c) {
                        let _ = fs::write(path, data);
                        stats.characters_synced += 1;
                    }
                }
            }
        }

        // 2. Chats
        if let Ok(chats) = doc.get_chats() {
            for chat in chats {
                let tomb_key = format!("chat:{}", chat.id);
                if let Some(tomb) = tombstones.get(&tomb_key) {
                    if tomb.deleted_at >= chat.updated_at {
                        let path = self.chats_dir().join(format!("{}.json", chat.id));
                        if path.exists() {
                            let _ = fs::remove_file(path);
                        }
                        continue;
                    }
                }
                let should_write = match self.load_chat(&chat.id.to_string()) {
                    Ok(existing) => chat.updated_at > existing.updated_at,
                    Err(_) => true,
                };
                if should_write {
                    let path = self.chats_dir().join(format!("{}.json", chat.id));
                    if let Ok(data) = serde_json::to_string_pretty(&chat) {
                        let _ = fs::write(path, data);
                        stats.chats_synced += 1;
                    }
                }
            }
        }

        // 3. Personas
        if let Ok(personas) = doc.get_personas() {
            for p in personas {
                let tomb_key = format!("persona:{}", p.id);
                if let Some(_tomb) = tombstones.get(&tomb_key) {
                    let path = self.personas_dir().join(format!("{}.json", p.id));
                    if path.exists() {
                        let _ = fs::remove_file(path);
                    }
                    continue;
                }
                let path = self.personas_dir().join(format!("{}.json", p.id));
                if let Ok(data) = serde_json::to_string_pretty(&p) {
                    let _ = fs::write(path, data);
                    stats.personas_synced += 1;
                }
            }
        }

        // 4. Lorebooks
        if let Ok(lorebooks) = doc.get_lorebooks() {
            for b in lorebooks {
                let tomb_key = format!("lorebook:{}", b.id);
                if let Some(tomb) = tombstones.get(&tomb_key) {
                    if tomb.deleted_at >= b.updated_at {
                        let path = self.lorebooks_dir().join(format!("{}.json", b.id));
                        if path.exists() {
                            let _ = fs::remove_file(path);
                        }
                        continue;
                    }
                }
                let should_write = match self.load_lorebook(&b.id) {
                    Ok(existing) => b.updated_at > existing.updated_at,
                    Err(_) => true,
                };
                if should_write {
                    let path = self.lorebooks_dir().join(format!("{}.json", b.id));
                    if let Ok(data) = serde_json::to_string_pretty(&b) {
                        let _ = fs::write(path, data);
                        stats.lorebooks_synced += 1;
                    }
                }
            }
        }

        // 5. Groups
        if let Ok(groups) = doc.get_groups() {
            for g in groups {
                let tomb_key = format!("group:{}", g.id);
                if let Some(tomb) = tombstones.get(&tomb_key) {
                    if tomb.deleted_at >= g.updated_at {
                        let path = self.groups_dir().join(format!("{}.json", g.id));
                        if path.exists() {
                            let _ = fs::remove_file(path);
                        }
                        continue;
                    }
                }
                let should_write = match self.load_group(&g.id) {
                    Ok(existing) => g.updated_at > existing.updated_at,
                    Err(_) => true,
                };
                if should_write {
                    let path = self.groups_dir().join(format!("{}.json", g.id));
                    if let Ok(data) = serde_json::to_string_pretty(&g) {
                        let _ = fs::write(path, data);
                    }
                }
            }
        }

        // Also sweep any local files matching active tombstones
        for key in tombstones.keys() {
            if let Some(id) = key.strip_prefix("character:") {
                let p = self.characters_dir().join(format!("{id}.json"));
                if p.exists() {
                    let _ = fs::remove_file(p);
                }
            } else if let Some(id) = key.strip_prefix("chat:") {
                let p = self.chats_dir().join(format!("{id}.json"));
                if p.exists() {
                    let _ = fs::remove_file(p);
                }
            } else if let Some(id) = key.strip_prefix("persona:") {
                let p = self.personas_dir().join(format!("{id}.json"));
                if p.exists() {
                    let _ = fs::remove_file(p);
                }
            } else if let Some(id) = key.strip_prefix("lorebook:") {
                let p = self.lorebooks_dir().join(format!("{id}.json"));
                if p.exists() {
                    let _ = fs::remove_file(p);
                }
            } else if let Some(id) = key.strip_prefix("group:") {
                let p = self.groups_dir().join(format!("{id}.json"));
                if p.exists() {
                    let _ = fs::remove_file(p);
                }
            }
        }

        stats.message = format!(
            "Synced {} characters, {} chats, {} personas, {} lorebooks",
            stats.characters_synced,
            stats.chats_synced,
            stats.personas_synced,
            stats.lorebooks_synced
        );
        Ok(stats)
    }
}
