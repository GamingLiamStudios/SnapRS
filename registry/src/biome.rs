use rgb::Rgb;
use serde::{
    Deserialize,
    Serialize,
};

mod color {
    use rgb::{
        ComponentSlice,
        Rgb,
    };
    use serde::{
        Deserializer,
        Serializer,
        de::Visitor,
    };

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S>(
        value: &Rgb<u8>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut bytes = [0u8; 4];
        bytes[1..].copy_from_slice(value.as_slice());
        serializer.serialize_i32(bytemuck::cast(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Rgb<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorVisitor;

        impl Visitor<'_> for ColorVisitor {
            type Value = Rgb<u8>;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str("Expecting i32 (Int tag)")
            }

            fn visit_i32<E>(
                self,
                v: i32,
            ) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let bytes: [u8; 4] = bytemuck::cast(v);
                Ok(*bytemuck::from_bytes(&bytes[1..]))
            }
        }

        deserializer.deserialize_i32(ColorVisitor)
    }
}

// Thank you https://github.com/serde-rs/serde/issues/1301
mod opt_color {
    use rgb::Rgb;
    use serde::{
        Deserialize,
        Deserializer,
        Serialize,
        Serializer,
    };

    #[allow(clippy::ref_option, clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S>(
        value: &Option<Rgb<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Helper<'a>(#[serde(with = "super::color")] &'a Rgb<u8>);

        value.as_ref().map(Helper).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Rgb<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper(#[serde(with = "super::color")] Rgb<u8>);

        let helper = Option::deserialize(deserializer)?;
        Ok(helper.map(|Helper(color)| color))
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", try_from = "&str")]
pub enum TemperatureModifier {
    #[default]
    None,
    Frozen,
}

impl<'a> TryFrom<&'a str> for TemperatureModifier {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "none" => Ok(Self::None),
            "frozen" => Ok(Self::Frozen),
            value => Err(value),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GrassColorModifier {
    #[default]
    None,
    DarkForest,
    Swamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct AmbientSound<'a> {
    pub sound_id: &'a str,
    pub range:    Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct MoodSound<'a> {
    pub sound:        &'a str,
    pub tick_delay:   i32,
    pub block_search: i32,
    pub offset:       f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, bound(deserialize = "'de: 'a"))]
pub enum AdditionsSound<'a> {
    Simple(&'a str),
    Extended {
        sound:       &'a str,
        tick_chance: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct Music<'a> {
    pub sound:           &'a str,
    pub min_delay:       i32,
    pub max_delay:       i32,
    pub replace_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct ParticleOptions<'a> {
    #[serde(rename = "type")]
    pub particle_id: &'a str,
    // TODO: Other particle metadata(probably requires enum repr)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct Particle<'a> {
    pub probability: f32,
    pub options:     ParticleOptions<'a>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct BiomeEffects<'a> {
    #[serde(with = "color")]
    pub fog_color: Rgb<u8>,
    #[serde(with = "color")]
    pub sky_color: Rgb<u8>,

    #[serde(with = "color")]
    pub water_color:     Rgb<u8>,
    #[serde(with = "color")]
    pub water_fog_color: Rgb<u8>,

    #[serde(with = "opt_color")]
    pub foliage_color:        Option<Rgb<u8>>,
    #[serde(with = "opt_color")]
    pub grass_color:          Option<Rgb<u8>>,
    #[serde(default)]
    pub grass_color_modifier: GrassColorModifier,

    pub particle: Option<Particle<'a>>,

    pub ambient_sound:   Option<AmbientSound<'a>>,
    pub mood_sound:      Option<MoodSound<'a>>,
    pub additions_sound: Option<AdditionsSound<'a>>,
    pub music:           Option<Music<'a>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct BiomeRegistry<'a> {
    pub has_precipitation: bool,
    pub temperature:       f32,

    #[serde(default)]
    pub temperature_modifier: TemperatureModifier,
    pub downfall:             f32,

    pub effects: BiomeEffects<'a>,
}
