use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{watch, Mutex};
use uuid::Uuid;

use engine::character::{Character, UserPersona};
use engine::chat::{resolve_next_speaker, AuthorRole, ChatTree, Group, MessageViewNode};
use engine::llm::{fetch_models, stream_chat_completion, GenerationParams};
use engine::lorebook::Lorebook;
use engine::parser::{export_character_json, export_character_png, parse_character_card};
use engine::prompt::{
    build_chat_prompt_with_lorebooks, build_group_chat_prompt_with_lorebooks, PromptConfig,
};
use tauri::{Emitter, Manager, State, Window};

pub mod storage;
pub mod sync;
use storage::{AppSettings, ChatSummary, StorageManager};
use sync::{DiscoveredPeer, SyncDeviceInfo, SyncManager, SyncStats};

pub struct AppState {
    pub storage: Arc<StorageManager>,
    pub active_chat: Mutex<Option<ChatTree>>,
    pub active_character: Mutex<Option<Character>>,
    pub active_group: Mutex<Option<Group>>,
    pub settings: Mutex<AppSettings>,
    pub user_persona: Mutex<UserPersona>,
    pub abort_tx: Mutex<Option<watch::Sender<bool>>>,
    pub sync_manager: parking_lot::RwLock<Option<Arc<SyncManager>>>,
}
// --- Character Commands ---

#[tauri::command]
async fn get_all_characters(state: State<'_, Arc<AppState>>) -> Result<Vec<Character>, String> {
    state.storage.list_characters()
}

#[tauri::command]
async fn get_character(id: String, state: State<'_, Arc<AppState>>) -> Result<Character, String> {
    state.storage.load_character(&id)
}

#[tauri::command]
async fn save_character(
    character: Character,
    state: State<'_, Arc<AppState>>,
) -> Result<Character, String> {
    state.storage.save_character(&character)?;
    let mut active = state.active_character.lock().await;
    if let Some(cur) = active.as_ref() {
        if cur.id == character.id {
            *active = Some(character.clone());
        }
    }
    Ok(character)
}

#[tauri::command]
async fn delete_character(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.storage.delete_character(&id)?;
    let mut active_char = state.active_character.lock().await;
    if let Some(cur) = active_char.as_ref() {
        if cur.id == id {
            *active_char = None;
        }
    }
    let mut active_chat = state.active_chat.lock().await;
    if let Some(chat) = active_chat.as_ref() {
        if chat.character_id == id {
            *active_chat = None;
        }
    }
    Ok(())
}

#[tauri::command]
async fn import_character_card(
    file_bytes: Vec<u8>,
    state: State<'_, Arc<AppState>>,
) -> Result<Character, String> {
    let (card, avatar) = parse_character_card(&file_bytes).map_err(|e| e.to_string())?;
    let character = Character::from_card(card, avatar);
    state.storage.save_character(&character)?;
    Ok(character)
}

#[tauri::command]
async fn export_card_png(
    character_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<u8>, String> {
    let character = state.storage.load_character(&character_id)?;

    // Check if avatar is stored as base64 data url
    let avatar_bytes = if let Some(data_url) = &character.avatar_data_url {
        if let Some(b64_part) = data_url.split(',').nth(1) {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(b64_part.as_bytes())
                .ok()
        } else {
            None
        }
    } else {
        None
    };

    export_character_png(&character.card, avatar_bytes.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_card_json(
    character_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let character = state.storage.load_character(&character_id)?;
    export_character_json(&character.card).map_err(|e| e.to_string())
}

// --- Group Commands ---

#[tauri::command]
async fn get_all_groups(state: State<'_, Arc<AppState>>) -> Result<Vec<Group>, String> {
    state.storage.list_groups()
}

#[tauri::command]
async fn get_group(id: String, state: State<'_, Arc<AppState>>) -> Result<Group, String> {
    state.storage.load_group(&id)
}

#[tauri::command]
async fn save_group(group: Group, state: State<'_, Arc<AppState>>) -> Result<Group, String> {
    state.storage.save_group(&group)?;
    let mut active = state.active_group.lock().await;
    if let Some(cur) = active.as_ref() {
        if cur.id == group.id {
            *active = Some(group.clone());
        }
    }
    Ok(group)
}

#[tauri::command]
async fn delete_group(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.storage.delete_group(&id)?;
    let mut active = state.active_group.lock().await;
    if let Some(cur) = active.as_ref() {
        if cur.id == id {
            *active = None;
        }
    }
    let mut active_chat = state.active_chat.lock().await;
    if let Some(chat) = active_chat.as_ref() {
        if chat.group_id.as_deref() == Some(&id) {
            *active_chat = None;
        }
    }
    Ok(())
}

#[tauri::command]
async fn create_group_chat(
    group_id: String,
    title: Option<String>,
    first_mes: Option<String>,
    first_speaker_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<ChatTree, String> {
    let group = state.storage.load_group(&group_id)?;
    let chat_title = title.unwrap_or_else(|| format!("Group: {}", group.name));
    let mut tree = ChatTree::new_group(group_id.clone(), chat_title);

    if let Some(first_text) = first_mes {
        if !first_text.trim().is_empty() {
            let speaker_name = if let Some(sid) = &first_speaker_id {
                state
                    .storage
                    .load_character(sid)
                    .ok()
                    .map(|c| c.card.data.name)
            } else {
                None
            };
            tree.append_message_with_author(
                AuthorRole::Assistant,
                first_text,
                None,
                first_speaker_id,
                speaker_name,
            );
        }
    } else if let Some(sid) = first_speaker_id {
        if let Ok(ch) = state.storage.load_character(&sid) {
            if !ch.card.data.first_mes.trim().is_empty() {
                tree.append_message_with_author(
                    AuthorRole::Assistant,
                    ch.card.data.first_mes.clone(),
                    None,
                    Some(sid.clone()),
                    Some(ch.card.data.name.clone()),
                );
            }
        }
    }

    state.storage.save_chat(&tree)?;

    {
        let mut active_chat = state.active_chat.lock().await;
        *active_chat = Some(tree.clone());
    }
    {
        let mut active_grp = state.active_group.lock().await;
        *active_grp = Some(group);
    }
    {
        let mut settings = state.settings.lock().await;
        settings.active_character_id = Some(tree.character_id.clone());
        settings.active_chat_id = Some(tree.id.to_string());
        let _ = state.storage.save_settings(&settings);
    }

    Ok(tree)
}

#[tauri::command]
async fn list_group_chats(
    group_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ChatSummary>, String> {
    state.storage.list_chats_for_group(&group_id)
}

#[tauri::command]
async fn set_message_author(
    id: Uuid,
    character_id: Option<String>,
    name: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<MessageViewNode>, String> {
    let mut lock = state.active_chat.lock().await;
    let tree = lock.as_mut().ok_or("No active chat session")?;
    tree.set_message_author(id, character_id, name)?;
    state.storage.save_chat(tree)?;
    Ok(tree.get_active_view_nodes())
}

// --- Chat Commands ---

#[tauri::command]
async fn create_chat(
    character_id: String,
    first_mes: Option<String>,
    title: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<ChatTree, String> {
    let character = state.storage.load_character(&character_id)?;
    let chat_title = title.unwrap_or_else(|| format!("Chat with {}", character.card.data.name));
    let mut tree = ChatTree::new(character_id.clone(), chat_title);

    let primary_greeting = first_mes.unwrap_or_else(|| character.card.data.first_mes.clone());
    if !primary_greeting.is_empty() {
        tree.append_message(AuthorRole::Assistant, primary_greeting, None);
    }

    // Add all alternate greetings as sibling root swipes
    for alt in &character.card.data.alternate_greetings {
        if !alt.trim().is_empty() {
            tree.add_swipe(None, alt.clone());
        }
    }
    tree.active_root_index = 0;
    state.storage.save_chat(&tree)?;

    {
        let mut active_chat = state.active_chat.lock().await;
        *active_chat = Some(tree.clone());
    }
    {
        let mut active_char = state.active_character.lock().await;
        *active_char = Some(character);
    }
    {
        let mut settings = state.settings.lock().await;
        settings.active_character_id = Some(character_id);
        settings.active_chat_id = Some(tree.id.to_string());
        let _ = state.storage.save_settings(&settings);
    }

    Ok(tree)
}

#[tauri::command]
async fn load_chat(chat_id: String, state: State<'_, Arc<AppState>>) -> Result<ChatTree, String> {
    let mut tree = state.storage.load_chat(&chat_id)?;

    if let Some(gid) = &tree.group_id {
        if let Ok(group) = state.storage.load_group(gid) {
            let mut active_grp = state.active_group.lock().await;
            *active_grp = Some(group);
        }
    } else if let Ok(character) = state.storage.load_character(&tree.character_id) {
        // If chat tree has only 1 root message and character has alternate greetings, populate them as swipes
        if tree.root_message_ids.len() == 1 && !character.card.data.alternate_greetings.is_empty() {
            for alt in &character.card.data.alternate_greetings {
                if !alt.trim().is_empty()
                    && !tree
                        .nodes
                        .values()
                        .any(|n| n.parent_id.is_none() && n.content == *alt)
                {
                    tree.add_swipe(None, alt.clone());
                }
            }
            tree.active_root_index = 0;
            let _ = state.storage.save_chat(&tree);
        }

        let mut active_char = state.active_character.lock().await;
        *active_char = Some(character);
    }

    {
        let mut active_chat = state.active_chat.lock().await;
        *active_chat = Some(tree.clone());
    }
    {
        let mut settings = state.settings.lock().await;
        settings.active_character_id = Some(tree.character_id.clone());
        settings.active_chat_id = Some(tree.id.to_string());
        let _ = state.storage.save_settings(&settings);
    }

    Ok(tree)
}

#[tauri::command]
async fn list_chats(
    character_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ChatSummary>, String> {
    state.storage.list_chats_for_character(&character_id)
}

#[tauri::command]
async fn delete_chat(chat_id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.storage.delete_chat(&chat_id)?;
    let mut active = state.active_chat.lock().await;
    if let Some(cur) = active.as_ref() {
        if cur.id.to_string() == chat_id {
            *active = None;
        }
    }
    Ok(())
}

#[tauri::command]
async fn import_chat_jsonl(
    file_bytes: Vec<u8>,
    character_id: Option<String>,
    title: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<ChatTree, String> {
    let tree =
        state
            .storage
            .import_chat_jsonl(&file_bytes, character_id.as_deref(), title.as_deref())?;

    // Automatically switch active chat to the newly imported chat
    let character = state.storage.load_character(&tree.character_id).ok();
    {
        let mut active_chat = state.active_chat.lock().await;
        *active_chat = Some(tree.clone());
    }
    if let Some(ch) = character {
        let mut active_char = state.active_character.lock().await;
        *active_char = Some(ch);
    }
    {
        let mut settings = state.settings.lock().await;
        settings.active_character_id = Some(tree.character_id.clone());
        settings.active_chat_id = Some(tree.id.to_string());
        let _ = state.storage.save_settings(&settings);
    }

    Ok(tree)
}

#[tauri::command]
async fn export_chat_jsonl(
    chat_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let chat = state.storage.load_chat(&chat_id)?;
    let character = state.storage.load_character(&chat.character_id).ok();
    let user_persona = state.user_persona.lock().await;
    let character_name = if let Some(c) = &character {
        c.card.data.name.clone()
    } else if let Some(gid) = &chat.group_id {
        state
            .storage
            .load_group(gid)
            .map(|g| g.name)
            .unwrap_or_else(|_| "Group".to_string())
    } else {
        "Character".to_string()
    };
    let user_name = if !user_persona.name.trim().is_empty() {
        user_persona.name.clone()
    } else {
        "User".to_string()
    };

    engine::chat::export_sillytavern_chat_jsonl(&chat, &user_name, &character_name)
}

#[tauri::command]
async fn get_active_messages(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<MessageViewNode>, String> {
    let lock = state.active_chat.lock().await;
    match lock.as_ref() {
        Some(tree) => Ok(tree.get_active_view_nodes()),
        None => Ok(Vec::new()),
    }
}

#[tauri::command]
async fn append_message(
    role: AuthorRole,
    content: String,
    parent_id: Option<Uuid>,
    character_id: Option<String>,
    name: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Uuid, String> {
    let mut lock = state.active_chat.lock().await;
    let tree = lock.as_mut().ok_or("No active chat session")?;
    let node_id = tree.append_message_with_author(role, content, parent_id, character_id, name);
    state.storage.save_chat(tree)?;
    Ok(node_id)
}

#[tauri::command]
async fn edit_message(
    id: Uuid,
    new_content: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<MessageViewNode>, String> {
    let mut lock = state.active_chat.lock().await;
    let tree = lock.as_mut().ok_or("No active chat session")?;
    tree.edit_message(id, new_content)?;
    state.storage.save_chat(tree)?;
    Ok(tree.get_active_view_nodes())
}

#[tauri::command]
async fn delete_message(
    id: Uuid,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<MessageViewNode>, String> {
    let mut lock = state.active_chat.lock().await;
    let tree = lock.as_mut().ok_or("No active chat session")?;
    tree.delete_message(id)?;
    state.storage.save_chat(tree)?;
    Ok(tree.get_active_view_nodes())
}

#[tauri::command]
async fn switch_branch(
    parent_id: Option<Uuid>,
    index: usize,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<MessageViewNode>, String> {
    let mut lock = state.active_chat.lock().await;
    let tree = lock.as_mut().ok_or("No active chat session")?;
    if tree.switch_branch(parent_id, index) {
        state.storage.save_chat(tree)?;
        Ok(tree.get_active_view_nodes())
    } else {
        Err("Branch index out of bounds".into())
    }
}

// --- Settings & Persona Commands ---

#[tauri::command]
async fn get_settings(state: State<'_, Arc<AppState>>) -> Result<AppSettings, String> {
    let settings = state.settings.lock().await;
    Ok(settings.clone())
}

#[tauri::command]
async fn save_settings(
    new_settings: AppSettings,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.storage.save_settings(&new_settings)?;
    let mut settings = state.settings.lock().await;
    *settings = new_settings;
    Ok(())
}

#[tauri::command]
async fn get_all_user_personas(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<UserPersona>, String> {
    state.storage.list_user_personas()
}

#[tauri::command]
async fn get_active_user_persona(state: State<'_, Arc<AppState>>) -> Result<UserPersona, String> {
    let persona = state.user_persona.lock().await;
    Ok(persona.clone())
}

#[tauri::command]
async fn save_user_persona(
    new_persona: UserPersona,
    state: State<'_, Arc<AppState>>,
) -> Result<UserPersona, String> {
    state.storage.save_user_persona(&new_persona)?;
    let mut persona = state.user_persona.lock().await;
    if persona.id == new_persona.id {
        *persona = new_persona.clone();
    }
    Ok(new_persona)
}

#[tauri::command]
async fn delete_user_persona(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.storage.delete_user_persona(&id)?;
    let personas = state.storage.list_user_personas()?;
    let mut persona = state.user_persona.lock().await;
    if persona.id == id {
        if let Some(first) = personas.first() {
            *persona = first.clone();
            let mut settings = state.settings.lock().await;
            settings.active_persona_id = Some(first.id.clone());
            let _ = state.storage.save_settings(&settings);
        }
    }
    Ok(())
}

#[tauri::command]
async fn set_active_user_persona(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<UserPersona, String> {
    let found = state.storage.load_user_persona(&id)?;
    {
        let mut persona = state.user_persona.lock().await;
        *persona = found.clone();
    }
    {
        let mut settings = state.settings.lock().await;
        settings.active_persona_id = Some(id);
        let _ = state.storage.save_settings(&settings);
    }
    Ok(found)
}

// --- Lorebook Commands ---

#[tauri::command]
async fn get_all_lorebooks(state: State<'_, Arc<AppState>>) -> Result<Vec<Lorebook>, String> {
    state.storage.list_lorebooks()
}

#[tauri::command]
async fn get_lorebook(id: String, state: State<'_, Arc<AppState>>) -> Result<Lorebook, String> {
    state.storage.load_lorebook(&id)
}

#[tauri::command]
async fn save_lorebook(
    lorebook: Lorebook,
    state: State<'_, Arc<AppState>>,
) -> Result<Lorebook, String> {
    state.storage.save_lorebook(&lorebook)?;
    Ok(lorebook)
}

#[tauri::command]
async fn delete_lorebook(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.storage.delete_lorebook(&id)?;
    // Also remove from global_lorebook_ids if present
    let mut settings = state.settings.lock().await;
    if settings.global_lorebook_ids.contains(&id) {
        settings.global_lorebook_ids.retain(|item| item != &id);
        let _ = state.storage.save_settings(&settings);
    }
    Ok(())
}

#[tauri::command]
async fn import_lorebook(
    file_bytes: Vec<u8>,
    state: State<'_, Arc<AppState>>,
) -> Result<Lorebook, String> {
    state.storage.import_lorebook(&file_bytes)
}

#[tauri::command]
async fn export_lorebook_json(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    state.storage.export_lorebook_json(&id)
}

#[tauri::command]
async fn fetch_endpoint_models(endpoint: String, api_key: String) -> Result<Vec<String>, String> {
    fetch_models(&endpoint, &api_key).await
}
// --- Generation & Streaming Commands ---

#[tauri::command]
async fn generate_reply(
    window: Window,
    state: State<'_, Arc<AppState>>,
    is_swipe: Option<bool>,
    is_continue: Option<bool>,
    target_character_id: Option<String>,
) -> Result<(), String> {
    let is_swipe = is_swipe.unwrap_or(false);
    let is_continue = is_continue.unwrap_or(false);

    // 1. Prepare data snapshot
    let (chat_tree, user, settings, active_group_opt, active_char_opt) = {
        let chat_lock = state.active_chat.lock().await;
        let tree = chat_lock.as_ref().ok_or("No active chat session")?.clone();

        let user_lock = state.user_persona.lock().await;
        let user = user_lock.clone();

        let settings_lock = state.settings.lock().await;
        let settings = settings_lock.clone();

        let group_lock = state.active_group.lock().await;
        let group = group_lock.clone();

        let char_lock = state.active_character.lock().await;
        let character = char_lock.clone();

        (tree, user, settings, group, character)
    };

    let prompt_config = PromptConfig {
        system_template: settings.system_template.clone(),
        max_context_tokens: settings.max_context_tokens,
        max_response_tokens: settings.max_tokens,
        include_examples: true,
    };

    let is_group_chat =
        chat_tree.group_id.is_some() || chat_tree.character_id.starts_with("group:");

    let (target_character, _other_characters, prompt_messages) = if is_group_chat {
        let group = if let Some(g) = active_group_opt {
            g
        } else if let Some(gid) = &chat_tree.group_id {
            state.storage.load_group(gid)?
        } else if let Some(gid) = chat_tree.character_id.strip_prefix("group:") {
            state.storage.load_group(gid)?
        } else {
            return Err("Group data not found for group chat".to_string());
        };

        let target_id = if let Some(tid) = target_character_id {
            tid
        } else if is_swipe {
            let path = chat_tree.get_active_path();
            if let Some(last) = path.last() {
                if let Some(cid) = &last.character_id {
                    cid.clone()
                } else if let Some(name) = &last.name {
                    group
                        .members
                        .iter()
                        .find(|m| {
                            state
                                .storage
                                .load_character(&m.character_id)
                                .ok()
                                .map(|c| c.card.data.name.eq_ignore_ascii_case(name))
                                .unwrap_or(false)
                        })
                        .map(|m| m.character_id.clone())
                        .unwrap_or_else(|| {
                            group
                                .members
                                .first()
                                .map(|m| m.character_id.clone())
                                .unwrap_or_default()
                        })
                } else {
                    group
                        .members
                        .first()
                        .map(|m| m.character_id.clone())
                        .unwrap_or_default()
                }
            } else {
                group
                    .members
                    .first()
                    .map(|m| m.character_id.clone())
                    .unwrap_or_default()
            }
        } else {
            let last_speaker = chat_tree.get_active_path().iter().rev().find_map(|m| {
                if m.role == AuthorRole::Assistant {
                    m.character_id.as_deref()
                } else {
                    None
                }
            });
            resolve_next_speaker(&group, last_speaker)
                .ok_or_else(|| "No active/unmuted character available in group".to_string())?
        };

        let target_char = state.storage.load_character(&target_id)?;

        let mut other_chars = Vec::new();
        for member in &group.members {
            if member.character_id != target_char.id {
                if let Ok(ch) = state.storage.load_character(&member.character_id) {
                    other_chars.push(ch);
                }
            }
        }

        let mut active_lorebooks = Vec::new();
        let target_embedded = target_char
            .card
            .data
            .character_book
            .as_ref()
            .map(|cb| cb.to_lorebook());
        if let Some(book) = &target_embedded {
            active_lorebooks.push(book);
        }

        let mut linked_books = Vec::new();
        for lb_id in &target_char.card.data.lorebook_ids {
            if let Ok(b) = state.storage.load_lorebook(lb_id) {
                linked_books.push(b);
            }
        }
        for other in &other_chars {
            if let Some(cb) = &other.card.data.character_book {
                let ob = cb.to_lorebook();
                linked_books.push(ob);
            }
            for lb_id in &other.card.data.lorebook_ids {
                if !target_char.card.data.lorebook_ids.contains(lb_id) {
                    if let Ok(b) = state.storage.load_lorebook(lb_id) {
                        linked_books.push(b);
                    }
                }
            }
        }
        for b in &linked_books {
            active_lorebooks.push(b);
        }

        let mut global_books = Vec::new();
        for lb_id in &settings.global_lorebook_ids {
            if !target_char.card.data.lorebook_ids.contains(lb_id) {
                if let Ok(b) = state.storage.load_lorebook(lb_id) {
                    global_books.push(b);
                }
            }
        }
        for b in &global_books {
            active_lorebooks.push(b);
        }

        let other_refs: Vec<(&engine::character::CharacterData, &str)> = other_chars
            .iter()
            .map(|c| (&c.card.data, c.id.as_str()))
            .collect();

        let prompt = build_group_chat_prompt_with_lorebooks(
            &target_char.card.data,
            &target_char.id,
            &other_refs,
            &user,
            &chat_tree,
            &prompt_config,
            &active_lorebooks,
        );

        (target_char, other_chars, prompt)
    } else {
        let character = active_char_opt.ok_or("No active character")?;

        let mut active_lorebooks = Vec::new();
        let embedded_book = character
            .card
            .data
            .character_book
            .as_ref()
            .map(|cb| cb.to_lorebook());
        if let Some(book) = &embedded_book {
            active_lorebooks.push(book);
        }

        let mut char_linked_books = Vec::new();
        for lb_id in &character.card.data.lorebook_ids {
            if let Ok(book) = state.storage.load_lorebook(lb_id) {
                char_linked_books.push(book);
            }
        }
        for book in &char_linked_books {
            active_lorebooks.push(book);
        }

        let mut global_books = Vec::new();
        for lb_id in &settings.global_lorebook_ids {
            if !character.card.data.lorebook_ids.contains(lb_id) {
                if let Ok(book) = state.storage.load_lorebook(lb_id) {
                    global_books.push(book);
                }
            }
        }
        for book in &global_books {
            active_lorebooks.push(book);
        }

        let prompt = build_chat_prompt_with_lorebooks(
            &character.card.data,
            &user,
            &chat_tree,
            &prompt_config,
            &active_lorebooks,
        );

        (character, Vec::new(), prompt)
    };

    let gen_params = GenerationParams {
        temperature: settings.temperature,
        top_p: settings.top_p,
        frequency_penalty: settings.frequency_penalty,
        presence_penalty: settings.presence_penalty,
        max_tokens: settings.max_tokens,
        stop: settings.stop_sequences.clone(),
    };

    // 3. Setup cancellation token
    let (cancel_tx, cancel_rx) = watch::channel(false);
    {
        let mut abort = state.abort_tx.lock().await;
        *abort = Some(cancel_tx);
    }

    let _ = window.emit(
        "llm-start",
        serde_json::json!({
            "character_id": target_character.id,
            "character_name": target_character.card.data.name,
        }),
    );

    let win = window.clone();
    let result = stream_chat_completion(
        &settings.endpoint,
        &settings.api_key,
        &settings.active_model,
        &prompt_messages,
        &gen_params,
        Some(cancel_rx),
        move |token| {
            let _ = win.emit("llm-token", token);
        },
    )
    .await;

    // Clear abort handle
    {
        let mut abort = state.abort_tx.lock().await;
        *abort = None;
    }

    let accumulated_text = result?;

    // 4. Save assistant response into ChatTree with author metadata
    if !accumulated_text.trim().is_empty() {
        let mut lock = state.active_chat.lock().await;
        if let Some(tree) = lock.as_mut() {
            let author_id = Some(target_character.id.clone());
            let author_name = Some(target_character.card.data.name.clone());

            if is_continue {
                let path = tree.get_active_path();
                if let Some(last) = path.last() {
                    let last_id = last.id;
                    let _ = tree.append_to_message(last_id, &accumulated_text);
                }
            } else if is_swipe {
                let path = tree.get_active_path();
                let parent_id = if let Some(last) = path.last() {
                    last.parent_id
                } else {
                    None
                };
                tree.add_swipe_with_author(parent_id, accumulated_text, author_id, author_name);
            } else {
                let parent_id = tree.get_last_node_id();
                tree.append_message_with_author(
                    AuthorRole::Assistant,
                    accumulated_text,
                    parent_id,
                    author_id,
                    author_name,
                );
            }
            let _ = state.storage.save_chat(tree);
        }
    }

    let _ = window.emit("llm-done", ());
    Ok(())
}
#[tauri::command]
async fn abort_generation(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let abort = state.abort_tx.lock().await;
    if let Some(tx) = abort.as_ref() {
        let _ = tx.send(true);
    }
    Ok(())
}

// --- Sync Commands ---

#[tauri::command]
async fn get_sync_device_info(state: State<'_, Arc<AppState>>) -> Result<SyncDeviceInfo, String> {
    let mgr = {
        let mgr_lock = state.sync_manager.read();
        mgr_lock
            .as_ref()
            .ok_or("Sync manager not initialized")?
            .clone()
    };
    Ok(mgr.get_device_info().await)
}

#[tauri::command]
async fn scan_sync_peers(
    timeout_ms: Option<u64>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<DiscoveredPeer>, String> {
    let mgr = {
        let mgr_lock = state.sync_manager.read();
        mgr_lock
            .as_ref()
            .ok_or("Sync manager not initialized")?
            .clone()
    };
    mgr.scan_peers(timeout_ms.unwrap_or(1500)).await
}

#[tauri::command]
async fn trigger_sync(
    target_address: String,
    pin: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<SyncStats, String> {
    let mgr = {
        let mgr_lock = state.sync_manager.read();
        mgr_lock
            .as_ref()
            .ok_or("Sync manager not initialized")?
            .clone()
    };
    mgr.sync_with(&target_address, pin).await
}

#[tauri::command]
async fn update_sync_settings(
    device_name: Option<String>,
    sync_pin: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let mut settings = state.settings.lock().await;
    if let Some(name) = &device_name {
        settings.device_name = Some(name.clone());
        let mgr_opt = {
            let mgr_lock = state.sync_manager.read();
            mgr_lock.clone()
        };
        if let Some(mgr) = mgr_opt {
            mgr.update_device_name(name.clone()).await;
        }
    }
    if let Some(pin) = &sync_pin {
        settings.sync_pin = if pin.trim().is_empty() {
            None
        } else {
            Some(pin.clone())
        };
    }
    state.storage.save_settings(&settings)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Resolve app storage directory
            let base_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("./tavern_data"));

            let storage = Arc::new(StorageManager::new(base_dir.clone()));
            let settings = storage.load_settings();
            let user_persona = if let Some(p_id) = &settings.active_persona_id {
                storage.load_user_persona(p_id).unwrap_or_else(|_| {
                    storage
                        .list_user_personas()
                        .ok()
                        .and_then(|list| list.into_iter().next())
                        .unwrap_or_default()
                })
            } else {
                storage
                    .list_user_personas()
                    .ok()
                    .and_then(|list| list.into_iter().next())
                    .unwrap_or_default()
            };

            let initial_char = if let Some(char_id) = &settings.active_character_id {
                storage.load_character(char_id).ok()
            } else {
                storage
                    .list_characters()
                    .ok()
                    .and_then(|list| list.into_iter().next())
            };

            let initial_chat = if let Some(chat_id) = &settings.active_chat_id {
                storage.load_chat(chat_id).ok()
            } else if let Some(ch) = &initial_char {
                storage
                    .list_chats_for_character(&ch.id)
                    .ok()
                    .and_then(|chats| {
                        chats
                            .into_iter()
                            .next()
                            .and_then(|c| storage.load_chat(&c.id).ok())
                    })
            } else {
                None
            };

            let initial_group = if let Some(chat) = &initial_chat {
                if let Some(gid) = &chat.group_id {
                    storage.load_group(gid).ok()
                } else {
                    None
                }
            } else {
                None
            };

            let state = Arc::new(AppState {
                storage,
                active_chat: Mutex::new(initial_chat),
                active_character: Mutex::new(initial_char),
                active_group: Mutex::new(initial_group),
                settings: Mutex::new(settings.clone()),
                user_persona: Mutex::new(user_persona),
                abort_tx: Mutex::new(None),
                sync_manager: parking_lot::RwLock::new(None),
            });

            let sync_mgr = Arc::new(SyncManager::new(
                base_dir,
                &settings,
                Arc::clone(&state),
                app.handle().clone(),
            ));
            sync_mgr.start();
            {
                let mut w = state.sync_manager.write();
                *w = Some(sync_mgr);
            }

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_characters,
            get_character,
            save_character,
            delete_character,
            import_character_card,
            export_card_png,
            export_card_json,
            get_all_groups,
            get_group,
            save_group,
            delete_group,
            create_group_chat,
            list_group_chats,
            set_message_author,
            create_chat,
            load_chat,
            list_chats,
            delete_chat,
            import_chat_jsonl,
            export_chat_jsonl,
            get_active_messages,
            append_message,
            edit_message,
            delete_message,
            switch_branch,
            get_settings,
            save_settings,
            get_all_user_personas,
            get_active_user_persona,
            save_user_persona,
            delete_user_persona,
            set_active_user_persona,
            fetch_endpoint_models,
            generate_reply,
            abort_generation,
            get_all_lorebooks,
            get_lorebook,
            save_lorebook,
            delete_lorebook,
            import_lorebook,
            export_lorebook_json,
            get_sync_device_info,
            scan_sync_peers,
            trigger_sync,
            update_sync_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
