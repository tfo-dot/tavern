use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use engine::character::{Character, CharacterCardV2, UserPersona};
use engine::chat::ChatTree;
use serde::{Deserialize, Serialize};

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
    }

    fn ensure_starter_data(&self) {
        if let Ok(entries) = fs::read_dir(self.characters_dir()) {
            let count = entries.count();
            if count == 0 {
                let mut card = CharacterCardV2::default();
                card.data.name = "Seraphina".to_string();
                card.data.description = "A warm, enigmatic tavern keeper with silver hair and amethyst eyes. She runs the Starlight Tavern, a mystical refuge between worlds where travelers rest and share their stories.".to_string();
                card.data.personality = "Empathetic, witty, observant, gentle yet possessing ancient arcane knowledge.".to_string();
                card.data.scenario = "{{user}} pushes open the heavy oak doors of the Starlight Tavern on a stormy night, seeking shelter from the bitter cold.".to_string();
                card.data.first_mes = "*Rain lashes against the stained glass windows as the heavy oak door creaks open. Behind the polished mahogany counter, Seraphina looks up from polishing a crystal glass, her amethyst eyes sparkling warmly.*\n\n\"Welcome, traveler! Step inside and warm yourself by the hearth. Come, take a seat. What tale or drink brings you to my tavern tonight?\"".to_string();
                card.data.alternate_greetings = vec![
                    "*The tavern is quiet tonight, soft lute music playing from an unseen corner. Seraphina smiles gently as you step in.*\n\n\"Back again, traveler? I just brewed fresh spiced cider. Pull up a chair and let's catch up.\"".to_string()
                ];
                card.data.tags = vec!["Fantasy".to_string(), "Tavern".to_string(), "Mystical".to_string(), "Friendly".to_string()];
                
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
        fs::write(path, data).map_err(|e| e.to_string())
    }

    pub fn load_character(&self, id: &str) -> Result<Character, String> {
        let path = self.characters_dir().join(format!("{id}.json"));
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| e.to_string())?;
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
        characters.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(characters)
    }

    pub fn delete_character(&self, id: &str) -> Result<(), String> {
        let path = self.characters_dir().join(format!("{id}.json"));
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
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
        fs::write(path, data).map_err(|e| e.to_string())
    }

    pub fn load_chat(&self, id: &str) -> Result<ChatTree, String> {
        let path = self.chats_dir().join(format!("{id}.json"));
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| e.to_string())?;
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
                                });
                            }
                        }
                    }
                }
            }
        }
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(summaries)
    }

    pub fn delete_chat(&self, id: &str) -> Result<(), String> {
        let path = self.chats_dir().join(format!("{id}.json"));
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        Ok(())
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
        fs::write(path, data).map_err(|e| e.to_string())
    }

    pub fn load_user_persona(&self, id: &str) -> Result<UserPersona, String> {
        let path = self.personas_dir().join(format!("{id}.json"));
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| e.to_string())?;
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
        Ok(())
    }
}
