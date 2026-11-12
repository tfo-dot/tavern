use chrono::{DateTime, Utc};
use engine::character::{Character, CharacterCardV2, UserPersona};
use engine::chat::{ChatTree, Group, export_sillytavern_chat_jsonl, import_sillytavern_chat_jsonl};
use engine::crdt::TavernCrdtDoc;
use engine::lorebook::{Lorebook, parse_lorebook};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use uuid::Uuid;

use crate::sync::protocol::SyncStats;

fn default_min_p() -> f32 {
    0.0
}

fn default_top_k() -> u32 {
    40
}

fn default_repetition_penalty() -> f32 {
    1.05
}

fn default_authors_note_depth() -> usize {
    1
}

fn default_authors_note_interval() -> usize {
    1
}
fn default_true() -> bool {
    true
}

fn default_rag_top_k() -> usize {
    3
}

fn default_rag_threshold() -> f32 {
    0.25
}

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
    #[serde(default = "default_min_p")]
    pub min_p: f32,
    #[serde(default = "default_top_k")]
    pub top_k: u32,
    #[serde(default = "default_repetition_penalty")]
    pub repetition_penalty: f32,
    #[serde(default)]
    pub api_type: Option<String>,
    #[serde(default)]
    pub authors_note: String,
    #[serde(default = "default_authors_note_depth")]
    pub authors_note_depth: usize,
    #[serde(default = "default_authors_note_interval")]
    pub authors_note_interval: usize,
    #[serde(default = "engine::regex_engine::default_regex_rules")]
    pub regex_rules: Vec<engine::regex_engine::RegexRule>,
    #[serde(default = "default_true")]
    pub vector_memory_enabled: bool,
    #[serde(default = "default_rag_top_k")]
    pub vector_memory_top_k: usize,
    #[serde(default = "default_rag_threshold")]
    pub vector_memory_threshold: f32,
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
            min_p: 0.0,
            top_k: 40,
            repetition_penalty: 1.05,
            api_type: None,
            authors_note: String::new(),
            authors_note_depth: 1,
            authors_note_interval: 1,
            regex_rules: engine::regex_engine::default_regex_rules(),
            vector_memory_enabled: true,
            vector_memory_top_k: 3,
            vector_memory_threshold: 0.25
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

enum CrdtJob {
    SetCharacter(Box<Character>),
    DeleteCharacterAndChats { id: String, chat_ids: Vec<Uuid> },
    SetChat(Box<ChatTree>),
    DeleteChat(Uuid),
    SetGroup(Box<Group>),
    DeleteGroupAndChats { id: String, chat_ids: Vec<Uuid> },
    SetPersona(Box<UserPersona>),
    DeletePersona(String),
    SetLorebook(Box<Lorebook>),
    DeleteLorebook(String),
    ReplaceSnapshot(Vec<u8>),
    Run(Box<dyn FnOnce(&TavernCrdtDoc) + Send>),
    RunMut(Box<dyn FnOnce(&TavernCrdtDoc) -> bool + Send>),
    Flush(Sender<()>),
    Shutdown,
}

pub struct StorageManager {
    base_dir: PathBuf,
    chats_index: Mutex<Option<HashMap<String, ChatSummary>>>,
    crdt_tx: Mutex<Option<Sender<CrdtJob>>>,
    crdt_worker: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for StorageManager {
    fn drop(&mut self) {
        if let Some(tx) = self.crdt_tx.lock().take() {
            let _ = tx.send(CrdtJob::Shutdown);
        }
        if let Some(handle) = self.crdt_worker.lock().take() {
            let _ = handle.join();
        }
    }
}

impl StorageManager {
    pub fn new(base_dir: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel();
        let worker_base = base_dir.clone();
        let handle = thread::Builder::new()
            .name("tavern-crdt-worker".to_string())
            .spawn(move || Self::crdt_worker_loop(worker_base, rx))
            .expect("Failed to spawn CRDT background worker");

        let mgr = Self {
            base_dir,
            chats_index: Mutex::new(None),
            crdt_tx: Mutex::new(Some(tx)),
            crdt_worker: Mutex::new(Some(handle)),
        };
        mgr.init_directories();
        mgr.ensure_starter_data();
        mgr.ensure_starter_persona();
        mgr
    }

    fn send_crdt_job(&self, job: CrdtJob) {
        if let Some(tx) = self.crdt_tx.lock().as_ref() {
            let _ = tx.send(job);
        }
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
        if self.legacy_user_persona_path().exists()
            && let Ok(content) = fs::read_to_string(self.legacy_user_persona_path())
            && let Ok(persona) = serde_json::from_str::<UserPersona>(&content)
        {
            let _ = self.save_user_persona(&persona);
        }

        // If personas dir is empty, create default persona
        if let Ok(entries) = fs::read_dir(self.personas_dir())
            && entries.count() == 0
        {
            let default_p = UserPersona {
                id: "default_user".to_string(),
                name: "User".to_string(),
                description: "".to_string(),
                avatar_data_url: None,
            };
            let _ = self.save_user_persona(&default_p);
        }
    }

    // --- Characters ---

    pub fn save_character(&self, character: &Character) -> Result<(), String> {
        let path = self.characters_dir().join(format!("{}.json", character.id));
        let data = serde_json::to_vec(character).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())?;
        self.send_crdt_job(CrdtJob::SetCharacter(Box::new(character.clone())));
        Ok(())
    }

    pub fn load_character(&self, id: &str) -> Result<Character, String> {
        let path = self.characters_dir().join(format!("{id}.json"));
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }

    pub fn list_characters(&self) -> Result<Vec<Character>, String> {
        let mut characters = load_json_dir_parallel::<Character>(&self.characters_dir());
        characters.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(characters)
    }

    pub fn delete_character(&self, id: &str) -> Result<(), String> {
        let path = self.characters_dir().join(format!("{id}.json"));
        if let Err(e) = fs::remove_file(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            return Err(e.to_string());
        }

        let matching_chats: Vec<(String, Option<Uuid>)> = {
            let mut guard = self.chats_index.lock();
            let index = self.ensure_chats_index(&mut guard);
            let keys: Vec<String> = index
                .iter()
                .filter(|(_, s)| s.character_id == id)
                .map(|(k, _)| k.clone())
                .collect();
            keys.into_iter()
                .map(|k| {
                    index.remove(&k);
                    let uuid_opt = Uuid::parse_str(&k).ok();
                    (k, uuid_opt)
                })
                .collect()
        };

        let mut chat_uuids = Vec::with_capacity(matching_chats.len());
        for (chat_id, uuid_opt) in matching_chats {
            let chat_path = self.chats_dir().join(format!("{chat_id}.json"));
            let _ = fs::remove_file(chat_path);
            if let Some(u) = uuid_opt {
                chat_uuids.push(u);
            }
        }

        self.send_crdt_job(CrdtJob::DeleteCharacterAndChats {
            id: id.to_string(),
            chat_ids: chat_uuids,
        });
        Ok(())
    }

    // --- Chats ---

    fn ensure_chats_index<'a>(
        &self,
        guard: &'a mut Option<HashMap<String, ChatSummary>>,
    ) -> &'a mut HashMap<String, ChatSummary> {
        guard.get_or_insert_with(|| {
            let chats = load_json_dir_parallel::<ChatTree>(&self.chats_dir());
            let mut map = HashMap::with_capacity(chats.len());
            for chat in chats {
                let summary = chat_to_summary(&chat);
                map.insert(summary.id.clone(), summary);
            }
            map
        })
    }

    pub fn save_chat(&self, chat: &ChatTree) -> Result<(), String> {
        let path = self.chats_dir().join(format!("{}.json", chat.id));
        let data = serde_json::to_vec(chat).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())?;
        {
            let mut guard = self.chats_index.lock();
            if let Some(index) = guard.as_mut() {
                let summary = chat_to_summary(chat);
                index.insert(summary.id.clone(), summary);
            }
        }
        self.send_crdt_job(CrdtJob::SetChat(Box::new(chat.clone())));
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
        let mut summaries: Vec<ChatSummary> = {
            let mut guard = self.chats_index.lock();
            let index = self.ensure_chats_index(&mut guard);
            index
                .values()
                .filter(|s| s.character_id == character_id)
                .cloned()
                .collect()
        };
        summaries.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(summaries)
    }

    pub fn list_chats_for_group(&self, group_id: &str) -> Result<Vec<ChatSummary>, String> {
        let mut summaries: Vec<ChatSummary> = {
            let mut guard = self.chats_index.lock();
            let index = self.ensure_chats_index(&mut guard);
            index
                .values()
                .filter(|s| s.group_id.as_deref() == Some(group_id))
                .cloned()
                .collect()
        };
        summaries.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(summaries)
    }

    // --- Groups ---

    pub fn save_group(&self, group: &Group) -> Result<(), String> {
        let path = self.groups_dir().join(format!("{}.json", group.id));
        let data = serde_json::to_vec(group).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())?;
        self.send_crdt_job(CrdtJob::SetGroup(Box::new(group.clone())));
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
                if path.extension().and_then(|s| s.to_str()) == Some("json")
                    && let Ok(content) = fs::read_to_string(&path)
                    && let Ok(group) = serde_json::from_str::<Group>(&content)
                {
                    groups.push(group);
                }
            }
        }
        groups.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(groups)
    }

    pub fn delete_group(&self, id: &str) -> Result<(), String> {
        let path = self.groups_dir().join(format!("{id}.json"));
        if let Err(e) = fs::remove_file(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            return Err(e.to_string());
        }

        let matching_chats: Vec<(String, Option<Uuid>)> = {
            let mut guard = self.chats_index.lock();
            let index = self.ensure_chats_index(&mut guard);
            let keys: Vec<String> = index
                .iter()
                .filter(|(_, s)| s.group_id.as_deref() == Some(id))
                .map(|(k, _)| k.clone())
                .collect();
            keys.into_iter()
                .map(|k| {
                    index.remove(&k);
                    let uuid_opt = Uuid::parse_str(&k).ok();
                    (k, uuid_opt)
                })
                .collect()
        };

        let mut chat_uuids = Vec::with_capacity(matching_chats.len());
        for (chat_id, uuid_opt) in matching_chats {
            let chat_path = self.chats_dir().join(format!("{chat_id}.json"));
            let _ = fs::remove_file(chat_path);
            if let Some(u) = uuid_opt {
                chat_uuids.push(u);
            }
        }

        self.send_crdt_job(CrdtJob::DeleteGroupAndChats {
            id: id.to_string(),
            chat_ids: chat_uuids,
        });
        Ok(())
    }

    pub fn delete_chat(&self, id: &str) -> Result<(), String> {
        let path = self.chats_dir().join(format!("{id}.json"));
        if let Err(e) = fs::remove_file(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            return Err(e.to_string());
        }
        {
            let mut guard = self.chats_index.lock();
            if let Some(index) = guard.as_mut() {
                index.remove(id);
            }
        }
        if let Ok(uuid_val) = Uuid::parse_str(id) {
            self.send_crdt_job(CrdtJob::DeleteChat(uuid_val));
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
        if let Ok(content) = fs::read_to_string(self.settings_path())
            && let Ok(settings) = serde_json::from_str::<AppSettings>(&content)
        {
            return settings;
        }
        let default_settings = AppSettings::default();
        let _ = self.save_settings(&default_settings);
        default_settings
    }

    // --- User Personas (Multi-Persona) ---

    pub fn save_user_persona(&self, persona: &UserPersona) -> Result<(), String> {
        let path = self.personas_dir().join(format!("{}.json", persona.id));
        let data = serde_json::to_vec(persona).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())?;
        self.send_crdt_job(CrdtJob::SetPersona(Box::new(persona.clone())));
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
                if path.extension().and_then(|s| s.to_str()) == Some("json")
                    && let Ok(content) = fs::read_to_string(&path)
                    && let Ok(persona) = serde_json::from_str::<UserPersona>(&content)
                {
                    personas.push(persona);
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
        if let Err(e) = fs::remove_file(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            return Err(e.to_string());
        }
        self.send_crdt_job(CrdtJob::DeletePersona(id.to_string()));
        Ok(())
    }

    // --- Lorebooks (World Info) ---

    pub fn save_lorebook(&self, lorebook: &Lorebook) -> Result<(), String> {
        let path = self.lorebooks_dir().join(format!("{}.json", lorebook.id));
        let data = serde_json::to_vec(lorebook).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())?;
        self.send_crdt_job(CrdtJob::SetLorebook(Box::new(lorebook.clone())));
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
                if path.extension().and_then(|s| s.to_str()) == Some("json")
                    && let Ok(content) = fs::read_to_string(&path)
                    && let Ok(book) = serde_json::from_str::<Lorebook>(&content)
                {
                    lorebooks.push(book);
                }
            }
        }
        lorebooks.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(lorebooks)
    }

    pub fn delete_lorebook(&self, id: &str) -> Result<(), String> {
        let path = self.lorebooks_dir().join(format!("{id}.json"));
        if let Err(e) = fs::remove_file(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            return Err(e.to_string());
        }
        self.send_crdt_job(CrdtJob::DeleteLorebook(id.to_string()));
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

    pub fn flush_crdt(&self) {
        let (tx, rx) = mpsc::channel();
        self.send_crdt_job(CrdtJob::Flush(tx));
        let _ = rx.recv();
    }

    pub fn with_crdt_doc<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&TavernCrdtDoc) -> Result<R, String> + Send + 'static,
        R: Send + 'static,
    {
        let (res_tx, res_rx) = mpsc::channel();
        self.send_crdt_job(CrdtJob::Run(Box::new(move |doc| {
            let _ = res_tx.send(f(doc));
        })));
        res_rx
            .recv()
            .map_err(|_| "CRDT worker unavailable".to_string())?
    }

    pub fn with_crdt_doc_mut<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&TavernCrdtDoc) -> Result<(R, bool), String> + Send + 'static,
        R: Send + 'static,
    {
        let (res_tx, res_rx) = mpsc::channel();
        self.send_crdt_job(CrdtJob::RunMut(Box::new(move |doc| match f(doc) {
            Ok((val, dirty)) => {
                let _ = res_tx.send(Ok(val));
                dirty
            }
            Err(e) => {
                let _ = res_tx.send(Err(e));
                false
            }
        })));
        res_rx
            .recv()
            .map_err(|_| "CRDT worker unavailable".to_string())?
    }

    pub fn load_crdt_doc(&self) -> Result<TavernCrdtDoc, String> {
        self.with_crdt_doc(|_| Ok(()))?;
        self.flush_crdt();
        let bytes = fs::read(self.crdt_doc_path()).map_err(|e| e.to_string())?;
        TavernCrdtDoc::from_snapshot(&bytes)
    }

    pub fn save_crdt_doc(&self, doc: &TavernCrdtDoc) -> Result<(), String> {
        let bytes = doc.export_snapshot()?;
        save_crdt_bytes_atomic(&self.crdt_doc_path(), &bytes)?;
        self.send_crdt_job(CrdtJob::ReplaceSnapshot(bytes));
        self.flush_crdt();
        Ok(())
    }

    fn crdt_worker_loop(base_dir: PathBuf, rx: mpsc::Receiver<CrdtJob>) {
        let crdt_path = base_dir.join("crdt_doc.bin");
        let mut doc_opt: Option<TavernCrdtDoc> = None;
        let mut dirty = false;

        loop {
            let first_job = if dirty {
                match rx.recv_timeout(Duration::from_millis(75)) {
                    Ok(j) => j,
                    Err(RecvTimeoutError::Timeout) => {
                        if let Some(doc) = doc_opt.as_ref() {
                            let _ = save_crdt_snapshot_atomic(&crdt_path, doc);
                        }
                        dirty = false;
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            } else {
                match rx.recv() {
                    Ok(j) => j,
                    Err(_) => break,
                }
            };

            let mut should_shutdown = false;
            let mut flush_waiters: Vec<Sender<()>> = Vec::new();

            let mut process = |job: CrdtJob| match job {
                CrdtJob::SetCharacter(c) => {
                    if let Some(doc) = ensure_crdt_doc(&base_dir, &crdt_path, &mut doc_opt)
                        && doc.set_character(&c).is_ok()
                    {
                        dirty = true;
                    }
                }
                CrdtJob::DeleteCharacterAndChats { id, chat_ids } => {
                    if let Some(doc) = ensure_crdt_doc(&base_dir, &crdt_path, &mut doc_opt) {
                        let _ = doc.delete_character(&id);
                        for chat_id in &chat_ids {
                            let _ = doc.delete_chat(chat_id);
                        }
                        dirty = true;
                    }
                }
                CrdtJob::SetChat(chat) => {
                    if let Some(doc) = ensure_crdt_doc(&base_dir, &crdt_path, &mut doc_opt)
                        && doc.set_chat(&chat).is_ok()
                    {
                        dirty = true;
                    }
                }
                CrdtJob::DeleteChat(uuid_val) => {
                    if let Some(doc) = ensure_crdt_doc(&base_dir, &crdt_path, &mut doc_opt)
                        && doc.delete_chat(&uuid_val).is_ok()
                    {
                        dirty = true;
                    }
                }
                CrdtJob::SetGroup(g) => {
                    if let Some(doc) = ensure_crdt_doc(&base_dir, &crdt_path, &mut doc_opt)
                        && doc.set_group(&g).is_ok()
                    {
                        dirty = true;
                    }
                }
                CrdtJob::DeleteGroupAndChats { id, chat_ids } => {
                    if let Some(doc) = ensure_crdt_doc(&base_dir, &crdt_path, &mut doc_opt) {
                        let _ = doc.delete_group(&id);
                        for chat_id in &chat_ids {
                            let _ = doc.delete_chat(chat_id);
                        }
                        dirty = true;
                    }
                }
                CrdtJob::SetPersona(p) => {
                    if let Some(doc) = ensure_crdt_doc(&base_dir, &crdt_path, &mut doc_opt)
                        && doc.set_persona(&p).is_ok()
                    {
                        dirty = true;
                    }
                }
                CrdtJob::DeletePersona(id) => {
                    if let Some(doc) = ensure_crdt_doc(&base_dir, &crdt_path, &mut doc_opt)
                        && doc.delete_persona(&id).is_ok()
                    {
                        dirty = true;
                    }
                }
                CrdtJob::SetLorebook(b) => {
                    if let Some(doc) = ensure_crdt_doc(&base_dir, &crdt_path, &mut doc_opt)
                        && doc.set_lorebook(&b).is_ok()
                    {
                        dirty = true;
                    }
                }
                CrdtJob::DeleteLorebook(id) => {
                    if let Some(doc) = ensure_crdt_doc(&base_dir, &crdt_path, &mut doc_opt)
                        && doc.delete_lorebook(&id).is_ok()
                    {
                        dirty = true;
                    }
                }
                CrdtJob::ReplaceSnapshot(bytes) => {
                    if let Ok(new_doc) = TavernCrdtDoc::from_snapshot(&bytes) {
                        doc_opt = Some(new_doc);
                        dirty = false;
                    }
                }
                CrdtJob::Run(f) => {
                    if let Some(doc) = ensure_crdt_doc(&base_dir, &crdt_path, &mut doc_opt) {
                        f(doc);
                    } else {
                        let fallback = TavernCrdtDoc::new();
                        f(&fallback);
                    }
                }
                CrdtJob::RunMut(f) => {
                    if let Some(doc) = ensure_crdt_doc(&base_dir, &crdt_path, &mut doc_opt) {
                        if f(doc) || dirty {
                            let _ = save_crdt_snapshot_atomic(&crdt_path, doc);
                            dirty = false;
                        }
                    } else {
                        let fallback = TavernCrdtDoc::new();
                        if f(&fallback) {
                            let _ = save_crdt_snapshot_atomic(&crdt_path, &fallback);
                        }
                        doc_opt = Some(fallback);
                        dirty = false;
                    }
                }
                CrdtJob::Flush(tx) => {
                    flush_waiters.push(tx);
                }
                CrdtJob::Shutdown => {
                    should_shutdown = true;
                }
            };

            process(first_job);
            while let Ok(next_job) = rx.try_recv() {
                process(next_job);
            }

            if should_shutdown || !flush_waiters.is_empty() {
                if dirty
                    && let Some(doc) = doc_opt.as_ref()
                {
                    let _ = save_crdt_snapshot_atomic(&crdt_path, doc);
                    dirty = false;
                }
                for tx in flush_waiters {
                    let _ = tx.send(());
                }
                if should_shutdown {
                    break;
                }
            }
        }
    }

    pub fn populate_crdt_from_disk(&self, doc: &TavernCrdtDoc) -> Result<(), String> {
        populate_crdt_from_disk_dir(&self.base_dir, doc);
        Ok(())
    }

    pub fn sync_disk_from_crdt(&self, doc: &TavernCrdtDoc) -> Result<SyncStats, String> {
        let mut stats = SyncStats::default();
        let tombstones = doc.get_tombstones().unwrap_or_default();

        // 1. Characters
        if let Ok(characters) = doc.get_characters() {
            for c in characters {
                let tomb_key = format!("character:{}", c.id);
                if let Some(tomb) = tombstones.get(&tomb_key)
                    && tomb.deleted_at >= c.updated_at
                {
                    let path = self.characters_dir().join(format!("{}.json", c.id));
                    if path.exists() {
                        let _ = fs::remove_file(path);
                    }
                    continue;
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
                if let Some(tomb) = tombstones.get(&tomb_key)
                    && tomb.deleted_at >= chat.updated_at
                {
                    let path = self.chats_dir().join(format!("{}.json", chat.id));
                    if path.exists() {
                        let _ = fs::remove_file(path);
                    }
                    continue;
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
                if let Some(tomb) = tombstones.get(&tomb_key)
                    && tomb.deleted_at >= b.updated_at
                {
                    let path = self.lorebooks_dir().join(format!("{}.json", b.id));
                    if path.exists() {
                        let _ = fs::remove_file(path);
                    }
                    continue;
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
                if let Some(tomb) = tombstones.get(&tomb_key)
                    && tomb.deleted_at >= g.updated_at
                {
                    let path = self.groups_dir().join(format!("{}.json", g.id));
                    if path.exists() {
                        let _ = fs::remove_file(path);
                    }
                    continue;
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

        *self.chats_index.lock() = None;
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

fn chat_to_summary(chat: &ChatTree) -> ChatSummary {
    let active_path = chat.get_active_path();
    let count = active_path.len();
    let preview = active_path
        .last()
        .map(|m| {
            let c = m.content.chars().take(80).collect::<String>();
            if m.content.chars().nth(80).is_some() {
                format!("{c}...")
            } else {
                c
            }
        })
        .unwrap_or_else(|| "Empty chat".to_string());

    ChatSummary {
        id: chat.id.to_string(),
        character_id: chat.character_id.clone(),
        title: chat.title.clone(),
        created_at: chat.created_at,
        updated_at: chat.updated_at,
        message_count: count,
        last_message_preview: preview,
        group_id: chat.group_id.clone(),
    }
}

fn load_json_dir_parallel<T>(dir: &Path) -> Vec<T>
where
    T: for<'de> Deserialize<'de> + Send,
{
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();

    if paths.is_empty() {
        return Vec::new();
    }
    if paths.len() == 1 {
        return fs::read(&paths[0])
            .ok()
            .and_then(|b| serde_json::from_slice::<T>(&b).ok())
            .into_iter()
            .collect();
    }

    let workers = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(paths.len())
        .max(1);
    let chunk_size = paths.len().div_ceil(workers);

    let mut out = Vec::with_capacity(paths.len());
    thread::scope(|s| {
        let mut handles = Vec::with_capacity(workers);
        for chunk in paths.chunks(chunk_size) {
            handles.push(s.spawn(move || {
                let mut local = Vec::with_capacity(chunk.len());
                for path in chunk {
                    if let Ok(bytes) = fs::read(path)
                        && let Ok(item) = serde_json::from_slice::<T>(&bytes)
                    {
                        local.push(item);
                    }
                }
                local
            }));
        }
        for h in handles {
            if let Ok(mut local) = h.join() {
                out.append(&mut local);
            }
        }
    });
    out
}

fn ensure_crdt_doc<'a>(
    base_dir: &Path,
    crdt_path: &Path,
    doc_opt: &'a mut Option<TavernCrdtDoc>,
) -> Option<&'a TavernCrdtDoc> {
    if doc_opt.is_none() {
        let doc = if crdt_path.exists() {
            match fs::read(crdt_path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| TavernCrdtDoc::from_snapshot(&bytes))
            {
                Ok(d) => d,
                Err(_) => {
                    let d = TavernCrdtDoc::new();
                    populate_crdt_from_disk_dir(base_dir, &d);
                    let _ = save_crdt_snapshot_atomic(crdt_path, &d);
                    d
                }
            }
        } else {
            let d = TavernCrdtDoc::new();
            populate_crdt_from_disk_dir(base_dir, &d);
            let _ = save_crdt_snapshot_atomic(crdt_path, &d);
            d
        };
        *doc_opt = Some(doc);
    }
    doc_opt.as_ref()
}

fn save_crdt_bytes_atomic(crdt_path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp_path = crdt_path.with_extension("bin.tmp");
    if fs::write(&tmp_path, bytes).is_ok() && fs::rename(&tmp_path, crdt_path).is_ok() {
        return Ok(());
    }
    let _ = fs::remove_file(&tmp_path);
    fs::write(crdt_path, bytes).map_err(|e| e.to_string())
}

fn save_crdt_snapshot_atomic(crdt_path: &Path, doc: &TavernCrdtDoc) -> Result<(), String> {
    let bytes = doc.export_snapshot()?;
    save_crdt_bytes_atomic(crdt_path, &bytes)
}

fn populate_crdt_from_disk_dir(base_dir: &Path, doc: &TavernCrdtDoc) {
    for c in load_json_dir_parallel::<Character>(&base_dir.join("characters")) {
        let _ = doc.set_character(&c);
    }
    for chat in load_json_dir_parallel::<ChatTree>(&base_dir.join("chats")) {
        let _ = doc.set_chat(&chat);
    }
    for p in load_json_dir_parallel::<UserPersona>(&base_dir.join("personas")) {
        let _ = doc.set_persona(&p);
    }
    for b in load_json_dir_parallel::<Lorebook>(&base_dir.join("lorebooks")) {
        let _ = doc.set_lorebook(&b);
    }
    for g in load_json_dir_parallel::<Group>(&base_dir.join("groups")) {
        let _ = doc.set_group(&g);
    }
    doc.commit();
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::chat::AuthorRole;

    fn temp_storage_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tavern_test_{}_{}", name, Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_character_and_chat_index_delete_and_crdt_consistency() {
        let dir = temp_storage_dir("char_del");
        {
            let storage = StorageManager::new(dir.clone());

            let c1 = Character::new("Alice".into(), "Hello!".into(), None);
            let c2 = Character::new("Bob".into(), "Hi there!".into(), None);
            storage.save_character(&c1).unwrap();
            storage.save_character(&c2).unwrap();

            let mut chat1 = ChatTree::new(c1.id.clone(), "Alice Chat 1".into());
            chat1.append_message(AuthorRole::User, "Hey Alice".into(), None);
            let mut chat2 = ChatTree::new(c1.id.clone(), "Alice Chat 2".into());
            chat2.append_message(AuthorRole::User, "Another chat".into(), None);
            let mut chat_bob = ChatTree::new(c2.id.clone(), "Bob Chat".into());
            chat_bob.append_message(AuthorRole::User, "Hey Bob".into(), None);

            storage.save_chat(&chat1).unwrap();
            storage.save_chat(&chat2).unwrap();
            storage.save_chat(&chat_bob).unwrap();

            assert_eq!(storage.list_chats_for_character(&c1.id).unwrap().len(), 2);
            assert_eq!(storage.list_chats_for_character(&c2.id).unwrap().len(), 1);

            // Delete Alice -> should delete Alice and both of Alice's chats, leaving Bob untouched
            storage.delete_character(&c1.id).unwrap();

            assert!(storage.load_character(&c1.id).is_err());
            assert!(storage.load_character(&c2.id).is_ok());
            assert_eq!(storage.list_chats_for_character(&c1.id).unwrap().len(), 0);
            assert_eq!(storage.list_chats_for_character(&c2.id).unwrap().len(), 1);
            assert!(storage.load_chat(&chat1.id.to_string()).is_err());
            assert!(storage.load_chat(&chat2.id.to_string()).is_err());
            assert!(storage.load_chat(&chat_bob.id.to_string()).is_ok());

            storage.flush_crdt();
            let crdt = storage.load_crdt_doc().unwrap();
            let tombstones = crdt.get_tombstones().unwrap();
            assert!(tombstones.contains_key(&format!("character:{}", c1.id)));
            assert!(tombstones.contains_key(&format!("chat:{}", chat1.id)));
            assert!(tombstones.contains_key(&format!("chat:{}", chat2.id)));
            assert!(!tombstones.contains_key(&format!("character:{}", c2.id)));

            let crdt_chars = crdt.get_characters().unwrap();
            assert!(crdt_chars.iter().any(|c| c.id == c2.id));
            assert!(!crdt_chars.iter().any(|c| c.id == c1.id));
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
