use std::{
    collections::{
        HashMap,
        HashSet,
    },
    error::Error,
    fs::File,
    io::BufReader,
};

use convert_case::{
    Case,
    Casing,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BlockState {
    default:    Option<bool>,
    id:         u16,
    properties: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct Block {
    properties: Option<HashMap<String, HashSet<String>>>,
    states:     Vec<BlockState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Boolean,
    Axis,
    Cooldown,

    Compare,
    Structure,
    Piston,
    Chest,
    Noteblock,
    TrialSpawner,
    Redstone,
    Bell,
    Bed,
    Dripstone,
    Bamboo,

    Height,
    Direct,
    Orient,
    Face,
    Tilt,

    StairShape,
    RailShape,

    Half,
    Side,

    Integer(u8),
    Bitflags(u8),
}

impl Type {
    #[must_use]
    pub const fn to_string(self) -> &'static str {
        match self {
            Self::Integer(_) | Self::Bitflags(_) => "u8",
            Self::Boolean => "bool",
            Self::Axis => "Axis",
            Self::Cooldown => "SkulkPhase",
            Self::Compare => "ComparitorMode",
            Self::Structure => "StructureBlockMode",
            Self::Piston => "PistonType",
            Self::Chest => "ChestType",
            Self::Noteblock => "NoteblockInstrument",
            Self::TrialSpawner => "TrialSpawnerState",
            Self::Redstone => "RedstoneOrientation",
            Self::Bell => "BellAttachment",
            Self::Bed => "BedPart",
            Self::Dripstone => "DripstoneThickness",
            Self::Bamboo => "BambooLeafSize",
            Self::Height => "WallHeight",
            Self::Direct => "BlockDirection",
            Self::Orient => "BlockOrientation",
            Self::Tilt => "BlockTilt",
            Self::Face => "BlockFace",
            Self::StairShape => "StairShape",
            Self::RailShape => "RailShape",
            Self::Half => "BlockHalf",
            Self::Side => "HingeSide",
        }
    }
}

#[derive(Debug)]
pub struct ParsedBlock {
    pub name:       String,
    pub properties: Vec<(String, Type)>,
    pub states:     Vec<(u16, Vec<(String, String)>)>,
    pub default:    u16,
}

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub fn parse_blocks() -> Result<Vec<ParsedBlock>, Box<dyn Error>> {
    println!("cargo::rerun-if-changed=generated/reports/blocks.json");
    let reader = BufReader::new(File::open("generated/reports/blocks.json")?);
    let blocks: HashMap<String, Block> = serde_json::from_reader(reader)?;

    println!("cargo::warning=Found {} blocks", blocks.len());
    let mut parsed_blocks = Vec::new();
    for (name, block) in blocks {
        // Convert block_name into something rusty
        let name = name
            .strip_prefix("minecraft:")
            .expect("No prefix to block type")
            .to_owned()
            .to_case(Case::UpperCamel);

        if let Some(mut props) = block.properties {
            let mut params = Vec::with_capacity(props.len());

            if let Some(bitflag_name) = props
                .iter()
                .filter(|(name, _)| name.contains(char::is_numeric))
                .map(|(name, _)| name)
                .next()
                .cloned()
            {
                let Some((before, after)) = bitflag_name.split_once(char::is_numeric) else {
                    break;
                };

                // Detect how many options we have
                let mut index = 0;
                while props.contains_key(&format!("{before}{index}{after}")) {
                    index += 1;
                }

                let bitflag_name = if after.is_empty() {
                    before[..before.len() - 1].to_owned()
                } else {
                    format!("{}_{}", &before[..before.len() - 1], &after[1..])
                };

                params.push((bitflag_name, Type::Bitflags(index)));

                // Remove from props
                for i in 0..index {
                    props.remove(&format!("{before}{i}{after}"));
                }
            }

            for (prop, values) in props {
                let mut values = values
                    .iter()
                    .map(std::string::String::as_str)
                    .collect::<Vec<_>>();
                values.sort_unstable();

                if values.iter().all(|s| s.chars().all(char::is_numeric)) {
                    // Get max value
                    let highest = values.last().expect("No data in parameters");
                    params.push((prop, Type::Integer(highest.parse().expect("Non integer"))));
                    continue;
                }

                let prop = if prop == "type" {
                    "block_type".to_owned()
                } else {
                    prop
                };

                match &values[..] {
                    &["false", "true"] => {
                        params.push((prop, Type::Boolean));
                    },
                    &["bottom", "top"] | &["lower", "upper"] | &["bottom", "double", "top"] => {
                        params.push((prop, Type::Half));
                    },
                    &["x", "y", "z"] | &["x", "z"] => {
                        params.push((prop, Type::Axis));
                    },
                    &["low", "none", "tall"] => {
                        params.push((prop, Type::Height));
                    },
                    &["large", "none", "small"] => {
                        params.push((prop, Type::Bamboo));
                    },
                    &["foot", "head"] => {
                        params.push((prop, Type::Bed));
                    },
                    &["east", "north", "south", "west"]
                    | &["down", "east", "north", "south", "up", "west"]
                    | &["down", "up"]
                    | &["down", "east", "north", "south", "west"] => {
                        params.push((prop, Type::Direct));
                    },
                    &["none", "side", "up"] => {
                        params.push((prop, Type::Redstone));
                    },
                    &["compare", "subtract"] => {
                        params.push((prop, Type::Compare));
                    },
                    &["normal", "sticky"] => {
                        params.push((prop, Type::Piston));
                    },
                    &["left", "right"] => {
                        params.push((prop, Type::Side));
                    },
                    &["left", "right", "single"] => {
                        params.push((prop, Type::Chest));
                    },
                    &["ceiling", "floor", "wall"] => {
                        params.push((prop, Type::Face));
                    },
                    &["corner", "data", "load", "save"] => {
                        params.push((prop, Type::Structure));
                    },
                    &[
                        "banjo",
                        "basedrum",
                        "bass",
                        "bell",
                        "bit",
                        "chime",
                        "cow_bell",
                        "creeper",
                        "custom_head",
                        "didgeridoo",
                        "dragon",
                        "flute",
                        "guitar",
                        "harp",
                        "hat",
                        "iron_xylophone",
                        "piglin",
                        "pling",
                        "skeleton",
                        "snare",
                        "wither_skeleton",
                        "xylophone",
                        "zombie",
                    ] => {
                        params.push((prop, Type::Noteblock));
                    },
                    &[
                        "active",
                        "cooldown",
                        "ejecting_reward",
                        "inactive",
                        "waiting_for_players",
                        "waiting_for_reward_ejection",
                    ] => {
                        params.push((prop, Type::TrialSpawner));
                    },
                    &["active", "cooldown", "inactive"] => {
                        params.push((prop, Type::Cooldown));
                    },
                    &["ceiling", "double_wall", "floor", "single_wall"] => {
                        params.push((prop, Type::Bell));
                    },
                    &[
                        "inner_left",
                        "inner_right",
                        "outer_left",
                        "outer_right",
                        "straight",
                    ] => {
                        params.push((prop, Type::StairShape));
                    },
                    &[
                        "ascending_east",
                        "ascending_north",
                        "ascending_south",
                        "ascending_west",
                        "east_west",
                        "north_east",
                        "north_south",
                        "north_west",
                        "south_east",
                        "south_west",
                    ]
                    | &[
                        "ascending_east",
                        "ascending_north",
                        "ascending_south",
                        "ascending_west",
                        "east_west",
                        "north_south",
                    ] => {
                        params.push((prop, Type::RailShape));
                    },
                    &[
                        "down_east",
                        "down_north",
                        "down_south",
                        "down_west",
                        "east_up",
                        "north_up",
                        "south_up",
                        "up_east",
                        "up_north",
                        "up_south",
                        "up_west",
                        "west_up",
                    ] => {
                        params.push((prop, Type::Orient));
                    },
                    &["base", "frustum", "middle", "tip", "tip_merge"] => {
                        params.push((prop, Type::Dripstone));
                    },
                    &["full", "none", "partial", "unstable"] => {
                        params.push((prop, Type::Tilt));
                    },
                    unknown => println!("cargo::error=Unknown Type; {unknown:?}"),
                }
            }

            // Next, parse states
            let mut default = 0;
            let states = block
                .states
                .iter()
                .map(|state| {
                    if state.default.is_some() {
                        default = state.id;
                    }

                    let Some(ref props) = state.properties else {
                        panic!("Uhhhhh");
                    };

                    let mut values = Vec::new();
                    for (prop, value) in props {
                        let name = if prop == "type" {
                            "block_type"
                        } else {
                            prop.as_str()
                        };
                        values.push((name.to_owned(), value.clone()));
                    }

                    (state.id, values)
                })
                .collect::<Vec<_>>();

            parsed_blocks.push(ParsedBlock {
                name,
                default,
                properties: params,
                states,
            });
        } else {
            let id = block.states.first().expect("No states for block").id;
            parsed_blocks.push(ParsedBlock {
                name,
                default: id,
                properties: Vec::new(),
                states: vec![(id, Vec::new())],
            });
        }
    }

    Ok(parsed_blocks)
}
