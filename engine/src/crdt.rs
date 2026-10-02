use chrono::{DateTime, Utc};
use loro::{ExportMode, LoroDoc, LoroValue, VersionVector};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use uuid::Uuid;

use crate::character::{Character, UserPersona};
use crate::chat::{ChatTree, Group};
use crate::lorebook::Lorebook;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeletionTombstone {
    pub id: String,
    pub entity_type: String,
    pub deleted_at: DateTime<Utc>,
}

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
        self.doc
            .export(ExportMode::Snapshot)
            .map_err(|e| e.to_string())
    }

    pub fn state_vector(&self) -> VersionVector {
        self.doc.oplog_vv()
    }

    pub fn export_updates_from(&self, vv: &VersionVector) -> Result<Vec<u8>, String> {
        self.doc
            .export(ExportMode::Updates {
                from: std::borrow::Cow::Borrowed(vv),
            })
            .map_err(|e| e.to_string())
    }

    pub fn import_updates(&self, updates: &[u8]) -> Result<(), String> {
        if updates.is_empty() {
            return Ok(());
        }
        self.doc.import(updates).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[inline]
    pub fn commit(&self) {
        self.doc.commit();
    }

    // --- Generic Internal CRDT Helpers ---

    fn put_entity<T: Serialize>(
        &self,
        map_name: &str,
        entity_type: &str,
        id: &str,
        entity: &T,
    ) -> Result<(), String> {
        let map = self.doc.get_map(map_name);
        let json_str = serde_json::to_string(entity).map_err(|e| e.to_string())?;
        map.insert(id, json_str).map_err(|e| e.to_string())?;

        // Clear any prior tombstone
        let tombstones_map = self.doc.get_map("tombstones");
        let tomb_key = format!("{entity_type}:{id}");
        let _ = tombstones_map.delete(&tomb_key);

        self.commit();
        Ok(())
    }

    fn delete_entity(&self, map_name: &str, entity_type: &str, id: &str) -> Result<(), String> {
        let map = self.doc.get_map(map_name);
        let _ = map.delete(id);

        let tombstones_map = self.doc.get_map("tombstones");
        let tombstone = DeletionTombstone {
            id: id.to_string(),
            entity_type: entity_type.to_string(),
            deleted_at: Utc::now(),
        };
        let tomb_json = serde_json::to_string(&tombstone).map_err(|e| e.to_string())?;
        tombstones_map
            .insert(&format!("{entity_type}:{id}"), tomb_json)
            .map_err(|e| e.to_string())?;

        self.commit();
        Ok(())
    }

    fn list_entities<T: DeserializeOwned>(&self, map_name: &str) -> Vec<T> {
        let map = self.doc.get_map(map_name);
        let mut list = Vec::new();
        if let LoroValue::Map(inner_map) = map.get_value() {
            for val in inner_map.values() {
                if let LoroValue::String(s) = val
                    && let Ok(item) = serde_json::from_str::<T>(s)
                {
                    list.push(item);
                }
            }
        }
        list
    }

    // --- Public Entity APIs ---

    pub fn set_character(&self, character: &Character) -> Result<(), String> {
        self.put_entity("characters", "character", &character.id, character)
    }

    pub fn delete_character(&self, id: &str) -> Result<(), String> {
        self.delete_entity("characters", "character", id)
    }

    pub fn get_characters(&self) -> Result<Vec<Character>, String> {
        let mut list: Vec<Character> = self.list_entities("characters");
        list.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(list)
    }

    pub fn set_chat(&self, chat: &ChatTree) -> Result<(), String> {
        self.put_entity("chats", "chat", &chat.id.to_string(), chat)
    }

    pub fn delete_chat(&self, id: &Uuid) -> Result<(), String> {
        self.delete_entity("chats", "chat", &id.to_string())
    }

    pub fn get_chats(&self) -> Result<Vec<ChatTree>, String> {
        let mut list: Vec<ChatTree> = self.list_entities("chats");
        list.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(list)
    }

    pub fn set_persona(&self, persona: &UserPersona) -> Result<(), String> {
        self.put_entity("personas", "persona", &persona.id, persona)
    }

    pub fn delete_persona(&self, id: &str) -> Result<(), String> {
        self.delete_entity("personas", "persona", id)
    }

    pub fn get_personas(&self) -> Result<Vec<UserPersona>, String> {
        Ok(self.list_entities("personas"))
    }

    pub fn set_lorebook(&self, lorebook: &Lorebook) -> Result<(), String> {
        self.put_entity("lorebooks", "lorebook", &lorebook.id, lorebook)
    }

    pub fn delete_lorebook(&self, id: &str) -> Result<(), String> {
        self.delete_entity("lorebooks", "lorebook", id)
    }

    pub fn get_lorebooks(&self) -> Result<Vec<Lorebook>, String> {
        let mut list: Vec<Lorebook> = self.list_entities("lorebooks");
        list.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(list)
    }

    pub fn set_group(&self, group: &Group) -> Result<(), String> {
        self.put_entity("groups", "group", &group.id, group)
    }

    pub fn delete_group(&self, id: &str) -> Result<(), String> {
        self.delete_entity("groups", "group", id)
    }

    pub fn get_groups(&self) -> Result<Vec<Group>, String> {
        let mut list: Vec<Group> = self.list_entities("groups");
        list.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(list)
    }

    pub fn get_tombstones(&self) -> Result<HashMap<String, DeletionTombstone>, String> {
        let map = self.doc.get_map("tombstones");
        let mut map_out = HashMap::new();
        if let LoroValue::Map(inner) = map.get_value() {
            for (k, val) in inner.iter() {
                if let LoroValue::String(s) = val
                    && let Ok(tomb) = serde_json::from_str::<DeletionTombstone>(s)
                {
                    map_out.insert(k.to_string(), tomb);
                }
            }
        }
        Ok(map_out)
    }
}
