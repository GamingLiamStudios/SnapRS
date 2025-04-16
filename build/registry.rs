use std::collections::HashMap;

use serde::Deserialize;

pub type Registies = HashMap<String, Registry>;

#[derive(Debug, Deserialize)]
pub struct Registry {
    pub default:     Option<String>,
    pub entries:     HashMap<String, RegistryEntry>,
    pub protocol_id: u16,
}

#[derive(Debug, Deserialize)]
pub struct RegistryEntry {
    pub protocol_id: u16,
}
