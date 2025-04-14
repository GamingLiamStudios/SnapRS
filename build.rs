use std::{
    collections::{
        HashMap,
        HashSet,
    },
    env,
    error::Error,
    fs::File,
    io::{
        BufReader,
        Write,
    },
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

fn parse_state(
    writer: &mut File,
    props: &[(String, Type)],
    state: &[(String, String)],
    prefix: &str,
) -> std::io::Result<()> {
    if let Some(bitflag_name) = state
        .iter()
        .filter(|(name, _)| name.contains(char::is_numeric))
        .map(|(name, _)| name)
        .next()
        .cloned()
    {
        if let Some((before, after)) = bitflag_name.split_once(char::is_numeric) {
            let mut bitflag = 0;
            let mut index = 0;
            while let Some((_, value)) = state
                .iter()
                .find(|(name, _)| name == &format!("{before}{index}{after}"))
            {
                bitflag |= u8::from(value == "true") << index;
                index += 1;
            }

            let bitflag_name = if after.is_empty() {
                before[..before.len() - 1].to_owned()
            } else {
                format!("{}_{}", &before[..before.len() - 1], &after[1..])
            };
            writeln!(writer, "{prefix}{bitflag_name}: {bitflag},")?;
        }
    }

    for (name, value) in state {
        if name.contains(char::is_numeric) {
            continue;
        }

        // Find type
        let (_, prop_type) = props
            .iter()
            .find(|(prop_name, _)| prop_name == name)
            .expect("Property doesn't exist");

        let value = match value.as_str() {
            "true" => "true",
            "false" => "false",
            "lower" => "Bottom",
            "upper" => "Top",
            "basedrum" => "BaseDrum",
            value if value.contains(char::is_numeric) => value,
            value => &value.to_case(Case::UpperCamel),
        };
        if matches!(prop_type, Type::Boolean | Type::Integer(_)) {
            writeln!(writer, "{prefix}{name}: {value},")?;
        } else {
            writeln!(
                writer,
                "{prefix}{name}: {}::{value},",
                prop_type.to_string()
            )?;
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct ParsedBlock {
    name:       String,
    properties: Vec<(String, Type)>,
    states:     Vec<(u16, Vec<(String, String)>)>,
    default:    u16,
}

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR")?;
    println!("cargo::warning={out_dir}");

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

    // Now that we have something parsed, lets turn it into code
    let mut blocks_file = File::create(format!("{out_dir}/blocks.rs"))?;

    writeln!(&mut blocks_file, "// Automatically Generated")?;

    // Blocks
    writeln!(
        &mut blocks_file,
        "#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]"
    )?;
    writeln!(&mut blocks_file, "#[repr(u16)]")?;
    writeln!(&mut blocks_file, "pub enum Blocks {{")?;
    for block in &parsed_blocks {
        writeln!(&mut blocks_file, "\t{} = {},", block.name, block.default)?;
    }
    writeln!(&mut blocks_file, "}}\n")?;

    // Enum definition
    writeln!(
        &mut blocks_file,
        "#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]"
    )?;
    writeln!(&mut blocks_file, "pub enum BlockState {{")?;
    for block in &parsed_blocks {
        write!(&mut blocks_file, "\t{}", block.name)?;

        match &block.properties[..] {
            &[] => (),
            properties => {
                writeln!(&mut blocks_file, " {{")?;
                for (name, property) in properties {
                    writeln!(&mut blocks_file, "\t\t{name}: {},", property.to_string())?;
                }
                write!(&mut blocks_file, "\t}}")?;
            },
        }

        writeln!(&mut blocks_file, ",")?;
    }
    writeln!(&mut blocks_file, "}}\n")?;

    // TODO: Convert Type::Bitfield into bitfields!
    writeln!(&mut blocks_file, "impl BlockState {{")?;

    writeln!(&mut blocks_file, "\t#[must_use]")?;
    writeln!(&mut blocks_file, "\tpub fn max_age(&self) -> Option<u8> {{")?;
    writeln!(&mut blocks_file, "\t\tmatch self {{")?;
    for block in parsed_blocks.iter().filter(|block| {
        block
            .properties
            .iter()
            .any(|(name, prop)| name == "age" && matches!(prop, Type::Integer(_)))
    }) {
        // Search through states to find largest age
        let mut max_age = 0;
        for (_id, state) in &block.states {
            for (name, value) in state {
                if name == "age" {
                    max_age = max_age.max(value.parse().expect("Non integer age"));
                }
            }
        }
        writeln!(
            &mut blocks_file,
            "\t\t\tSelf::{} {{ .. }} => Some({max_age}),",
            block.name
        )?;
    }
    writeln!(&mut blocks_file, "\t\t\t_ => None,")?;
    writeln!(&mut blocks_file, "\t\t}}")?;
    writeln!(&mut blocks_file, "\t}}")?;

    writeln!(&mut blocks_file, "\t#[must_use]")?;
    writeln!(
        &mut blocks_file,
        "\tpub fn default(block: Blocks) -> Self {{"
    )?;
    writeln!(&mut blocks_file, "\t\tmatch block {{")?;
    for block in &parsed_blocks {
        for (id, state) in &block.states {
            if *id != block.default {
                continue;
            }

            // Convert specified state into actual state
            write!(
                &mut blocks_file,
                "\t\t\tBlocks::{0} => Self::{0}",
                block.name
            )?;

            if !state.is_empty() {
                writeln!(&mut blocks_file, " {{")?;
                parse_state(&mut blocks_file, &block.properties, state, "\t\t\t\t")?;
                write!(&mut blocks_file, "\t\t\t}}")?;
            }
            writeln!(&mut blocks_file, ",")?;
        }
    }
    writeln!(&mut blocks_file, "\t\t}}")?;
    writeln!(&mut blocks_file, "\t}}")?;

    // TODO: IMPROVE THIS ITS NEARLY 400 THOUSAND LINES

    writeln!(&mut blocks_file, "\t#[must_use]")?;
    writeln!(&mut blocks_file, "\tpub fn from_id(id: u16) -> Self {{")?;
    writeln!(&mut blocks_file, "\t\tmatch id {{")?;
    for block in &parsed_blocks {
        for (id, state) in &block.states {
            // Convert specified state into actual state
            write!(&mut blocks_file, "\t\t\t{id} => Self::{0}", block.name)?;

            if !state.is_empty() {
                writeln!(&mut blocks_file, " {{")?;
                parse_state(&mut blocks_file, &block.properties, state, "\t\t\t\t")?;
                write!(&mut blocks_file, "\t\t\t}}")?;
            }
            writeln!(&mut blocks_file, ",")?;
        }
    }
    writeln!(
        &mut blocks_file,
        "\t\t\t_ => unimplemented!(\"BlockID doesn't exist\"),"
    )?;
    writeln!(&mut blocks_file, "\t\t}}")?;
    writeln!(&mut blocks_file, "\t}}")?;

    writeln!(&mut blocks_file, "\t#[must_use]")?;
    writeln!(&mut blocks_file, "\tpub fn to_id(&self) -> u16 {{")?;
    writeln!(&mut blocks_file, "\t\tmatch self {{")?;
    for block in &parsed_blocks {
        for (id, state) in &block.states {
            // Convert specified state into actual state
            write!(&mut blocks_file, "\t\t\tSelf::{0}", block.name)?;

            if !state.is_empty() {
                writeln!(&mut blocks_file, " {{")?;
                parse_state(&mut blocks_file, &block.properties, state, "\t\t\t\t")?;
                write!(&mut blocks_file, "\t\t\t}}")?;
            }
            writeln!(&mut blocks_file, " => {id},")?;
        }
    }
    writeln!(
        &mut blocks_file,
        "\t\t\t_ => unimplemented!(\"BlockID doesn't exist\"),"
    )?;
    writeln!(&mut blocks_file, "\t\t}}")?;
    writeln!(&mut blocks_file, "\t}}")?;

    writeln!(&mut blocks_file, "}}")?;

    Ok(())
}
