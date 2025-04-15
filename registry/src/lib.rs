use std::fmt::Debug;

use serde::{
    Deserialize,
    Serialize,
};

pub mod biome;
pub mod damage;
pub mod dimension;

pub use biome::*;
pub use damage::*;
pub use dimension::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(bound(deserialize = "T: Deserialize<'de>, 'de: 'a"))]
pub struct RegistryEntry<'a, T: 'a + Debug + Clone> {
    pub name:    &'a str,
    pub id:      i32,
    pub element: T,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(bound(deserialize = "&'a [T]: Deserialize<'de>, 'de: 'a"))]
pub struct Registry<'a, T: 'a + Debug + Clone> {
    #[serde(rename = "type")]
    pub registry_type: &'a str,
    pub value:         &'a [T],
}
