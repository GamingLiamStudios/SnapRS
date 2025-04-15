use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum DifficultyScaling {
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "when_caused_by_living_non_player")]
    NonPlayer,
    #[serde(rename = "always")]
    Always,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DamageEffect {
    Hurt,
    Thorns,
    Drowning,
    Burning,
    Poking,
    Freezing,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathMessageType {
    Default,
    FallVariants,
    IntentionalGameDesign,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct DamageTypeRegistry<'a> {
    pub message_id:         &'a str,
    pub scaling:            DifficultyScaling,
    pub exhaustion:         f32,
    pub effects:            Option<DamageEffect>,
    pub death_message_type: Option<DeathMessageType>,
}
