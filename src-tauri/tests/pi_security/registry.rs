use serde::Deserialize;

/// A single test case from the payload registry.
#[derive(Debug, Deserialize)]
pub struct PayloadEntry {
    pub id: String,
    pub name: String,
    pub tier: u8,
    pub taxonomy: Taxonomy,
    pub injection_point: String,
    pub payload: String,
    /// Input text for the LLM (raw_text for cleanup, selected_text or voice_command context).
    /// When `null`, the payload itself is used as the input.
    pub input_text: Option<String>,
    pub detection: Detection,
}

#[derive(Debug, Deserialize)]
pub struct Taxonomy {
    pub intent: Option<String>,
    pub technique: Option<String>,
    pub evasion: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum Detection {
    SystemPromptLeaked {
        markers: Vec<String>,
    },
    BehaviorDeviation {
        forbidden_in_output: Vec<String>,
        #[allow(dead_code)]
        expected_behavior: Option<String>,
    },
    ExtraneousContent {
        forbidden_in_output: Vec<String>,
    },
    UnsafeOutput {
        patterns: Vec<String>,
    },
    LengthAnomaly {
        max_output_input_ratio: f64,
    },
}

/// Load all payload entries from the embedded JSON file.
pub fn load_all() -> Vec<PayloadEntry> {
    let json = include_str!("payloads.json");
    serde_json::from_str(json).expect("Failed to parse payloads.json")
}

/// Load a single payload entry by ID.
pub fn load(id: &str) -> PayloadEntry {
    load_all()
        .into_iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("Payload {id} not found in registry"))
}

/// Load all entries for a given tier.
pub fn load_tier(tier: u8) -> Vec<PayloadEntry> {
    load_all().into_iter().filter(|p| p.tier == tier).collect()
}
