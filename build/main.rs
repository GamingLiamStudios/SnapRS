#![allow(clippy::pedantic)]

use std::{
    env,
    error::Error,
    fs::File,
    io::{
        BufReader,
        Write,
    },
};

use blocks::{
    Type,
    parse_blocks,
};
use convert_case::{
    Case,
    Casing,
};
use proc_macro2::Span;
use quote::{
    format_ident,
    quote,
};
use registry::{
    Registies,
    RegistryEntry,
};

mod blocks;
mod registry;

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

fn create_blocks() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let parsed_blocks = parse_blocks()?;

    // Now that we have something parsed, lets turn it into code
    let mut blocks_file = File::create(format!("{out_dir}/block_states.rs"))?;

    writeln!(&mut blocks_file, "// Automatically Generated")?;

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

fn create_registry_enum(
    registries: &Registies,
    name: &str,
) -> syn::File {
    let registry = registries
        .get(&format!("minecraft:{name}"))
        .expect("No such registry exists");
    let default = registry
        .default
        .as_ref()
        .expect("No default entity type")
        .strip_prefix("minecraft:")
        .expect("No prefix to entity type")
        .to_owned()
        .to_case(Case::UpperCamel);
    let entries = registry
        .entries
        .iter()
        .map(|(name, RegistryEntry { protocol_id })| {
            // Convert name to something nicer
            let name = name
                .strip_prefix("minecraft:")
                .expect("No prefix to entity type")
                .to_owned()
                .to_case(Case::UpperCamel);

            let ident = syn::Ident::new(&name, Span::call_site());
            syn::parse_quote! { #ident = #protocol_id }
        })
        .collect::<Vec<syn::Variant>>();

    let name = name.to_owned().to_case(Case::UpperCamel);

    let name = syn::Ident::new(&name, Span::call_site());
    let default = syn::Ident::new(&default, Span::call_site());

    syn::parse_quote! {
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
        #[repr(u16)]
        pub enum #name {
            #(#entries),*
        }

        impl Default for #name {
            fn default() -> Self {
                Self::#default
            }
        }
    }
}

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR")?;

    println!("cargo::rerun-if-changed=generated/reports/registries.json");
    let reader = File::open("generated/reports/registries.json")?;
    let registries: Registies = serde_json::from_reader(reader)?;

    let blocks = registries
        .get("minecraft:block")
        .expect("No block registry exists");
    let blocks = blocks
        .entries
        .iter()
        .map(|(name, RegistryEntry { protocol_id })| {
            // Convert name to something nicer
            let name = name
                .strip_prefix("minecraft:")
                .expect("No prefix to block type")
                .to_owned()
                .to_case(Case::UpperCamel);

            let ident = syn::Ident::new(&name, Span::call_site());
            syn::parse_quote! { #ident = #protocol_id }
        })
        .collect::<Vec<syn::Variant>>();

    let blocks = syn::parse_quote! {
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
        #[repr(u16)]
        pub enum Blocks {
            #(#blocks),*
        }
    };

    let entity_types = create_registry_enum(&registries, "entity_type");
    let fluids = create_registry_enum(&registries, "fluid");
    let game_events = create_registry_enum(&registries, "game_event");
    let items = create_registry_enum(&registries, "item");

    // Fetch known registries
    let list_files = |path: &str| -> Vec<_> {
        println!("cargo::rerun-if-changed={path}");
        let mut filenames = Vec::new();

        for entry in std::fs::read_dir(path).expect("shitface") {
            let entry = entry.expect("shitface");
            let path = entry.path();
            if path.is_file() {
                let filename = path
                    .file_stem()
                    .expect("shitface")
                    .to_string_lossy()
                    .to_string();
                filenames.push(filename);
            }
        }

        filenames.iter().map(|name| quote! { #name }).collect()
    };

    let biomes = list_files("generated/data/minecraft/worldgen/biome");
    let damage_types = list_files("generated/data/minecraft/damage_type");

    let tokens = syn::parse_quote! {
        pub mod entity_types;
        pub mod blocks;
        pub mod items;

        pub use entity_types::EntityType;
        pub use blocks::Blocks;

        pub const BIOME_NAMES: &[&str] = &[
            #(#biomes),*
        ];

        pub const DAMAGE_TYPES: &[&str] = &[
            #(#damage_types),*
        ];

        #game_events
        #fluids
    };

    std::fs::write(
        format!("{out_dir}/registry.rs"),
        prettyplease::unparse(&tokens),
    )?;
    std::fs::write(
        format!("{out_dir}/entity_types.rs"),
        prettyplease::unparse(&entity_types),
    )?;
    std::fs::write(
        format!("{out_dir}/blocks.rs"),
        prettyplease::unparse(&blocks),
    )?;
    std::fs::write(format!("{out_dir}/items.rs"), prettyplease::unparse(&items))?;

    create_blocks()?;

    Ok(())
}
