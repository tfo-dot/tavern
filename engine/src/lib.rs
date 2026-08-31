pub mod character;
pub mod parser;
pub mod chat;
pub mod llm;
pub mod template;
pub mod prompt;

#[cfg(test)]
mod tests {
    use super::*;
    use character::{CharacterCardV2, CharacterData, UserPersona};
    use chat::{AuthorRole, ChatTree};
    use parser::{export_character_png, parse_character_card};
    use prompt::{build_chat_prompt, PromptConfig};

    #[test]
    fn test_character_png_roundtrip() {
        let mut card = CharacterCardV2::default();
        card.data.name = "Seraphina".to_string();
        card.data.personality = "Kind, mystical, ancient mage".to_string();
        card.data.scenario = "A quiet tavern at dusk".to_string();
        card.data.first_mes = "Welcome, traveler. What brings you to my hearth?".to_string();

        let png_bytes = export_character_png(&card, None).expect("failed to export png");
        let (parsed_card, avatar) = parse_character_card(&png_bytes).expect("failed to parse png");

        assert_eq!(parsed_card.data.name, "Seraphina");
        assert_eq!(parsed_card.data.personality, "Kind, mystical, ancient mage");
        assert!(avatar.is_some());
    }

    #[test]
    fn test_chat_tree_swipes_and_edits() {
        let mut tree = ChatTree::new("char_1".to_string(), "Adventure".to_string());
        
        // Root greeting
        let m1 = tree.append_message(AuthorRole::Assistant, "Hello {{user}}!".to_string(), None);
        // User reply
        let m2 = tree.append_message(AuthorRole::User, "Hello {{char}}!".to_string(), Some(m1));
        // Assistant reply variation 1
        let a1 = tree.append_message(AuthorRole::Assistant, "Nice to meet you.".to_string(), Some(m2));
        
        // Swipe: variation 2
        let a2 = tree.add_swipe(Some(m2), "A pleasure to make your acquaintance.".to_string()).unwrap();

        let view_nodes = tree.get_active_view_nodes();
        assert_eq!(view_nodes.len(), 3);
        let last_node = &view_nodes[2];
        assert_eq!(last_node.id, a2);
        assert_eq!(last_node.sibling_index, 1);
        assert_eq!(last_node.sibling_total, 2);
        assert!(last_node.can_swipe_left);
        assert!(!last_node.can_swipe_right);

        // Switch back to variation 1
        assert!(tree.switch_branch(Some(m2), 0));
        let view_nodes2 = tree.get_active_view_nodes();
        assert_eq!(view_nodes2[2].id, a1);
        assert_eq!(view_nodes2[2].sibling_index, 0);

        // Edit variation 1
        tree.edit_message(a1, "Edited response.".to_string()).unwrap();
        let view_nodes3 = tree.get_active_view_nodes();
        assert_eq!(view_nodes3[2].content, "Edited response.");

        // Delete variation 1 -> variation 2 should become active
        tree.delete_message(a1).unwrap();
        let view_nodes4 = tree.get_active_view_nodes();
        assert_eq!(view_nodes4.len(), 3);
        assert_eq!(view_nodes4[2].id, a2);
        assert_eq!(view_nodes4[2].sibling_total, 1);
    }

    #[test]
    fn test_prompt_assembly() {
        let mut data = CharacterData::default();
        data.name = "Alice".to_string();
        data.description = "A friendly botanist in a futuristic greenhouse.".to_string();
        data.personality = "Curious, cheerful".to_string();
        data.scenario = "Exploring a rare alien flower".to_string();

        let user = UserPersona {
            id: "u1".to_string(),
            name: "Bob".to_string(),
            description: "A visiting scientist".to_string(),
            avatar_data_url: None,
        };

        let mut tree = ChatTree::new("alice".to_string(), "Chat 1".to_string());
        let m1 = tree.append_message(AuthorRole::Assistant, "Look at this bloom, Bob!".to_string(), None);
        tree.append_message(AuthorRole::User, "Is it dangerous?".to_string(), Some(m1));

        let config = PromptConfig::default();
        let prompt_messages = build_chat_prompt(&data, &user, &tree, &config);

        assert!(!prompt_messages.is_empty());
        assert_eq!(prompt_messages[0].role, "system");
        assert!(prompt_messages[0].content.contains("Alice"));
        assert!(prompt_messages[0].content.contains("Bob"));
        assert_eq!(prompt_messages.last().unwrap().role, "user");
        assert_eq!(prompt_messages.last().unwrap().content, "Is it dangerous?");
    }
}
