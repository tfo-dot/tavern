use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{watch, Mutex};
use uuid::Uuid;

use engine::character::{Character, UserPersona};
use engine::chat::{AuthorRole, ChatTree, MessageViewNode};
use engine::llm::{fetch_models, stream_chat_completion, GenerationParams};
use engine::parser::{export_character_json, export_character_png, parse_character_card};
use engine::prompt::{build_chat_prompt, PromptConfig};

use tauri::{Emitter, Manager, State, Window};

pub mod storage;
use storage::{AppSettings, ChatSummary, StorageManager};

pub struct AppState {
    pub storage: Arc<StorageManager>,
    pub active_chat: Mutex<Option<ChatTree>>,
    pub active_character: Mutex<Option<Character>>,
    pub settings: Mutex<AppSettings>,
    pub user_persona: Mutex<UserPersona>,
    pub abort_tx: Mutex<Option<watch::Sender<bool>>>,
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
            base64::engine::general_purpose::STANDARD.decode(b64_part.as_bytes()).ok()
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
    let character = state.storage.load_character(&tree.character_id)?;

    // If chat tree has only 1 root message and character has alternate greetings, populate them as swipes
    if tree.root_message_ids.len() == 1 && !character.card.data.alternate_greetings.is_empty() {
        for alt in &character.card.data.alternate_greetings {
            if !alt.trim().is_empty() && !tree.nodes.values().any(|n| n.parent_id.is_none() && n.content == *alt) {
                tree.add_swipe(None, alt.clone());
            }
        }
        tree.active_root_index = 0;
        let _ = state.storage.save_chat(&tree);
    }

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
    state: State<'_, Arc<AppState>>,
) -> Result<Uuid, String> {
    let mut lock = state.active_chat.lock().await;
    let tree = lock.as_mut().ok_or("No active chat session")?;
    let node_id = tree.append_message(role, content, parent_id);
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
async fn get_active_user_persona(
    state: State<'_, Arc<AppState>>,
) -> Result<UserPersona, String> {
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
async fn delete_user_persona(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
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
#[tauri::command]
async fn fetch_endpoint_models(
    endpoint: String,
    api_key: String,
) -> Result<Vec<String>, String> {
    fetch_models(&endpoint, &api_key).await
}

// --- Generation & Streaming Commands ---

#[tauri::command]
async fn generate_reply(
    window: Window,
    state: State<'_, Arc<AppState>>,
    is_swipe: bool,
) -> Result<(), String> {
    // 1. Prepare data snapshot
    let (chat_tree, character, user, settings) = {
        let chat_lock = state.active_chat.lock().await;
        let tree = chat_lock.as_ref().ok_or("No active chat session")?.clone();

        let char_lock = state.active_character.lock().await;
        let character = char_lock.as_ref().ok_or("No active character")?.clone();

        let user_lock = state.user_persona.lock().await;
        let user = user_lock.clone();

        let settings_lock = state.settings.lock().await;
        let settings = settings_lock.clone();

        (tree, character, user, settings)
    };

    let prompt_config = PromptConfig {
        system_template: settings.system_template.clone(),
        max_context_tokens: settings.max_context_tokens,
        max_response_tokens: settings.max_tokens,
        include_examples: true,
    };

    // 2. Build OpenAI-compatible chat prompt
    let prompt_messages = build_chat_prompt(&character.card.data, &user, &chat_tree, &prompt_config);

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

    // 4. Save assistant response into ChatTree
    if !accumulated_text.trim().is_empty() {
        let mut lock = state.active_chat.lock().await;
        if let Some(tree) = lock.as_mut() {
            let parent_id = if is_swipe {
                // If it's a swipe on the current assistant message, parent is the node before it
                let path = tree.get_active_path();
                if let Some(last) = path.last() {
                    last.parent_id
                } else {
                    None
                }
            } else {
                tree.get_last_node_id()
            };

            if is_swipe {
                tree.add_swipe(parent_id, accumulated_text);
            } else {
                tree.append_message(AuthorRole::Assistant, accumulated_text, parent_id);
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

            let storage = Arc::new(StorageManager::new(base_dir));
            let settings = storage.load_settings();
            let user_persona = if let Some(p_id) = &settings.active_persona_id {
                storage.load_user_persona(p_id).unwrap_or_else(|_| {
                    storage.list_user_personas().ok().and_then(|list| list.into_iter().next()).unwrap_or_default()
                })
            } else {
                storage.list_user_personas().ok().and_then(|list| list.into_iter().next()).unwrap_or_default()
            };

            let initial_char = if let Some(char_id) = &settings.active_character_id {
                storage.load_character(char_id).ok()
            } else {
                storage.list_characters().ok().and_then(|list| list.into_iter().next())
            };

            let initial_chat = if let Some(chat_id) = &settings.active_chat_id {
                storage.load_chat(chat_id).ok()
            } else if let Some(ch) = &initial_char {
                storage.list_chats_for_character(&ch.id).ok().and_then(|chats| {
                    chats.into_iter().next().and_then(|c| storage.load_chat(&c.id).ok())
                })
            } else {
                None
            };

            let state = Arc::new(AppState {
                storage,
                active_chat: Mutex::new(initial_chat),
                active_character: Mutex::new(initial_char),
                settings: Mutex::new(settings),
                user_persona: Mutex::new(user_persona),
                abort_tx: Mutex::new(None),
            });

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
            create_chat,
            load_chat,
            list_chats,
            delete_chat,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
