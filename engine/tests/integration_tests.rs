use engine::character::{CharacterCardV2, CharacterData, UserPersona};
use engine::chat::{
    AuthorRole, ChatTree, Group, TurnMode, export_sillytavern_chat_jsonl,
    import_sillytavern_chat_jsonl, resolve_next_speaker,
};
use engine::crdt::TavernCrdtDoc;
use engine::lorebook::{Lorebook, LorebookEntry, LorebookPosition, SelectiveLogic};
use engine::parser::{export_character_json, export_character_png, parse_character_card};
use engine::prompt::{
    PromptConfig, build_chat_prompt, build_chat_prompt_with_lorebooks,
    build_group_chat_prompt_with_lorebooks, estimate_tokens,
};
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
    let (reparsed_card, _) =
        parse_character_card(v2_json.as_bytes()).expect("Failed to reparse V2 JSON");
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
    let (parsed_card, avatar_url) =
        parse_character_card(&png_bytes).expect("Failed to parse PNG card");
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
    let root_id = tree.append_message(
        AuthorRole::Assistant,
        "Greetings traveler.".to_string(),
        None,
    );

    // 2. User Message
    let user_msg_1 =
        tree.append_message(AuthorRole::User, "Where are we?".to_string(), Some(root_id));

    // 3. Assistant Generation 1
    let gen_1 = tree.append_message(
        AuthorRole::Assistant,
        "In the Whispering Glade.".to_string(),
        Some(user_msg_1),
    );

    // 4. Swipe 2 (Add swipe to user_msg_1)
    let gen_2 = tree
        .add_swipe(
            Some(user_msg_1),
            "In the heart of the Obsidian Citadel.".to_string(),
        )
        .unwrap();

    // 5. Swipe 3
    let gen_3 = tree
        .add_swipe(
            Some(user_msg_1),
            "Aboard the starship Hyperion.".to_string(),
        )
        .unwrap();

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
    let user_msg_2 = tree.append_message(
        AuthorRole::User,
        "Tell me about this glade.".to_string(),
        Some(gen_1),
    );
    let gen_1_reply = tree.append_message(
        AuthorRole::Assistant,
        "The trees whisper ancient spells here.".to_string(),
        Some(user_msg_2),
    );

    let nodes_extended = tree.get_active_view_nodes();
    assert_eq!(nodes_extended.len(), 5);
    assert_eq!(nodes_extended.last().unwrap().id, gen_1_reply);

    // Switch back to Swipe 2 branch: should branch off at gen_2 without gen_1_reply
    assert!(tree.switch_branch(Some(user_msg_1), 1));
    let nodes_swipe_2 = tree.get_active_view_nodes();
    assert_eq!(nodes_swipe_2.len(), 3);
    assert_eq!(nodes_swipe_2[2].id, gen_2);
    assert_eq!(
        nodes_swipe_2[2].content,
        "In the heart of the Obsidian Citadel."
    );
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
        authors_note: None,
        authors_note_depth: None,
        authors_note_interval: None,
        vector_memory_enabled: true,
        vector_memory_top_k: 3,
        vector_memory_threshold: 0.25,
    };

    let prompt_messages = build_chat_prompt(&char_data, &user, &tree, &config);

    // System prompt must always be preserved
    assert_eq!(prompt_messages[0].role, "system");
    // Latest message must be present
    assert!(prompt_messages.last().unwrap().content.contains("Turn 19"));

    // Total tokens of all prompt messages should be constrained
    let total_tokens: usize = prompt_messages
        .iter()
        .map(|m| estimate_tokens(&m.content))
        .sum();
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
    let m1 = tree.append_message(
        AuthorRole::Assistant,
        "The eclipse begins tonight.".to_string(),
        None,
    );
    let m2 = tree.append_message(
        AuthorRole::User,
        "Do you have the telescope and obsidian lens ready?".to_string(),
        Some(m1),
    );
    tree.append_message(
        AuthorRole::Assistant,
        "Yes, mounted on the north tower.".to_string(),
        Some(m2),
    );

    // Create Lorebook
    let mut lorebook = Lorebook::new(
        "Celestial Lore".to_string(),
        "Astronomy world info".to_string(),
    );
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
    let prompt_msgs =
        build_chat_prompt_with_lorebooks(&char_data, &user, &tree, &config, &[&lorebook]);

    // Top system entry should appear at the very start
    assert!(
        prompt_msgs[0]
            .content
            .contains("The astral calendar has twelve moon cycles.")
    );
    // BeforeChar entry should appear before character description
    assert!(
        prompt_msgs[0]
            .content
            .contains("Great Telescope of Aethelgard")
    );
    // AfterChar entry should appear
    assert!(
        prompt_msgs[0]
            .content
            .contains("The Obsidian Lens filters out corrupted void light.")
    );
    // Unmatched entry should NOT be present
    assert!(
        !prompt_msgs[0]
            .content
            .contains("Supernovas occur when elder stars collapse.")
    );

    // AtDepth entry should be injected in the history
    let at_depth_found = prompt_msgs
        .iter()
        .any(|m| m.content.contains("high celestial winds"));
    assert!(
        at_depth_found,
        "At-depth entry must be injected into chat history messages"
    );
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
    let _m2_a = chat1.append_message(
        AuthorRole::Assistant,
        "In the enchanted forest.".to_string(),
        Some(m1),
    );
    doc_a.set_chat(&chat1).unwrap();

    // Sync A -> B
    let vv_b = doc_b.state_vector();
    let delta_a_to_b = doc_a.export_updates_from(&vv_b).unwrap();
    doc_b.import_updates(&delta_a_to_b).unwrap();

    // Device A adds an alternate swipe generation under m1
    let mut chat_a = doc_a.get_chats().unwrap().into_iter().next().unwrap();
    let m2_swipe = chat_a
        .add_swipe(Some(m1), "Near the crystal lake.".to_string())
        .unwrap();
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

#[test]
fn test_chat_tree_continuation_and_empty_user_turns() {
    let mut tree = ChatTree::new("char_valkyrie".to_string(), "Epic Saga".to_string());

    // 1. Initial Greeting (Assistant)
    let greeting_id = tree.append_message(
        AuthorRole::Assistant,
        "I am Brynhildr, shieldmaiden of the north.".to_string(),
        None,
    );

    // 2. Continue on the greeting (AI generation continuation without creating a new message node)
    assert!(
        tree.append_to_message(greeting_id, " What brings you to my realm, warrior?")
            .is_ok()
    );
    let view_nodes = tree.get_active_view_nodes();
    assert_eq!(view_nodes.len(), 1);
    assert_eq!(
        view_nodes[0].content,
        "I am Brynhildr, shieldmaiden of the north. What brings you to my realm, warrior?"
    );

    // 3. User sends empty turn / no additional text -> next Assistant turn generated under greeting
    let parent_id = tree.get_last_node_id();
    let a2_id = tree.append_message(
        AuthorRole::Assistant,
        "You stand silent before me. Speak, or draw your blade!".to_string(),
        parent_id,
    );
    let view_nodes2 = tree.get_active_view_nodes();
    assert_eq!(view_nodes2.len(), 2);
    assert_eq!(view_nodes2[0].role, AuthorRole::Assistant);
    assert_eq!(view_nodes2[1].role, AuthorRole::Assistant);
    assert_eq!(view_nodes2[1].id, a2_id);

    // 4. Continue on second assistant message
    assert!(
        tree.append_to_message(a2_id, " The cold winds wait for no one.")
            .is_ok()
    );
    let view_nodes3 = tree.get_active_view_nodes();
    assert_eq!(view_nodes3.len(), 2);
    assert_eq!(
        view_nodes3[1].content,
        "You stand silent before me. Speak, or draw your blade! The cold winds wait for no one."
    );
}

#[test]
fn test_prompt_assembly_continuation_and_consecutive_assistant_turns() {
    let mut data = CharacterData::default();
    data.name = "Brynhildr".to_string();
    data.description = "A fierce Norse shieldmaiden".to_string();
    data.post_history_instructions = "[Write in dramatic tone]".to_string();

    let user = UserPersona {
        id: "u1".to_string(),
        name: "Sigurd".to_string(),
        description: "A dragon slayer".to_string(),
        avatar_data_url: None,
    };

    let mut tree = ChatTree::new("brynhildr".to_string(), "Chat".to_string());
    let m1 = tree.append_message(AuthorRole::Assistant, "Hail, {{user}}.".to_string(), None);
    // Consecutive assistant message (generated without user text)
    let _m2 = tree.append_message(
        AuthorRole::Assistant,
        "Why do you wander these frozen peaks?".to_string(),
        Some(m1),
    );

    let config = PromptConfig::default();
    let prompt_messages = build_chat_prompt(&data, &user, &tree, &config);

    // Trailing message must be the last assistant message
    let last = prompt_messages.last().unwrap();
    assert_eq!(last.role, "assistant");
    assert_eq!(last.content, "Why do you wander these frozen peaks?");

    // The prompt contains interpolated character & user names
    let first_hist = prompt_messages
        .iter()
        .find(|m| m.content == "Hail, Sigurd.")
        .expect("greeting found");
    assert_eq!(first_hist.role, "assistant");
}

#[test]
fn test_sillytavern_chat_jsonl_full_conversation_roundtrip() {
    let st_jsonl = r#"{"user_name":"Morgan","character_name":"Vesper","create_date":"2026-09-01 @12h 00m 00s 000ms","chat_metadata":{"notes":"Test session"}}
{"name":"Vesper","is_user":false,"is_name":true,"send_date":"2026-09-01 @12h 00m 00s 000ms","mes":"The neon lights flicker in the rain. What are you looking for?","swipes":["The neon lights flicker in the rain. What are you looking for?","Looking for trouble in this city?"],"swipe_id":0}
{"name":"Morgan","is_user":true,"is_name":true,"send_date":"2026-09-01 @12h 01m 30s 500ms","mes":"I'm looking for the cybernetics dealer.","swipes":["I'm looking for the cybernetics dealer."],"swipe_id":0}
{"name":"Vesper","is_user":false,"is_name":true,"send_date":"2026-09-01 @12h 02m 10s 000ms","mes":"Lower your voice. Follow me down this alley.","swipes":["You're asking dangerous questions.","Lower your voice. Follow me down this alley."],"swipe_id":1}
{"name":"Morgan","is_user":true,"is_name":true,"send_date":"2026-09-01 @12h 03m 00s 000ms","mes":"Lead the way.","swipes":["Lead the way."],"swipe_id":0}"#;

    let tree = import_sillytavern_chat_jsonl(st_jsonl, "char_vesper", Some("Cyberpunk Alley"))
        .expect("Failed to import SillyTavern chat JSONL");

    assert_eq!(tree.character_id, "char_vesper");
    assert_eq!(tree.title, "Cyberpunk Alley");
    assert_eq!(tree.nodes.len(), 6); // 2 root swipes + 1 user + 2 assistant swipes + 1 user

    let active_nodes = tree.get_active_view_nodes();
    assert_eq!(active_nodes.len(), 4);
    assert_eq!(
        active_nodes[0].content,
        "The neon lights flicker in the rain. What are you looking for?"
    );
    assert_eq!(active_nodes[0].sibling_total, 2);
    assert_eq!(
        active_nodes[1].content,
        "I'm looking for the cybernetics dealer."
    );
    assert_eq!(
        active_nodes[2].content,
        "Lower your voice. Follow me down this alley."
    );
    assert_eq!(active_nodes[2].sibling_index, 1);
    assert_eq!(active_nodes[2].sibling_total, 2);
    assert_eq!(active_nodes[3].content, "Lead the way.");

    // Export back to JSONL
    let exported = export_sillytavern_chat_jsonl(&tree, "Morgan", "Vesper")
        .expect("Failed to export chat to JSONL");

    let exported_lines: Vec<&str> = exported.lines().collect();
    assert_eq!(exported_lines.len(), 5); // 1 header + 4 message turns

    // Parse lines to verify JSON schema compliance
    let header_val: serde_json::Value = serde_json::from_str(exported_lines[0]).unwrap();
    assert_eq!(header_val["user_name"], "Morgan");
    assert_eq!(header_val["character_name"], "Vesper");

    let turn1_val: serde_json::Value = serde_json::from_str(exported_lines[1]).unwrap();
    assert_eq!(turn1_val["is_user"], false);
    assert_eq!(turn1_val["swipes"].as_array().unwrap().len(), 2);
    assert_eq!(turn1_val["swipe_id"], 0);

    let turn3_val: serde_json::Value = serde_json::from_str(exported_lines[3]).unwrap();
    assert_eq!(turn3_val["is_user"], false);
    assert_eq!(
        turn3_val["mes"],
        "Lower your voice. Follow me down this alley."
    );
    assert_eq!(turn3_val["swipe_id"], 1);
}

#[test]
fn test_sillytavern_chat_crdt_sync_roundtrip() {
    let st_jsonl = r#"{"user_name":"Tester","character_name":"Oracle","create_date":"2026-09-01 @10h 00m 00s 000ms"}
{"name":"Oracle","is_user":false,"is_name":true,"send_date":"2026-09-01 @10h 00m 00s 000ms","mes":"I see all possibilities.","swipes":["I see all possibilities.","The stars reveal truth."],"swipe_id":0}
{"name":"Tester","is_user":true,"is_name":true,"send_date":"2026-09-01 @10h 01m 00s 000ms","mes":"What is my future?"}"#;

    let imported_chat = import_sillytavern_chat_jsonl(st_jsonl, "oracle_1", Some("Oracle Chat"))
        .expect("Import failed");

    // Device A stores imported chat in CRDT
    let doc_a = TavernCrdtDoc::new();
    doc_a.set_chat(&imported_chat).unwrap();

    // Device B syncs from Device A
    let doc_b = TavernCrdtDoc::new();
    let vv_b = doc_b.state_vector();
    let delta_a_to_b = doc_a.export_updates_from(&vv_b).unwrap();
    doc_b.import_updates(&delta_a_to_b).unwrap();

    let chats_b = doc_b.get_chats().unwrap();
    assert_eq!(chats_b.len(), 1);
    assert_eq!(chats_b[0].title, "Oracle Chat");

    // Export from Device B's synchronized ChatTree
    let exported_b = export_sillytavern_chat_jsonl(&chats_b[0], "Tester", "Oracle").unwrap();
    let reimported_on_c = import_sillytavern_chat_jsonl(&exported_b, "oracle_1", None).unwrap();
    assert_eq!(reimported_on_c.get_active_view_nodes().len(), 2);
}

#[test]
fn test_group_turn_mode_resolution() {
    let mut group = Group::new(
        "Adventuring Party".to_string(),
        vec![
            "char_a".to_string(),
            "char_b".to_string(),
            "char_c".to_string(),
        ],
    );

    // 1. Natural / Round-Robin mode
    group.turn_mode = TurnMode::Natural;
    // No prior speaker -> first member
    assert_eq!(
        resolve_next_speaker(&group, None),
        Some("char_a".to_string())
    );
    // After char_a -> char_b
    assert_eq!(
        resolve_next_speaker(&group, Some("char_a")),
        Some("char_b".to_string())
    );
    // After char_b -> char_c
    assert_eq!(
        resolve_next_speaker(&group, Some("char_b")),
        Some("char_c".to_string())
    );
    // After char_c -> wrap around to char_a
    assert_eq!(
        resolve_next_speaker(&group, Some("char_c")),
        Some("char_a".to_string())
    );

    // 2. Muting / Disabling a member
    group.members[1].enabled = false; // Disable char_b
    assert_eq!(
        resolve_next_speaker(&group, Some("char_a")),
        Some("char_c".to_string())
    );
    assert_eq!(
        resolve_next_speaker(&group, Some("char_c")),
        Some("char_a".to_string())
    );

    group.members[1].enabled = true;
    group.members[1].mute = true; // Mute char_b
    assert_eq!(
        resolve_next_speaker(&group, Some("char_a")),
        Some("char_c".to_string())
    );

    // 3. Random mode without self-responses
    group.members[1].mute = false;
    group.turn_mode = TurnMode::Random;
    group.allow_self_responses = false;
    let next = resolve_next_speaker(&group, Some("char_a")).unwrap();
    assert!(next == "char_b" || next == "char_c");
}

#[test]
fn test_group_chat_tree_and_speaker_attribution() {
    let mut chat = ChatTree::new_group("group_123".to_string(), "Tavern Gathering".to_string());
    assert_eq!(chat.group_id, Some("group_123".to_string()));
    assert_eq!(chat.character_id, "group:group_123");

    // Turn 1: Char A speaks
    let turn1 = chat.append_message_with_author(
        AuthorRole::Assistant,
        "Greetings travelers, what brings you here?".to_string(),
        None,
        Some("char_a".to_string()),
        Some("Seraphina".to_string()),
    );

    // Turn 2: User replies
    let turn2 = chat.append_message(
        AuthorRole::User,
        "We seek shelter from the storm.".to_string(),
        Some(turn1),
    );

    // Turn 3: Char B speaks
    let _turn3 = chat.append_message_with_author(
        AuthorRole::Assistant,
        "There's room by the fireplace.".to_string(),
        Some(turn2),
        Some("char_b".to_string()),
        Some("Garrick".to_string()),
    );

    // Add swipe for Char B
    let swipe_b2 = chat.add_swipe_with_author(
        Some(turn2),
        "Pull up a bench and dry your cloaks.".to_string(),
        Some("char_b".to_string()),
        Some("Garrick".to_string()),
    );
    assert!(swipe_b2.is_some());

    let active_nodes = chat.get_active_view_nodes();
    assert_eq!(active_nodes.len(), 3);
    assert_eq!(active_nodes[0].character_id, Some("char_a".to_string()));
    assert_eq!(active_nodes[0].name, Some("Seraphina".to_string()));
    assert_eq!(active_nodes[1].role, AuthorRole::User);
    assert_eq!(active_nodes[2].character_id, Some("char_b".to_string()));
    assert_eq!(active_nodes[2].name, Some("Garrick".to_string()));
    assert_eq!(active_nodes[2].sibling_total, 2);
}

#[test]
fn test_group_chat_prompt_assembly_with_multi_character_context() {
    let mut char_a_data = CharacterData::default();
    char_a_data.name = "Seraphina".to_string();
    char_a_data.description = "Warm tavern keeper with arcane knowledge.".to_string();
    char_a_data.personality = "Empathetic and calm.".to_string();

    let mut char_b_data = CharacterData::default();
    char_b_data.name = "Garrick".to_string();
    char_b_data.description = "A gruff veteran warrior.".to_string();
    char_b_data.personality = "Blunt and protective.".to_string();

    let user = UserPersona {
        id: "u1".to_string(),
        name: "Robin".to_string(),
        description: "A wandering bard.".to_string(),
        avatar_data_url: None,
    };

    let mut chat = ChatTree::new_group("group_1".to_string(), "Group Roleplay".to_string());
    let m1 = chat.append_message(AuthorRole::User, "Hello everyone!".to_string(), None);
    let m2 = chat.append_message_with_author(
        AuthorRole::Assistant,
        "Welcome Robin! Take a seat.".to_string(),
        Some(m1),
        Some("char_a".to_string()),
        Some("Seraphina".to_string()),
    );
    let _m3 = chat.append_message_with_author(
        AuthorRole::Assistant,
        "Keep your hands off the weapons rack.".to_string(),
        Some(m2),
        Some("char_b".to_string()),
        Some("Garrick".to_string()),
    );

    let config = PromptConfig::default();
    let other_chars = [(&char_b_data, "char_b")];

    // Assemble prompt for Seraphina's turn
    let prompt = build_group_chat_prompt_with_lorebooks(
        &char_a_data,
        "char_a",
        &other_chars,
        &user,
        &chat,
        &config,
        &[],
    );

    // System prompt verification
    let sys_msg = &prompt[0];
    assert_eq!(sys_msg.role, "system");
    assert!(sys_msg.content.contains("[Group Roleplay Context]"));
    assert!(sys_msg.content.contains("roleplaying ONLY as Seraphina"));
    assert!(
        sys_msg
            .content
            .contains("[Other Participants in this Conversation]")
    );
    assert!(sys_msg.content.contains("Garrick"));

    // History messages verification
    // Turn 1 (User): role = "user", content = "Robin: Hello everyone!"
    assert_eq!(prompt[1].role, "user");
    assert_eq!(prompt[1].content, "Robin: Hello everyone!");

    // Turn 2 (Seraphina's own prior message): role = "assistant"
    assert_eq!(prompt[2].role, "assistant");
    assert_eq!(prompt[2].content, "Welcome Robin! Take a seat.");

    // Turn 3 (Garrick's message from Seraphina's perspective): role = "user", content contains Garrick's name
    assert_eq!(prompt[3].role, "user");
    assert_eq!(
        prompt[3].content,
        "Garrick: Keep your hands off the weapons rack."
    );
}

#[test]
fn test_group_crdt_sync_roundtrip() {
    let group = Group::new(
        "Midnight Council".to_string(),
        vec!["char_1".to_string(), "char_2".to_string()],
    );

    let doc_a = TavernCrdtDoc::new();
    doc_a.set_group(&group).unwrap();

    let doc_b = TavernCrdtDoc::new();
    let vv_b = doc_b.state_vector();
    let delta = doc_a.export_updates_from(&vv_b).unwrap();
    doc_b.import_updates(&delta).unwrap();

    let groups_b = doc_b.get_groups().unwrap();
    assert_eq!(groups_b.len(), 1);
    assert_eq!(groups_b[0].name, "Midnight Council");
    assert_eq!(groups_b[0].members.len(), 2);
}

#[test]
fn test_group_chat_sillytavern_jsonl_roundtrip() {
    let mut chat = ChatTree::new_group("grp_heroes".to_string(), "The Quest Begins".to_string());

    let t1 = chat.append_message_with_author(
        AuthorRole::Assistant,
        "The dungeon entrance looms ahead.".to_string(),
        None,
        Some("char_guide".to_string()),
        Some("Eldrin".to_string()),
    );

    let t2 = chat.append_message(
        AuthorRole::User,
        "I light a torch and lead the way.".to_string(),
        Some(t1),
    );

    let _t3 = chat.append_message_with_author(
        AuthorRole::Assistant,
        "Careful, check for traps on the floor.".to_string(),
        Some(t2),
        Some("char_rogue".to_string()),
        Some("Kaelen".to_string()),
    );

    let exported = export_sillytavern_chat_jsonl(&chat, "Adventurer", "Party").unwrap();
    let lines: Vec<&str> = exported.lines().collect();
    assert_eq!(lines.len(), 4); // 1 header + 3 messages

    let msg1: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(msg1["name"], "Eldrin");
    assert_eq!(msg1["character_id"], "char_guide");

    let msg2: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(msg2["name"], "Adventurer");
    assert_eq!(msg2["is_user"], true);

    let msg3: serde_json::Value = serde_json::from_str(lines[3]).unwrap();
    assert_eq!(msg3["name"], "Kaelen");
    assert_eq!(msg3["character_id"], "char_rogue");

    // Re-import
    let imported =
        import_sillytavern_chat_jsonl(&exported, "group:grp_heroes", Some("The Quest Begins"))
            .unwrap();
    let view_nodes = imported.get_active_view_nodes();
    assert_eq!(view_nodes.len(), 3);
    assert_eq!(view_nodes[0].name, Some("Eldrin".to_string()));
    assert_eq!(view_nodes[0].character_id, Some("char_guide".to_string()));
    assert_eq!(view_nodes[2].name, Some("Kaelen".to_string()));
    assert_eq!(view_nodes[2].character_id, Some("char_rogue".to_string()));
}

#[test]
fn test_group_chat_lorebook_activation_across_members() {
    let mut entry1 = LorebookEntry::default();
    entry1.keys = vec!["eldrin".to_string(), "starlight".to_string()];
    entry1.content = "The Starlight Order is an ancient guild of mages.".to_string();
    entry1.position = LorebookPosition::BeforeChar;

    let mut entry2 = LorebookEntry::default();
    entry2.keys = vec!["torch".to_string(), "dungeon".to_string()];
    entry2.content = "Dungeon shadows conceal ancient stone traps.".to_string();
    entry2.position = LorebookPosition::AfterChar;

    let lorebook1 = Lorebook {
        id: "lb1".to_string(),
        name: "Mage Lore".to_string(),
        entries: vec![entry1],
        ..Default::default()
    };

    let lorebook2 = Lorebook {
        id: "lb2".to_string(),
        name: "Dungeon Lore".to_string(),
        entries: vec![entry2],
        ..Default::default()
    };
    let mut char_data = CharacterData::default();
    char_data.name = "Eldrin".to_string();

    let mut other_data = CharacterData::default();
    other_data.name = "Kaelen".to_string();

    let user = UserPersona::default();

    let mut chat = ChatTree::new_group("g1".to_string(), "Quest".to_string());
    chat.append_message(
        AuthorRole::User,
        "I light a torch near the dungeon entrance.".to_string(),
        None,
    );

    let config = PromptConfig::default();
    let prompt = build_group_chat_prompt_with_lorebooks(
        &char_data,
        "c1",
        &[(&other_data, "c2")],
        &user,
        &chat,
        &config,
        &[&lorebook1, &lorebook2],
    );

    let sys_content = &prompt[0].content;
    // entry 2 matched "torch" and "dungeon"
    assert!(sys_content.contains("Dungeon shadows conceal ancient stone traps."));
}

#[test]
fn test_authors_note_injection_at_depth() {
    let mut char_data = CharacterData::default();
    char_data.name = "Aria".to_string();
    char_data.description = "A gentle companion.".to_string();
    let user = UserPersona::default();

    let mut tree = ChatTree::new("aria".to_string(), "Test A/N Depth".to_string());
    let m1 = tree.append_message(AuthorRole::Assistant, "Hello there!".to_string(), None);
    let m2 = tree.append_message(AuthorRole::User, "Good day Aria.".to_string(), Some(m1));
    let _m3 = tree.append_message(
        AuthorRole::Assistant,
        "The sky is blue today.".to_string(),
        Some(m2),
    );

    // Test Depth 0: Inserted at the end of history
    tree.authors_note = "Stay cheerful and friendly.".to_string();
    tree.authors_note_depth = 0;
    tree.authors_note_interval = 1;

    let config = PromptConfig::default();
    let prompt_d0 = build_chat_prompt_with_lorebooks(&char_data, &user, &tree, &config, &[]);
    // Last message should be the Author's Note
    assert!(
        prompt_d0
            .last()
            .unwrap()
            .content
            .contains("Stay cheerful and friendly.")
    );
    assert_eq!(prompt_d0.last().unwrap().role, "system");

    // Test Depth 1: Inserted 1 message from the bottom
    tree.authors_note_depth = 1;
    let prompt_d1 = build_chat_prompt_with_lorebooks(&char_data, &user, &tree, &config, &[]);
    let len = prompt_d1.len();
    assert!(
        prompt_d1[len - 2]
            .content
            .contains("Stay cheerful and friendly.")
    );
    assert_eq!(prompt_d1[len - 2].role, "system");
    assert!(
        prompt_d1[len - 1]
            .content
            .contains("The sky is blue today.")
    );

    // Test Depth 2: Inserted 2 messages from the bottom
    tree.authors_note_depth = 2;
    let prompt_d2 = build_chat_prompt_with_lorebooks(&char_data, &user, &tree, &config, &[]);
    let len2 = prompt_d2.len();
    assert!(
        prompt_d2[len2 - 3]
            .content
            .contains("Stay cheerful and friendly.")
    );
    assert_eq!(prompt_d2[len2 - 3].role, "system");
}

#[test]
fn test_authors_note_update_interval() {
    let char_data = CharacterData::default();
    let user = UserPersona::default();

    let mut tree = ChatTree::new("c1".to_string(), "Interval Test".to_string());
    let m1 = tree.append_message(AuthorRole::Assistant, "Turn 1".to_string(), None);
    let m2 = tree.append_message(AuthorRole::User, "Turn 2".to_string(), Some(m1));
    let _m3 = tree.append_message(AuthorRole::Assistant, "Turn 3".to_string(), Some(m2));

    tree.authors_note = "Inject on even turns only.".to_string();
    tree.authors_note_depth = 1;
    tree.authors_note_interval = 2;

    let config = PromptConfig::default();
    // Path length is 3 (odd). With interval 2, it should NOT be injected.
    let prompt_odd = build_chat_prompt_with_lorebooks(&char_data, &user, &tree, &config, &[]);
    assert!(
        !prompt_odd
            .iter()
            .any(|m| m.content.contains("Inject on even turns only."))
    );

    // Add another message to make path length 4 (even).
    tree.append_message(AuthorRole::User, "Turn 4".to_string(), Some(_m3));
    let prompt_even = build_chat_prompt_with_lorebooks(&char_data, &user, &tree, &config, &[]);
    assert!(
        prompt_even
            .iter()
            .any(|m| m.content.contains("Inject on even turns only."))
    );
}

#[test]
fn test_exact_token_counting_and_context_breakdown() {
    use engine::prompt::{calculate_context_breakdown, count_tokens};

    let sample_text = "The quick brown fox jumps over the lazy dog.";
    let tokens = count_tokens(sample_text);
    assert_eq!(tokens, 10);

    let mut char_data = CharacterData::default();
    char_data.name = "Luna".to_string();
    char_data.description = "A nocturnal spirit guardian of the midnight grove.".to_string();
    char_data.personality = "Calm, watchful, mysterious.".to_string();
    char_data.scenario = "The grove is bathed in silver moonlight.".to_string();
    char_data.mes_example =
        "<START>\n{{user}}: Who are you?\n{{char}}: I am the guardian of this sacred grove."
            .to_string();

    let mut user = UserPersona::default();
    user.name = "Traveler".to_string();
    user.description = "A lost wanderer carrying an antique brass lantern.".to_string();

    let mut tree = ChatTree::new("luna".to_string(), "Midnight Grove".to_string());
    let m1 = tree.append_message(
        AuthorRole::Assistant,
        "Welcome to the grove, Traveler.".to_string(),
        None,
    );
    tree.append_message(
        AuthorRole::User,
        "The shadows feel alive here.".to_string(),
        Some(m1),
    );

    tree.authors_note = "Describe ambient forest sounds and rustling leaves.".to_string();
    tree.authors_note_depth = 1;
    tree.authors_note_interval = 1;

    let config = PromptConfig {
        system_template: "You are {{char}} in a dialogue with {{user}}.".to_string(),
        max_context_tokens: 2048,
        max_response_tokens: 500,
        include_examples: true,
        authors_note: None,
        authors_note_depth: None,
        authors_note_interval: None,
        vector_memory_enabled: true,
        vector_memory_top_k: 3,
        vector_memory_threshold: 0.25,
    };

    let breakdown = calculate_context_breakdown(
        &char_data,
        &user,
        &tree,
        &config,
        &[],
        Some("Is there a safe path forward?"),
    );

    assert!(breakdown.system_tokens > 0);
    assert!(breakdown.character_tokens > 0);
    assert!(breakdown.user_persona_tokens > 0);
    assert!(breakdown.authors_note_tokens > 0);
    assert!(breakdown.examples_tokens > 0);
    assert!(breakdown.history_tokens > 0);
    assert!(breakdown.draft_tokens > 0);
    assert_eq!(breakdown.response_tokens, 500);
    assert_eq!(breakdown.max_context_tokens, 2048);
    assert!(breakdown.total_tokens > 0);
    assert!(breakdown.free_tokens > 0);
    assert!(breakdown.percentage > 0.0 && breakdown.percentage < 100.0);
}

#[test]
fn test_generation_params_and_api_type_detection() {
    use engine::llm::{
        ApiType, GenerationParams, normalize_endpoint_for_type, normalize_models_endpoint_for_type,
    };

    let default_params = GenerationParams::default();
    assert_eq!(default_params.min_p, 0.0);
    assert_eq!(default_params.top_k, 40);
    assert_eq!(default_params.repetition_penalty, 1.05);

    // ApiType Detection
    assert_eq!(
        ApiType::detect("https://api.anthropic.com"),
        ApiType::Anthropic
    );
    assert_eq!(
        ApiType::detect("https://api.anthropic.com/v1/messages"),
        ApiType::Anthropic
    );
    assert_eq!(ApiType::detect("http://localhost:5001"), ApiType::KoboldCpp);
    assert_eq!(
        ApiType::detect("http://localhost:5001/api/v1/generate"),
        ApiType::KoboldCpp
    );
    assert_eq!(
        ApiType::detect("http://localhost:11434/v1"),
        ApiType::OpenAi
    );
    assert_eq!(
        ApiType::detect("https://openrouter.ai/api/v1"),
        ApiType::OpenAi
    );

    // Endpoint Normalization
    assert_eq!(
        normalize_endpoint_for_type("https://api.anthropic.com", ApiType::Anthropic),
        "https://api.anthropic.com/v1/messages"
    );
    assert_eq!(
        normalize_endpoint_for_type("http://localhost:5001", ApiType::KoboldCpp),
        "http://localhost:5001/api/extra/generate/stream"
    );
    assert_eq!(
        normalize_endpoint_for_type("http://localhost:11434/v1", ApiType::OpenAi),
        "http://localhost:11434/v1/chat/completions"
    );

    // Models Endpoint Normalization
    assert_eq!(
        normalize_models_endpoint_for_type("https://api.anthropic.com", ApiType::Anthropic),
        "https://api.anthropic.com/v1/models"
    );
    assert_eq!(
        normalize_models_endpoint_for_type("http://localhost:5001", ApiType::KoboldCpp),
        "http://localhost:5001/api/v1/model"
    );
}

#[test]
fn test_reasoning_streamer_wrapping() {
    use engine::llm::ReasoningStreamer;

    let mut streamed_tokens = Vec::new();
    {
        let mut streamer = ReasoningStreamer::new(|tok| {
            streamed_tokens.push(tok);
        });

        // Stream reasoning
        streamer.push_reasoning("Analyzing situation... ");
        streamer.push_reasoning("The best response is to smile.");
        // Stream regular content
        streamer.push_content("Hello! ");
        streamer.push_content("How are you today?");
        let final_text = streamer.finish();

        assert!(final_text.starts_with("<think>\nAnalyzing situation... The best response is to smile.\n</think>\n\nHello! How are you today?"));
    }

    // Stream finishes while still in reasoning
    let mut streamed_tokens2 = Vec::new();
    {
        let mut streamer = ReasoningStreamer::new(|tok| {
            streamed_tokens2.push(tok);
        });
        streamer.push_reasoning("Just thinking deep thoughts");
        let final_text = streamer.finish();
        assert!(final_text.ends_with("</think>\n\n"));
    }
}

#[test]
fn test_chat_fork_at_message() {
    let mut tree = ChatTree::new("char_1".to_string(), "Original Campaign".to_string());
    let m1 = tree.append_message(AuthorRole::Assistant, "Prologue".to_string(), None);
    let m2 = tree.append_message(AuthorRole::User, "Step forward".to_string(), Some(m1));
    let m3 = tree.append_message(
        AuthorRole::Assistant,
        "Encounter a crossroad".to_string(),
        Some(m2),
    );
    let _m4 = tree.append_message(AuthorRole::User, "Take the dark path".to_string(), Some(m3));

    // Fork at message 3 (crossroad)
    let forked = tree
        .fork_at_message(m3, Some("Light Path Campaign".to_string()))
        .expect("fork should succeed");

    assert_eq!(forked.title, "Light Path Campaign");
    assert_eq!(forked.character_id, "char_1");
    let active_nodes = forked.get_active_view_nodes();
    assert_eq!(active_nodes.len(), 3);
    assert_eq!(active_nodes[0].content, "Prologue");
    assert_eq!(active_nodes[1].content, "Step forward");
    assert_eq!(active_nodes[2].content, "Encounter a crossroad");

    // Now in the forked tree, append a different choice
    let forked_last_id = forked.get_last_node_id();
    let mut forked_mut = forked;
    forked_mut.append_message(
        AuthorRole::User,
        "Take the bright sunlit path".to_string(),
        forked_last_id,
    );
    assert_eq!(forked_mut.get_active_view_nodes().len(), 4);
    assert_eq!(tree.get_active_view_nodes().len(), 4);
    assert!(
        tree.get_active_view_nodes()[3]
            .content
            .contains("Take the dark path")
    );
    assert!(
        forked_mut.get_active_view_nodes()[3]
            .content
            .contains("Take the bright sunlit path")
    );
}

#[test]
fn test_regex_rules_application() {
    use engine::regex_engine::{RegexRule, apply_regex_rules};

    let rules = vec![
        RegexRule {
            id: "r1".to_string(),
            name: "Strip Prefix".to_string(),
            pattern: r"^Character:\s*".to_string(),
            replacement: "".to_string(),
            enabled: true,
            case_insensitive: true,
            run_on_output: true,
            run_on_input: false,
            run_on_display: true,
        },
        RegexRule {
            id: "r2".to_string(),
            name: "Format OOC".to_string(),
            pattern: r"\((?:OOC|ooc):?\s*(.*?)\)".to_string(),
            replacement: r#"<span class="rp-ooc">($1)</span>"#.to_string(),
            enabled: true,
            case_insensitive: false,
            run_on_output: false,
            run_on_input: false,
            run_on_display: true,
        },
    ];

    // Test output rule
    let output_text = "character: Hello adventurer!";
    let processed_output = apply_regex_rules(output_text, &rules, |r| r.run_on_output);
    assert_eq!(processed_output, "Hello adventurer!");

    // Test display rule
    let display_text = "Hello! (OOC: test note)";
    let processed_display = apply_regex_rules(display_text, &rules, |r| r.run_on_display);
    assert_eq!(
        processed_display,
        r#"Hello! <span class="rp-ooc">(test note)</span>"#
    );
}

#[test]
fn test_emotion_classification() {
    use engine::emotion::{Emotion, classify_emotion};

    assert_eq!(
        classify_emotion("*smiles warmly at you* Hello!"),
        Emotion::Joy
    );
    assert_eq!(
        classify_emotion("*giggles happily* That was funny!"),
        Emotion::Joy
    );
    assert_eq!(
        classify_emotion("*blushes deeply and looks away* I-it's not like that..."),
        Emotion::Blush
    );
    assert_eq!(
        classify_emotion("*fidgets nervously, face turning red*"),
        Emotion::Blush
    );
    assert_eq!(
        classify_emotion("*glares furiously with clenched fists* Back off!"),
        Emotion::Anger
    );
    assert_eq!(
        classify_emotion("*scowls and growls angrily*"),
        Emotion::Anger
    );
    assert_eq!(
        classify_emotion("*weeps softly with tears running down her cheeks*"),
        Emotion::Sadness
    );
    assert_eq!(
        classify_emotion("*gasps in utter shock, eyes widening*"),
        Emotion::Surprise
    );
    assert_eq!(
        classify_emotion("The weather is overcast today with a gentle breeze."),
        Emotion::Neutral
    );

    // Test explicit emotion tags
    assert_eq!(classify_emotion("[joy] I am so thrilled!"), Emotion::Joy);
    assert_eq!(classify_emotion("[anger] How dare you!"), Emotion::Anger);
    assert_eq!(classify_emotion("[blush] Thank you..."), Emotion::Blush);
}

#[test]
fn test_rag_memory_indexing_and_retrieval() {
    use engine::rag::{
        MemoryChunk, compute_embedding, cosine_similarity, format_retrieved_memories,
        retrieve_relevant_memories,
    };

    let v1 = compute_embedding(
        "The ancient obsidian key is hidden under the floorboards in the dusty attic.",
    );
    let v2 = compute_embedding("Where can we find the obsidian key?");
    let v3 = compute_embedding("I love baking fresh strawberry cakes on Sunday mornings.");

    let sim_relevant = cosine_similarity(&v1, &v2);
    let sim_irrelevant = cosine_similarity(&v2, &v3);

    assert!(sim_relevant > sim_irrelevant);
    assert!(sim_relevant > 0.25);

    let candidates = vec![
        MemoryChunk {
            message_id: uuid::Uuid::new_v4(),
            role: "assistant".to_string(),
            author: "Aria".to_string(),
            content: "We hid the obsidian key under the loose floorboards in the attic."
                .to_string(),
            turn_index: 42,
        },
        MemoryChunk {
            message_id: uuid::Uuid::new_v4(),
            role: "assistant".to_string(),
            author: "Aria".to_string(),
            content: "The weather yesterday was sunny and clear.".to_string(),
            turn_index: 105,
        },
        MemoryChunk {
            message_id: uuid::Uuid::new_v4(),
            role: "user".to_string(),
            author: "Traveler".to_string(),
            content: "I remember we discussed baking sweet strawberry pies.".to_string(),
            turn_index: 210,
        },
    ];

    let retrieved = retrieve_relevant_memories("Where is that obsidian key?", &candidates, 2, 0.20);
    assert!(!retrieved.is_empty());
    assert_eq!(retrieved[0].chunk.turn_index, 42);
    assert!(retrieved[0].chunk.content.contains("obsidian key"));

    let formatted = format_retrieved_memories(&retrieved);
    assert!(formatted.contains("[Relevant Past Memories"));
    assert!(formatted.contains("Turn 43"));
}
