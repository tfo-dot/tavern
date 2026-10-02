use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegexRule {
    pub id: String,
    pub name: String,
    pub pattern: String,
    pub replacement: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub case_insensitive: bool,
    #[serde(default = "default_true")]
    pub run_on_output: bool,
    #[serde(default)]
    pub run_on_input: bool,
    #[serde(default = "default_true")]
    pub run_on_display: bool,
}

impl RegexRule {
    pub fn new(
        name: impl Into<String>,
        pattern: impl Into<String>,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            pattern: pattern.into(),
            replacement: replacement.into(),
            enabled: true,
            case_insensitive: false,
            run_on_output: true,
            run_on_input: false,
            run_on_display: true,
        }
    }
}

pub fn default_regex_rules() -> Vec<RegexRule> {
    vec![
        RegexRule {
            id: "strip-char-prefix".to_string(),
            name: "Strip Character Prefix".to_string(),
            pattern: r"^(?:[A-Za-z0-9_]+:\s*)+".to_string(),
            replacement: "".to_string(),
            enabled: false,
            case_insensitive: false,
            run_on_output: true,
            run_on_input: false,
            run_on_display: true,
        },
        RegexRule {
            id: "format-ooc".to_string(),
            name: "Format OOC Brackets".to_string(),
            pattern: r"\((?:OOC|ooc):?\s*(.*?)\)".to_string(),
            replacement: r#"<span class="rp-ooc">($1)</span>"#.to_string(),
            enabled: true,
            case_insensitive: false,
            run_on_output: false,
            run_on_input: false,
            run_on_display: true,
        },
    ]
}

/// Applies matching regex rules to text in order.
pub fn apply_regex_rules<F>(text: &str, rules: &[RegexRule], predicate: F) -> String
where
    F: Fn(&RegexRule) -> bool,
{
    let mut result = text.to_string();
    for rule in rules {
        if rule.enabled && predicate(rule) && !rule.pattern.is_empty() {
            let pattern = if rule.case_insensitive {
                format!("(?i){}", rule.pattern)
            } else {
                rule.pattern.clone()
            };
            if let Ok(re) = regex::Regex::new(&pattern) {
                result = re
                    .replace_all(&result, rule.replacement.as_str())
                    .to_string();
            }
        }
    }
    result
}
