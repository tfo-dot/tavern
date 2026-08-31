use engine::character::{Character, CharacterCardV1, CharacterCardV2, CharacterData, UserPersona};
use engine::chat::{AuthorRole, ChatTree};
use engine::parser::{export_character_json, export_character_png, parse_character_card};
use engine::prompt::{build_chat_prompt, estimate_tokens, PromptConfig};
use engine::template::interpolate_macros;

#[test]
fn test_v1_card_import_and_conversion() {
    let v1_json = r#"{
        "name": "Evelyn",
        "description": "A sharp-witted detective in Victorian London.",
        "personality": "Perceptive, skeptical, composed",
        "scenario": "Investigating a missing artifact in the British Museum",
        "first_mes": "Ah, you must be my new assistant. Have a look at this broken glass.",
        "mes_example": "<START>\n{{user}}: What happened here?\n{{char}}: *kneels by the pedestal* Foul play, without question."
    }"#;

    let (card, avatar) = parse_character_card(v1_json.as_bytes()).expect("Failed to parse V1 card");
    assert_eq!(card.data.name, "Evelyn");
    assert_eq!(card.data.personality, "Perceptive, skeptical, composed");
    assert!(avatar.is_none());

    // Export to V2 JSON
    let v2_json = export_character_json(&card).expect("Failed to export V2 JSON");
    let (reparsed_card, _) = parse_character_card(v2_json.as_bytes()).expect("Failed to reparse V2 JSON");
    assert_eq!(reparsed_card.data.name, "Evelyn");
    assert_eq!(reparsed_card.spec, "chara_card_v2");
}

#[test]
fn test_png_card_with_custom_dimensions_roundtrip() {
    let mut card = CharacterCardV2::default();
    card.data.name = "Kaelen".to_string();
    card.data.description = "A rogue cybernetic hacker.".to_string();
    card.data.first_mes = "Jack in, we only have three minutes.".to_string();
    card.data.tags = vec!["Cyberpunk".to_string(), "Hacker".to_string()];

    // Generate PNG card
    let png_bytes = export_character_png(&card, None).expect("Failed to export PNG card");
    assert!(png_bytes.len() > 100);

    // Read back PNG card
    let (parsed_card, avatar_url) = parse_character_card(&png_bytes).expect("Failed to parse PNG card");
    assert_eq!(parsed_card.data.name, "Kaelen");
    assert_eq!(parsed_card.data.description, "A rogue cybernetic hacker.");
    assert_eq!(parsed_card.data.tags, vec!["Cyberpunk", "Hacker"]);
    assert!(avatar_url.is_some());
    assert!(avatar_url.unwrap().starts_with("data:image/png;base64,"));
}

#[test]
fn test_macro_interpolation_comprehensive() {
    let template = "Hello {{user}}, I am {{char}}.\nSetting: {{scenario}}\nMy traits: {{personality}}\nAbout you: {{persona}}";
    let interpolated = interpolate_macros(
        template,
        "Seraphina",
        "Alex",
        "A mystical barkeeper",
        "Witty and calm",
        "A cozy fireside tavern",
        "A wandering cartographer",
    );

    assert!(interpolated.contains("Hello Alex, I am Seraphina."));
    assert!(interpolated.contains("Setting: A cozy fireside tavern"));
    assert!(interpolated.contains("My traits: Witty and calm"));
    assert!(interpolated.contains("About you: A wandering cartographer"));
}

#[test]
fn test_multi_swipe_branching_and_navigation() {
    let mut tree = ChatTree::new("char_test".to_string(), "Deep Branching Test".to_string());

    // 1. Initial Greeting (Root)
    let root_id = tree.append_message(AuthorRole::Assistant, "Greetings traveler.".to_string(), None);

    // 2. User Message
    let user_msg_1 = tree.append_message(AuthorRole::User, "Where are we?".to_string(), Some(root_id));

    // 3. Assistant Generation 1
    let gen_1 = tree.append_message(AuthorRole::Assistant, "In the Whispering Glade.".to_string(), Some(user_msg_1));

    // 4. Swipe 2 (Add swipe to user_msg_1)
    let gen_2 = tree.add_swipe(Some(user_msg_1), "In the heart of the Obsidian Citadel.".to_string()).unwrap();

    // 5. Swipe 3
    let gen_3 = tree.add_swipe(Some(user_msg_1), "Aboard the starship Hyperion.".to_string()).unwrap();

    let view_nodes = tree.get_active_view_nodes();
    assert_eq!(view_nodes.len(), 3);
    assert_eq!(view_nodes[2].id, gen_3);
    assert_eq!(view_nodes[2].sibling_index, 2);
    assert_eq!(view_nodes[2].sibling_total, 3);
    assert!(view_nodes[2].can_swipe_left);
    assert!(!view_nodes[2].can_swipe_right);

    // Switch to Swipe 1
    assert!(tree.switch_branch(Some(user_msg_1), 0));
    let nodes_swipe_1 = tree.get_active_view_nodes();
    assert_eq!(nodes_swipe_1[2].id, gen_1);
    assert_eq!(nodes_swipe_1[2].content, "In the Whispering Glade.");
    assert_eq!(nodes_swipe_1[2].sibling_index, 0);
    assert!(!nodes_swipe_1[2].can_swipe_left);
    assert!(nodes_swipe_1[2].can_swipe_right);

    // Continue story from Swipe 1
    let user_msg_2 = tree.append_message(AuthorRole::User, "Tell me about this glade.".to_string(), Some(gen_1));
    let gen_1_reply = tree.append_message(AuthorRole::Assistant, "The trees whisper ancient spells here.".to_string(), Some(user_msg_2));

    let nodes_extended = tree.get_active_view_nodes();
    assert_eq!(nodes_extended.len(), 5);
    assert_eq!(nodes_extended.last().unwrap().id, gen_1_reply);

    // Switch back to Swipe 2 branch: should branch off at gen_2 without gen_1_reply
    assert!(tree.switch_branch(Some(user_msg_1), 1));
    let nodes_swipe_2 = tree.get_active_view_nodes();
    assert_eq!(nodes_swipe_2.len(), 3);
    assert_eq!(nodes_swipe_2[2].id, gen_2);
    assert_eq!(nodes_swipe_2[2].content, "In the heart of the Obsidian Citadel.");
}

#[test]
fn test_context_token_budgeting() {
    let mut char_data = CharacterData::default();
    char_data.name = "Chronos".to_string();
    char_data.description = "Keeper of the timelines.".to_string();

    let user = UserPersona {
        id: "u1".to_string(),
        name: "Traveler".to_string(),
        description: "Time anomaly seeker.".to_string(),
        avatar_data_url: None,
    };

    let mut tree = ChatTree::new("chronos".to_string(), "Token Test".to_string());
    let mut last_id = None;

    // Add 20 long turns
    for i in 0..20 {
        let u_id = tree.append_message(
            AuthorRole::User,
            format!("Turn {i}: Describe timeline fragment number {i} in great detail with many descriptors."),
            last_id,
        );
        let a_id = tree.append_message(
            AuthorRole::Assistant,
            format!("Turn {i}: Timeline fragment {i} is vibrating with cosmic energy, stars aligning in celestial geometry."),
            Some(u_id),
        );
        last_id = Some(a_id);
    }

    // Configure a small context window budget
    let config = PromptConfig {
        system_template: "You are {{char}}.".to_string(),
        max_context_tokens: 300,
        max_response_tokens: 100,
        include_examples: false,
    };

    let prompt_messages = build_chat_prompt(&char_data, &user, &tree, &config);

    // System prompt must always be preserved
    assert_eq!(prompt_messages[0].role, "system");
    // Latest message must be present
    assert!(prompt_messages.last().unwrap().content.contains("Turn 19"));

    // Total tokens of all prompt messages should be constrained
    let total_tokens: usize = prompt_messages.iter().map(|m| estimate_tokens(&m.content)).sum();
    assert!(total_tokens <= 350);
}
