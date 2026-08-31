pub mod character;
pub mod parser;
pub mod chat;
pub mod llm;
pub mod template;
pub mod prompt;
pub mod lorebook;

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

    #[test]
    fn test_lorebook_activation_and_selective_logic() {
        use lorebook::{Lorebook, LorebookEntry, SelectiveLogic, scan_lorebooks_for_activation, LorebookPosition};

        let mut book = Lorebook::new("World Lore".to_string(), "Lore of the realm".to_string());
        book.scan_depth = 3;

        // Entry 1: Standard primary key
        book.add_entry(LorebookEntry {
            keys: vec!["tavern".to_string(), "inn".to_string()],
            content: "The tavern was built 300 years ago by dwarven masons.".to_string(),
            comment: "Tavern History".to_string(),
            position: LorebookPosition::BeforeChar,
            ..Default::default()
        });

        // Entry 2: Selective logic (AndAll)
        book.add_entry(LorebookEntry {
            keys: vec!["sword".to_string()],
            secondary_keys: vec!["dragon".to_string(), "flame".to_string()],
            selective: true,
            selective_logic: SelectiveLogic::AndAll,
            content: "The Dragonflame Blade is an ancient relic.".to_string(),
            comment: "Dragon Blade".to_string(),
            position: LorebookPosition::AfterChar,
            ..Default::default()
        });

        // Entry 3: Constant entry
        book.add_entry(LorebookEntry {
            constant: true,
            content: "The world is called Elyria.".to_string(),
            comment: "World Name".to_string(),
            position: LorebookPosition::TopSystem,
            ..Default::default()
        });

        // Test with text containing "tavern" and "sword" + "dragon" (missing "flame")
        let messages = vec![
            "Welcome to the cozy tavern!".to_string(),
            "I draw my sword against the dragon.".to_string(),
        ];

        let activated = scan_lorebooks_for_activation(&[&book], &messages, "");
        let comments: Vec<&str> = activated.iter().map(|e| e.comment.as_str()).collect();

        assert!(comments.contains(&"World Name"), "Constant entry should activate");
        assert!(comments.contains(&"Tavern History"), "Tavern key should match");
        assert!(!comments.contains(&"Dragon Blade"), "Dragon Blade should NOT match because 'flame' is missing");

        // Now test when both dragon and flame are present
        let messages2 = vec![
            "I strike with the sword of dragon flame!".to_string(),
        ];
        let activated2 = scan_lorebooks_for_activation(&[&book], &messages2, "");
        let comments2: Vec<&str> = activated2.iter().map(|e| e.comment.as_str()).collect();
        assert!(comments2.contains(&"Dragon Blade"), "Dragon Blade should match when all secondary keys present");
    }

    #[test]
    fn test_lorebook_recursive_scanning() {
        use lorebook::{Lorebook, LorebookEntry, scan_lorebooks_for_activation, LorebookPosition};

        let mut book = Lorebook::new("Magic Lore".to_string(), "Lore".to_string());
        book.recursive_scanning = true;

        // Entry 1 triggers on "crystal" -> mentions "Eldritch Order"
        book.add_entry(LorebookEntry {
            keys: vec!["crystal".to_string()],
            content: "A glowing crystal crafted by the Eldritch Order.".to_string(),
            comment: "Crystal".to_string(),
            ..Default::default()
        });

        // Entry 2 triggers on "Eldritch Order"
        book.add_entry(LorebookEntry {
            keys: vec!["Eldritch Order".to_string()],
            content: "The Eldritch Order was founded in the First Era.".to_string(),
            comment: "Order".to_string(),
            ..Default::default()
        });

        let messages = vec!["I found a strange crystal on the ground.".to_string()];
        let activated = scan_lorebooks_for_activation(&[&book], &messages, "");
        let comments: Vec<&str> = activated.iter().map(|e| e.comment.as_str()).collect();

        assert!(comments.contains(&"Crystal"));
        assert!(comments.contains(&"Order"), "Order should be activated via recursive scanning");
    }

    #[test]
    fn test_sillytavern_format_import() {
        use lorebook::parse_lorebook;

        let st_json = r#"{
            "name": "SillyTavern World",
            "description": "Exported from SillyTavern",
            "scan_depth": 5,
            "token_budget": 1024,
            "recursive_scanning": true,
            "entries": {
                "0": {
                    "uid": 0,
                    "key": ["tavern", "inn"],
                    "keysecondary": ["ale", "drink"],
                    "comment": "Tavern Entry",
                    "content": "A warm resting place.",
                    "constant": false,
                    "selective": true,
                    "selectiveLogic": 0,
                    "order": 50,
                    "position": "before_char",
                    "disable": false
                },
                "1": {
                    "uid": 1,
                    "key": ["castle"],
                    "comment": "Castle Entry",
                    "content": "A grand stone castle.",
                    "constant": true,
                    "disable": false
                }
            }
        }"#;

        let parsed = parse_lorebook(st_json.as_bytes()).expect("failed to parse ST lorebook JSON");
        assert_eq!(parsed.name, "SillyTavern World");
        assert_eq!(parsed.scan_depth, 5);
        assert_eq!(parsed.token_budget, 1024);
        assert!(parsed.recursive_scanning);
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].keys, vec!["tavern", "inn"]);
        assert_eq!(parsed.entries[0].secondary_keys, vec!["ale", "drink"]);
        assert_eq!(parsed.entries[0].comment, "Tavern Entry");
        assert!(parsed.entries[0].enabled);
        assert!(parsed.entries[1].constant);
    }

    #[test]
    fn test_character_card_with_embedded_book_png_roundtrip() {
        use lorebook::{Lorebook, LorebookEntry};

        let mut card = CharacterCardV2::default();
        card.data.name = "Eldrin".to_string();

        let mut book = Lorebook::new("Eldrin Lore".to_string(), "Backstory".to_string());
        book.add_entry(LorebookEntry {
            keys: vec!["staff".to_string()],
            content: "Eldrin's staff is made of weeping willow wood.".to_string(),
            comment: "Staff".to_string(),
            ..Default::default()
        });
        card.data.character_book = Some(book.to_character_book());

        let png_bytes = export_character_png(&card, None).expect("failed to export png");
        let (parsed_card, _) = parse_character_card(&png_bytes).expect("failed to parse png");

        assert_eq!(parsed_card.data.name, "Eldrin");
        let parsed_book = parsed_card.data.character_book.expect("embedded book missing");
        assert_eq!(parsed_book.entries.len(), 1);
        assert_eq!(parsed_book.entries[0].comment, "Staff");
        assert_eq!(parsed_book.entries[0].keys, vec!["staff"]);
    }

    #[test]
    fn test_prompt_assembly_with_lorebook_positions() {
        use lorebook::{Lorebook, LorebookEntry, LorebookPosition};
        use prompt::build_chat_prompt_with_lorebooks;

        let mut data = CharacterData::default();
        data.name = "Seraphina".to_string();
        data.description = "A wise mage.".to_string();
        data.scenario = "At the tavern.".to_string();

        let user = UserPersona::default();
        let mut tree = ChatTree::new("char1".to_string(), "Chat 1".to_string());
        let m1 = tree.append_message(AuthorRole::Assistant, "Greetings!".to_string(), None);
        let m2 = tree.append_message(AuthorRole::User, "Tell me about the magic tower.".to_string(), Some(m1));
        tree.append_message(AuthorRole::Assistant, "The tower is high.".to_string(), Some(m2));

        let mut book = Lorebook::new("World".to_string(), "".to_string());
        book.add_entry(LorebookEntry {
            keys: vec!["tower".to_string()],
            content: "THE_TOWER_LORE_BEFORE_CHAR".to_string(),
            position: LorebookPosition::BeforeChar,
            ..Default::default()
        });
        book.add_entry(LorebookEntry {
            keys: vec!["tower".to_string()],
            content: "THE_TOWER_LORE_AT_DEPTH".to_string(),
            position: LorebookPosition::AtDepth,
            depth: 1,
            ..Default::default()
        });

        let config = PromptConfig::default();
        let msgs = build_chat_prompt_with_lorebooks(&data, &user, &tree, &config, &[&book]);

        // Verify system message has before_char content
        assert!(msgs[0].content.contains("THE_TOWER_LORE_BEFORE_CHAR"));
        // Verify at_depth content was inserted in history
        let at_depth_found = msgs.iter().any(|m| m.content.contains("THE_TOWER_LORE_AT_DEPTH"));
        assert!(at_depth_found, "At-depth entry should be inserted into history messages");
    }
}
