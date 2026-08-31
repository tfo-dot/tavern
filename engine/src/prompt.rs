use serde::{Deserialize, Serialize};
use crate::character::{CharacterData, UserPersona};
use crate::chat::{AuthorRole, ChatTree};
use crate::template::interpolate_macros;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: String, // "system", "user", "assistant"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub system_template: String,
    pub max_context_tokens: usize,
    pub max_response_tokens: usize,
    pub include_examples: bool,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            system_template: "Write {{char}}'s next reply in a fictional roleplay chat between {{char}} and {{user}}.\nFollow character traits, scenario, and tone strictly. Stay in character.".to_string(),
            max_context_tokens: 4096,
            max_response_tokens: 800,
            include_examples: true,
        }
    }
}

/// Rough token estimation (~3.5 characters per token)
pub fn estimate_tokens(text: &str) -> usize {
    let char_count = text.chars().count();
    (char_count + 3) / 4
}

/// Assembles OpenAI-compatible messages for the chat completion endpoint
pub fn build_chat_prompt(
    char_data: &CharacterData,
    user: &UserPersona,
    chat_tree: &ChatTree,
    config: &PromptConfig,
) -> Vec<ChatMessage> {
    let char_name = if char_data.name.is_empty() { "Character" } else { &char_data.name };
    let user_name = if user.name.is_empty() { "User" } else { &user.name };

    let mut system_parts = Vec::new();

    // 1. Base instructions from system template
    if !config.system_template.is_empty() {
        let base = interpolate_macros(
            &config.system_template,
            char_name,
            user_name,
            &char_data.description,
            &char_data.personality,
            &char_data.scenario,
            &user.description,
        );
        system_parts.push(base);
    }

    // 2. Character custom system prompt override if specified
    if !char_data.system_prompt.is_empty() {
        let char_sys = interpolate_macros(
            &char_data.system_prompt,
            char_name,
            user_name,
            &char_data.description,
            &char_data.personality,
            &char_data.scenario,
            &user.description,
        );
        system_parts.push(char_sys);
    }

    // 3. Character description & personality
    if !char_data.description.is_empty() {
        let desc = interpolate_macros(
            &char_data.description,
            char_name,
            user_name,
            &char_data.description,
            &char_data.personality,
            &char_data.scenario,
            &user.description,
        );
        system_parts.push(format!("[Character Description for {char_name}]\n{desc}"));
    }

    if !char_data.personality.is_empty() {
        let pers = interpolate_macros(
            &char_data.personality,
            char_name,
            user_name,
            &char_data.description,
            &char_data.personality,
            &char_data.scenario,
            &user.description,
        );
        system_parts.push(format!("[Character Personality for {char_name}]\n{pers}"));
    }

    // 4. Scenario
    if !char_data.scenario.is_empty() {
        let scn = interpolate_macros(
            &char_data.scenario,
            char_name,
            user_name,
            &char_data.description,
            &char_data.personality,
            &char_data.scenario,
            &user.description,
        );
        system_parts.push(format!("[Current Scenario]\n{scn}"));
    }

    // 5. User persona
    if !user.description.is_empty() {
        let udesc = interpolate_macros(
            &user.description,
            char_name,
            user_name,
            &char_data.description,
            &char_data.personality,
            &char_data.scenario,
            &user.description,
        );
        system_parts.push(format!("[User Persona for {user_name}]\n{udesc}"));
    }

    let full_system_content = system_parts.join("\n\n");
    let mut messages = Vec::new();

    if !full_system_content.is_empty() {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: full_system_content,
        });
    }

    // 6. Dialogue Examples (mes_example) if enabled
    if config.include_examples && !char_data.mes_example.is_empty() {
        let parsed_examples = interpolate_macros(
            &char_data.mes_example,
            char_name,
            user_name,
            &char_data.description,
            &char_data.personality,
            &char_data.scenario,
            &user.description,
        );
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: format!("[Example Dialogue]\n{parsed_examples}"),
        });
    }

    // 7. Active Chat History
    let active_path = chat_tree.get_active_path();
    let mut history_messages = Vec::new();

    for node in active_path {
        let role = match node.role {
            AuthorRole::User => "user".to_string(),
            AuthorRole::Assistant => "assistant".to_string(),
            AuthorRole::System => "system".to_string(),
        };

        let content = interpolate_macros(
            &node.content,
            char_name,
            user_name,
            &char_data.description,
            &char_data.personality,
            &char_data.scenario,
            &user.description,
        );

        history_messages.push(ChatMessage { role, content });
    }

    // 8. Post-history instructions if present
    if !char_data.post_history_instructions.is_empty() {
        let post_hist = interpolate_macros(
            &char_data.post_history_instructions,
            char_name,
            user_name,
            &char_data.description,
            &char_data.personality,
            &char_data.scenario,
            &user.description,
        );
        history_messages.push(ChatMessage {
            role: "system".to_string(),
            content: post_hist,
        });
    }

    // Apply token budgeting on history messages (preserve system messages, trim oldest history if needed)
    let available_tokens = config.max_context_tokens.saturating_sub(config.max_response_tokens);
    let mut current_tokens: usize = messages.iter().map(|m| estimate_tokens(&m.content)).sum();

    let mut included_history = Vec::new();
    for msg in history_messages.into_iter().rev() {
        let msg_tokens = estimate_tokens(&msg.content);
        if current_tokens + msg_tokens <= available_tokens || included_history.is_empty() {
            current_tokens += msg_tokens;
            included_history.push(msg);
        } else {
            break;
        }
    }
    included_history.reverse();
    messages.extend(included_history);

    messages
}
