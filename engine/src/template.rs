use chrono::Local;

/// Interpolates standard SillyTavern / Character Card macros into a string.
pub fn interpolate_macros(
    text: &str,
    char_name: &str,
    user_name: &str,
    char_desc: &str,
    char_personality: &str,
    scenario: &str,
    user_persona: &str,
) -> String {
    if text.is_empty() {
        return String::new();
    }

    let now = Local::now();
    let time_str = now.format("%H:%M").to_string();
    let date_str = now.format("%Y-%m-%d").to_string();

    let mut result = text.to_string();

    // {{char}} variations
    result = result.replace("{{char}}", char_name);
    result = result.replace("{{Char}}", char_name);
    result = result.replace("{{CHAR}}", char_name);
    result = result.replace("<BOT>", char_name);
    result = result.replace("<bot>", char_name);

    // {{user}} variations
    result = result.replace("{{user}}", user_name);
    result = result.replace("{{User}}", user_name);
    result = result.replace("{{USER}}", user_name);
    result = result.replace("<USER>", user_name);
    result = result.replace("<user>", user_name);

    // Context macros
    result = result.replace("{{description}}", char_desc);
    result = result.replace("{{personality}}", char_personality);
    result = result.replace("{{scenario}}", scenario);
    result = result.replace("{{persona}}", user_persona);
    result = result.replace("{{time}}", &time_str);
    result = result.replace("{{date}}", &date_str);

    result
}
