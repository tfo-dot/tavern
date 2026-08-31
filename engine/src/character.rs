use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::lorebook::CharacterBook;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterCardV2 {
    pub spec: String,         // "chara_card_v2"
    pub spec_version: String, // "2.0"
    pub data: CharacterData,
}

impl Default for CharacterCardV2 {
    fn default() -> Self {
        Self {
            spec: "chara_card_v2".to_string(),
            spec_version: "2.0".to_string(),
            data: CharacterData::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterData {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub scenario: String,
    #[serde(default)]
    pub first_mes: String,
    #[serde(default)]
    pub mes_example: String,

    #[serde(default)]
    pub creator_notes: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub post_history_instructions: String,
    #[serde(default)]
    pub alternate_greetings: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub creator: String,
    #[serde(default)]
    pub character_version: String,
    #[serde(default)]
    pub character_book: Option<CharacterBook>,
    #[serde(default)]
    pub lorebook_ids: Vec<String>,
    #[serde(default)]
    pub extensions: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterCardV1 {
    pub name: String,
    pub description: String,
    pub personality: String,
    pub scenario: String,
    pub first_mes: String,
    pub mes_example: String,
}

impl From<CharacterCardV1> for CharacterCardV2 {
    fn from(v1: CharacterCardV1) -> Self {
        Self {
            spec: "chara_card_v2".to_string(),
            spec_version: "2.0".to_string(),
            data: CharacterData {
                name: v1.name,
                description: v1.description,
                personality: v1.personality,
                scenario: v1.scenario,
                first_mes: v1.first_mes,
                mes_example: v1.mes_example,
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub card: CharacterCardV2,
    pub avatar_data_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Character {
    pub fn new(name: String, first_mes: String, avatar_data_url: Option<String>) -> Self {
        let mut card = CharacterCardV2::default();
        card.data.name = name;
        card.data.first_mes = first_mes;
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            card,
            avatar_data_url,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn from_card(card: CharacterCardV2, avatar_data_url: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            card,
            avatar_data_url,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPersona {
    pub id: String,
    pub name: String,
    pub description: String,
    pub avatar_data_url: Option<String>,
}

impl Default for UserPersona {
    fn default() -> Self {
        Self {
            id: "default_user".to_string(),
            name: "User".to_string(),
            description: "".to_string(),
            avatar_data_url: None,
        }
    }
}
