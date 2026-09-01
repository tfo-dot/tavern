use loro::{ExportMode, LoroDoc, LoroValue, VersionVector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::character::{Character, UserPersona};
use crate::chat::ChatTree;
use crate::lorebook::Lorebook;

/// Tombstone record for tracking deleted entities across distributed peers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeletionTombstone {
    pub id: String,
    pub entity_type: String, // "character" | "chat" | "lorebook" | "persona"
    pub deleted_at: DateTime<Utc>,
}

/// The core CRDT document wrapper managing Tavern data.
pub struct TavernCrdtDoc {
    pub doc: LoroDoc,
}

impl Default for TavernCrdtDoc {
    fn default() -> Self {
        Self::new()
    }
}

impl TavernCrdtDoc {
    pub fn new() -> Self {
        Self {
            doc: LoroDoc::new(),
        }
    }

    pub fn from_snapshot(bytes: &[u8]) -> Result<Self, String> {
        let doc = LoroDoc::new();
        if !bytes.is_empty() {
            doc.import(bytes).map_err(|e| e.to_string())?;
        }
        Ok(Self { doc })
    }

    pub fn export_snapshot(&self) -> Result<Vec<u8>, String> {
        self.doc.export(ExportMode::Snapshot).map_err(|e| e.to_string())
    }

    pub fn state_vector(&self) -> VersionVector {
        self.doc.oplog_vv()
    }

    pub fn export_updates_from(&self, vv: &VersionVector) -> Result<Vec<u8>, String> {
        self.doc.export(ExportMode::Updates { from: std::borrow::Cow::Borrowed(vv) }).map_err(|e| e.to_string())
    }

    pub fn import_updates(&self, updates: &[u8]) -> Result<(), String> {
        if updates.is_empty() {
            return Ok(());
        }
        self.doc.import(updates).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn commit(&self) {
        self.doc.commit();
    }

    // --- Characters ---

    pub fn set_character(&self, character: &Character) -> Result<(), String> {
        let chars_map = self.doc.get_map("characters");
        let json_str = serde_json::to_string(character).map_err(|e| e.to_string())?;
        chars_map.insert(&character.id, json_str).map_err(|e| e.to_string())?;
        
        // Remove from tombstones if it was previously marked deleted
        let tombstones_map = self.doc.get_map("tombstones");
        let tomb_key = format!("character:{}", character.id);
        let _ = tombstones_map.delete(&tomb_key);

        self.commit();
        Ok(())
    }

    pub fn delete_character(&self, id: &str) -> Result<(), String> {
        let chars_map = self.doc.get_map("characters");
        let _ = chars_map.delete(id);

        let tombstones_map = self.doc.get_map("tombstones");
        let tombstone = DeletionTombstone {
            id: id.to_string(),
            entity_type: "character".to_string(),
            deleted_at: Utc::now(),
        };
        let tomb_json = serde_json::to_string(&tombstone).map_err(|e| e.to_string())?;
        tombstones_map.insert(&format!("character:{id}"), tomb_json).map_err(|e| e.to_string())?;

        self.commit();
        Ok(())
    }

    pub fn get_characters(&self) -> Result<Vec<Character>, String> {
        let chars_map = self.doc.get_map("characters");
        let mut list = Vec::new();
        let value = chars_map.get_value();
        if let LoroValue::Map(map) = value {
            for (_, val) in map.iter() {
                if let LoroValue::String(s) = val {
                    if let Ok(char_obj) = serde_json::from_str::<Character>(s) {
                        list.push(char_obj);
                    }
                }
            }
        }
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(list)
    }

    // --- Chats ---

    pub fn set_chat(&self, chat: &ChatTree) -> Result<(), String> {
        let chats_map = self.doc.get_map("chats");
        let json_str = serde_json::to_string(chat).map_err(|e| e.to_string())?;
        chats_map.insert(&chat.id.to_string(), json_str).map_err(|e| e.to_string())?;

        let tombstones_map = self.doc.get_map("tombstones");
        let tomb_key = format!("chat:{}", chat.id);
        let _ = tombstones_map.delete(&tomb_key);

        self.commit();
        Ok(())
    }

    pub fn delete_chat(&self, id: &Uuid) -> Result<(), String> {
        let chats_map = self.doc.get_map("chats");
        let id_str = id.to_string();
        let _ = chats_map.delete(&id_str);

        let tombstones_map = self.doc.get_map("tombstones");
        let tombstone = DeletionTombstone {
            id: id_str.clone(),
            entity_type: "chat".to_string(),
            deleted_at: Utc::now(),
        };
        let tomb_json = serde_json::to_string(&tombstone).map_err(|e| e.to_string())?;
        tombstones_map.insert(&format!("chat:{id_str}"), tomb_json).map_err(|e| e.to_string())?;

        self.commit();
        Ok(())
    }

    pub fn get_chats(&self) -> Result<Vec<ChatTree>, String> {
        let chats_map = self.doc.get_map("chats");
        let mut list = Vec::new();
        let value = chats_map.get_value();
        if let LoroValue::Map(map) = value {
            for (_, val) in map.iter() {
                if let LoroValue::String(s) = val {
                    if let Ok(chat) = serde_json::from_str::<ChatTree>(s) {
                        list.push(chat);
                    }
                }
            }
        }
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(list)
    }

    // --- Personas ---

    pub fn set_persona(&self, persona: &UserPersona) -> Result<(), String> {
        let personas_map = self.doc.get_map("personas");
        let json_str = serde_json::to_string(persona).map_err(|e| e.to_string())?;
        personas_map.insert(&persona.id, json_str).map_err(|e| e.to_string())?;

        let tombstones_map = self.doc.get_map("tombstones");
        let tomb_key = format!("persona:{}", persona.id);
        let _ = tombstones_map.delete(&tomb_key);

        self.commit();
        Ok(())
    }

    pub fn delete_persona(&self, id: &str) -> Result<(), String> {
        let personas_map = self.doc.get_map("personas");
        let _ = personas_map.delete(id);

        let tombstones_map = self.doc.get_map("tombstones");
        let tombstone = DeletionTombstone {
            id: id.to_string(),
            entity_type: "persona".to_string(),
            deleted_at: Utc::now(),
        };
        let tomb_json = serde_json::to_string(&tombstone).map_err(|e| e.to_string())?;
        tombstones_map.insert(&format!("persona:{id}"), tomb_json).map_err(|e| e.to_string())?;

        self.commit();
        Ok(())
    }

    pub fn get_personas(&self) -> Result<Vec<UserPersona>, String> {
        let personas_map = self.doc.get_map("personas");
        let mut list = Vec::new();
        let value = personas_map.get_value();
        if let LoroValue::Map(map) = value {
            for (_, val) in map.iter() {
                if let LoroValue::String(s) = val {
                    if let Ok(persona) = serde_json::from_str::<UserPersona>(s) {
                        list.push(persona);
                    }
                }
            }
        }
        Ok(list)
    }

    // --- Lorebooks ---

    pub fn set_lorebook(&self, lorebook: &Lorebook) -> Result<(), String> {
        let lorebooks_map = self.doc.get_map("lorebooks");
        let json_str = serde_json::to_string(lorebook).map_err(|e| e.to_string())?;
        lorebooks_map.insert(&lorebook.id, json_str).map_err(|e| e.to_string())?;

        let tombstones_map = self.doc.get_map("tombstones");
        let tomb_key = format!("lorebook:{}", lorebook.id);
        let _ = tombstones_map.delete(&tomb_key);

        self.commit();
        Ok(())
    }

    pub fn delete_lorebook(&self, id: &str) -> Result<(), String> {
        let lorebooks_map = self.doc.get_map("lorebooks");
        let _ = lorebooks_map.delete(id);

        let tombstones_map = self.doc.get_map("tombstones");
        let tombstone = DeletionTombstone {
            id: id.to_string(),
            entity_type: "lorebook".to_string(),
            deleted_at: Utc::now(),
        };
        let tomb_json = serde_json::to_string(&tombstone).map_err(|e| e.to_string())?;
        tombstones_map.insert(&format!("lorebook:{id}"), tomb_json).map_err(|e| e.to_string())?;

        self.commit();
        Ok(())
    }

    pub fn get_lorebooks(&self) -> Result<Vec<Lorebook>, String> {
        let lorebooks_map = self.doc.get_map("lorebooks");
        let mut list = Vec::new();
        let value = lorebooks_map.get_value();
        if let LoroValue::Map(map) = value {
            for (_, val) in map.iter() {
                if let LoroValue::String(s) = val {
                    if let Ok(book) = serde_json::from_str::<Lorebook>(s) {
                        list.push(book);
                    }
                }
            }
        }
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(list)
    }

    // --- Tombstones ---

    pub fn get_tombstones(&self) -> Result<HashMap<String, DeletionTombstone>, String> {
        let tombstones_map = self.doc.get_map("tombstones");
        let mut map_out = HashMap::new();
        let value = tombstones_map.get_value();
        if let LoroValue::Map(map) = value {
            for (k, val) in map.iter() {
                if let LoroValue::String(s) = val {
                    if let Ok(tomb) = serde_json::from_str::<DeletionTombstone>(s) {
                        map_out.insert(k.to_string(), tomb);
                    }
                }
            }
        }
        Ok(map_out)
    }
}
