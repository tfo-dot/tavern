use engine::character::{Character, CharacterCardV1, CharacterCardV2, CharacterData, UserPersona};
use engine::chat::{AuthorRole, ChatTree};
use engine::lorebook::{
    parse_lorebook, scan_lorebooks_for_activation, Lorebook, LorebookEntry, LorebookPosition,
    SelectiveLogic,
};
use engine::parser::{export_character_json, export_character_png, parse_character_card};
use engine::prompt::{build_chat_prompt, build_chat_prompt_with_lorebooks, estimate_tokens, PromptConfig};
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

#[test]
fn test_lorebook_full_pipeline_activation_and_budgeting() {
    let mut char_data = CharacterData::default();
    char_data.name = "Lyra".to_string();
    char_data.description = "A celestial astronomer.".to_string();
    char_data.personality = "Quiet, insightful.".to_string();
    char_data.scenario = "Observing the lunar eclipse.".to_string();

    let user = UserPersona {
        id: "u1".to_string(),
        name: "Observer".to_string(),
        description: "An apprentice.".to_string(),
        avatar_data_url: None,
    };

    let mut tree = ChatTree::new("lyra".to_string(), "Eclipse".to_string());
    let m1 = tree.append_message(AuthorRole::Assistant, "The eclipse begins tonight.".to_string(), None);
    let m2 = tree.append_message(AuthorRole::User, "Do you have the telescope and obsidian lens ready?".to_string(), Some(m1));
    tree.append_message(AuthorRole::Assistant, "Yes, mounted on the north tower.".to_string(), Some(m2));

    // Create Lorebook
    let mut lorebook = Lorebook::new("Celestial Lore".to_string(), "Astronomy world info".to_string());
    lorebook.scan_depth = 3;
    lorebook.token_budget = 500;

    // Entry 1: Primary match
    lorebook.add_entry(LorebookEntry {
        keys: vec!["telescope".to_string()],
        content: "The Great Telescope of Aethelgard was forged from starmetal.".to_string(),
        comment: "Great Telescope".to_string(),
        position: LorebookPosition::BeforeChar,
        order: 10,
        ..Default::default()
    });

    // Entry 2: Selective match (requires "lens" AND "obsidian")
    lorebook.add_entry(LorebookEntry {
        keys: vec!["lens".to_string()],
        secondary_keys: vec!["obsidian".to_string()],
        selective: true,
        selective_logic: SelectiveLogic::AndAll,
        content: "The Obsidian Lens filters out corrupted void light.".to_string(),
        comment: "Obsidian Lens".to_string(),
        position: LorebookPosition::AfterChar,
        order: 20,
        ..Default::default()
    });

    // Entry 3: Unmatched entry
    lorebook.add_entry(LorebookEntry {
        keys: vec!["supernova".to_string()],
        content: "Supernovas occur when elder stars collapse.".to_string(),
        comment: "Supernova".to_string(),
        position: LorebookPosition::TopSystem,
        ..Default::default()
    });

    // Entry 4: Constant entry
    lorebook.add_entry(LorebookEntry {
        constant: true,
        content: "The astral calendar has twelve moon cycles.".to_string(),
        comment: "Astral Calendar".to_string(),
        position: LorebookPosition::TopSystem,
        order: 5,
        ..Default::default()
    });

    // Entry 5: AtDepth entry
    lorebook.add_entry(LorebookEntry {
        keys: vec!["tower".to_string()],
        content: "[System note: The north tower is exposed to high celestial winds.]".to_string(),
        comment: "North Tower Condition".to_string(),
        position: LorebookPosition::AtDepth,
        depth: 1,
        order: 30,
        ..Default::default()
    });

    let config = PromptConfig::default();
    let prompt_msgs = build_chat_prompt_with_lorebooks(&char_data, &user, &tree, &config, &[&lorebook]);

    // Top system entry should appear at the very start
    assert!(prompt_msgs[0].content.contains("The astral calendar has twelve moon cycles."));
    // BeforeChar entry should appear before character description
    assert!(prompt_msgs[0].content.contains("Great Telescope of Aethelgard"));
    // AfterChar entry should appear
    assert!(prompt_msgs[0].content.contains("The Obsidian Lens filters out corrupted void light."));
    // Unmatched entry should NOT be present
    assert!(!prompt_msgs[0].content.contains("Supernovas occur when elder stars collapse."));

    // AtDepth entry should be injected in the history
    let at_depth_found = prompt_msgs.iter().any(|m| m.content.contains("high celestial winds"));
    assert!(at_depth_found, "At-depth entry must be injected into chat history messages");
}

#[test]
fn test_version_vector_serialization() {
    use engine::crdt::TavernCrdtDoc;
    use loro::VersionVector;

    let doc = TavernCrdtDoc::new();
    let vv = doc.state_vector();
    let bytes = vv.encode();
    let decoded = VersionVector::decode(&bytes).unwrap();
    assert_eq!(vv, decoded);
}

#[test]
fn test_crdt_concurrent_edits_and_swipes_sync() {
    use engine::character::Character;
    use engine::chat::{AuthorRole, ChatTree};
    use engine::crdt::TavernCrdtDoc;
    use engine::lorebook::{Lorebook, LorebookEntry};

    let doc_a = TavernCrdtDoc::new();
    let doc_b = TavernCrdtDoc::new();

    // Create initial character and chat
    let mut char1 = Character::new("Elena".to_string(), "Hello!".to_string(), None);
    char1.id = "char_elena".to_string();
    doc_a.set_character(&char1).unwrap();

    let mut chat1 = ChatTree::new(char1.id.clone(), "Story Chapter 1".to_string());
    let m1 = chat1.append_message(AuthorRole::User, "Where are we?".to_string(), None);
    let m2_a = chat1.append_message(AuthorRole::Assistant, "In the enchanted forest.".to_string(), Some(m1));
    doc_a.set_chat(&chat1).unwrap();

    // Sync A -> B
    let vv_b = doc_b.state_vector();
    let delta_a_to_b = doc_a.export_updates_from(&vv_b).unwrap();
    doc_b.import_updates(&delta_a_to_b).unwrap();

    // Device A adds an alternate swipe generation under m1
    let mut chat_a = doc_a.get_chats().unwrap().into_iter().next().unwrap();
    let m2_swipe = chat_a.add_swipe(Some(m1), "Near the crystal lake.".to_string()).unwrap();
    doc_a.set_chat(&chat_a).unwrap();

    // Device B concurrently adds a lorebook entry
    let mut book = Lorebook::default();
    book.id = "world_lore".to_string();
    let mut entry = LorebookEntry::default();
    entry.id = "entry_1".to_string();
    entry.keys = vec!["crystal lake".to_string()];
    entry.content = "A mystical lake that reflects memories.".to_string();
    book.entries.push(entry);
    doc_b.set_lorebook(&book).unwrap();

    let vv_a_before = doc_a.state_vector();
    let vv_b_before = doc_b.state_vector();

    let delta_b_to_a = doc_b.export_updates_from(&vv_a_before).unwrap();
    let delta_a_to_b_2 = doc_a.export_updates_from(&vv_b_before).unwrap();

    doc_a.import_updates(&delta_b_to_a).unwrap();
    doc_b.import_updates(&delta_a_to_b_2).unwrap();

    // Verify convergence on both devices
    let chats_on_b = doc_b.get_chats().unwrap();
    assert_eq!(chats_on_b.len(), 1);
    assert_eq!(chats_on_b[0].nodes.len(), 3); // m1, m2_a, m2_swipe
    assert!(chats_on_b[0].nodes.contains_key(&m2_swipe));

    let books_on_a = doc_a.get_lorebooks().unwrap();
    assert_eq!(books_on_a.len(), 1);
    assert_eq!(books_on_a[0].entries.len(), 1);
    assert_eq!(books_on_a[0].entries[0].keys[0], "crystal lake");
}
