use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum DimensionEffects {
    #[serde(rename = "minecraft:overworld")]
    Overworld,
    #[serde(rename = "minecraft:the_nether")]
    Nether,
    #[serde(rename = "minecraft:the_end")]
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedDistribution {
    data:   FixedOrDistribution,
    weight: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum IntegerDistribution {
    #[serde(rename = "minecraft:constant")]
    Constant(i32),
    #[serde(rename = "minecraft:uniform")]
    Uniform {
        min_inclusive: i32,
        max_inclusive: i32,
    },
    #[serde(rename = "minecraft:biased_to_bottom")]
    BiasedToBottom {
        min_inclusive: i32,
        max_inclusive: i32,
    },

    #[serde(rename = "minecraft:clamped")]
    Clamped {
        min_inclusive: i32,
        max_inclusive: i32,
        source:        Box<FixedOrDistribution>,
    },

    #[serde(rename = "minecraft:clamped_normal")]
    ClampedNormal {
        mean:      f32,
        deviation: f32,

        min_inclusive: i32,
        max_inclusive: i32,
    },

    #[serde(rename = "minecraft:weighted_list")]
    WeightedList {
        distribution: Vec<WeightedDistribution>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FixedOrDistribution {
    Fixed(i32),
    Distribution(IntegerDistribution),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(clippy::struct_excessive_bools)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct DimensionRegistry<'a> {
    pub fixed_time:       Option<i64>,
    pub coordinate_scale: f64,

    pub has_skylight: bool,
    pub has_ceiling:  bool,
    pub ultrawarm:    bool,

    pub natural:              bool,
    pub bed_works:            bool,
    pub respawn_anchor_works: bool,

    pub min_y:          i32,
    pub height:         i32,
    pub logical_height: i32,

    pub infiniburn: &'a str,
    pub effects:    DimensionEffects,

    pub ambient_light: f32,
    pub piglin_safe:   bool,
    pub has_raids:     bool,

    #[serde(rename = "monster_spawn_light_level")]
    pub monster_spawn_light:       FixedOrDistribution,
    #[serde(rename = "monster_spawn_block_light_limit")]
    pub monster_spawn_block_light: i32,
}
