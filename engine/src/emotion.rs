use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Emotion {
    #[default]
    Neutral,
    Joy,
    Blush,
    Anger,
    Sadness,
    Surprise,
}

impl Emotion {
    pub fn as_str(&self) -> &'static str {
        match self {
            Emotion::Neutral => "neutral",
            Emotion::Joy => "joy",
            Emotion::Blush => "blush",
            Emotion::Anger => "anger",
            Emotion::Sadness => "sadness",
            Emotion::Surprise => "surprise",
        }
    }

    pub fn from_name(s: &str) -> Self {
        match s.to_lowercase().trim() {
            "joy" | "happy" | "smile" | "laugh" => Emotion::Joy,
            "blush" | "flustered" | "shy" | "embarrassed" | "love" => Emotion::Blush,
            "anger" | "angry" | "annoyed" | "mad" | "furious" => Emotion::Anger,
            "sadness" | "sad" | "cry" | "grief" | "depressed" => Emotion::Sadness,
            "surprise" | "surprised" | "shock" | "gasp" => Emotion::Surprise,
            _ => Emotion::Neutral,
        }
    }
}

impl std::str::FromStr for Emotion {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_name(s))
    }
}

/// Classifies the dominant emotion of a message text using keyword and action regex analysis.
pub fn classify_emotion(text: &str) -> Emotion {
    let lower = text.to_lowercase();

    // 1. Explicit emotion tags: [joy], [blush], [anger], etc.
    if lower.contains("[joy]") || lower.contains("[happy]") {
        return Emotion::Joy;
    }
    if lower.contains("[blush]") || lower.contains("[shy]") {
        return Emotion::Blush;
    }
    if lower.contains("[anger]") || lower.contains("[angry]") {
        return Emotion::Anger;
    }
    if lower.contains("[sadness]") || lower.contains("[sad]") {
        return Emotion::Sadness;
    }
    if lower.contains("[surprise]") || lower.contains("[shock]") {
        return Emotion::Surprise;
    }

    // 2. Action and descriptive pattern matching
    let joy_words = [
        "smil", "grin", "laugh", "giggle", "chuckle", "beam", "cheer", "happy", "joy", "giggle",
    ];
    let blush_words = [
        "blush",
        "fluster",
        "shy",
        "embarrass",
        "bashful",
        "turn red",
        "turns red",
        "fidget",
    ];
    let anger_words = [
        "glar", "scowl", "frown", "growl", "snap", "angr", "furious", "grit", "yell", "shout",
        "mad",
    ];
    let sadness_words = [
        "cry", "cries", "weep", "tear", "sigh", "sniffle", "sad", "sob", "whimper", "mourn",
    ];
    let surprise_words = [
        "gasp", "startle", "shock", "widen", "widens", "stun", "astonish", "surpris",
    ];

    let count_matches =
        |keywords: &[&str]| -> usize { keywords.iter().filter(|&&kw| lower.contains(kw)).count() };

    let joy_score = count_matches(&joy_words);
    let blush_score = count_matches(&blush_words);
    let anger_score = count_matches(&anger_words);
    let sadness_score = count_matches(&sadness_words);
    let surprise_score = count_matches(&surprise_words);

    let max_score = joy_score
        .max(blush_score)
        .max(anger_score)
        .max(sadness_score)
        .max(surprise_score);

    if max_score == 0 {
        return Emotion::Neutral;
    }

    if blush_score == max_score {
        Emotion::Blush
    } else if joy_score == max_score {
        Emotion::Joy
    } else if anger_score == max_score {
        Emotion::Anger
    } else if sadness_score == max_score {
        Emotion::Sadness
    } else if surprise_score == max_score {
        Emotion::Surprise
    } else {
        Emotion::Neutral
    }
}
