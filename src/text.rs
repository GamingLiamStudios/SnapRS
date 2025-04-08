use nom::IResult;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    encode::{
        Generate,
        SerializeFn,
        bounded_string,
    },
    parser::parse_string,
};

// TODO: Reduce number of owned things (Strings mainly)

// Possible specialization?
type Selector = String;

const fn default_seperator() -> Option<Box<TextComponent>> {
    None // TODO
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScoreboardValue {
    name:      Selector,
    objective: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum NbtSource {
    Block,
    Entity,
    Storage,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentType {
    Text {
        text: String,
    },
    #[serde(rename = "translatable")]
    Translated {
        translate: String,
        fallback:  Option<String>,
        with:      Option<Vec<TextComponent>>,
    },
    Scoreboard {
        score: ScoreboardValue,
    },
    Entities {
        selector:  Selector,
        #[serde(default = "default_seperator")]
        seperator: Option<Box<TextComponent>>,
    },
    Keybind {
        keybind: String,
    },
    NBT {
        source:    Option<NbtSource>,
        path:      String,
        interpret: Option<bool>,

        #[serde(default = "default_seperator")]
        seperator: Option<Box<TextComponent>>,

        block:   String,
        entity:  String,
        storage: String,
    },
}

fn default_font() -> String {
    "minecraft:default".to_string()
}

const fn default_modifier() -> bool {
    false
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ShadowColor {
    Packed(u32),
    List([f32; 4]),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(clippy::struct_excessive_bools)] // trust me clippy, this is intentional
pub struct TextComponent {
    /// Children will share formatting of parent
    #[serde(rename = "extra", skip_serializing_if = "Vec::is_empty")]
    children: Vec<TextComponent>,

    #[serde(flatten)]
    content: ContentType,

    // TODO: Specialize
    color: Option<String>,

    #[serde(default = "default_font")]
    font:          String,
    #[serde(default = "default_modifier")]
    bold:          bool,
    #[serde(default = "default_modifier")]
    italic:        bool,
    #[serde(default = "default_modifier")]
    underlined:    bool,
    #[serde(default = "default_modifier")]
    strikethrough: bool,
    #[serde(default = "default_modifier")]
    obfuscated:    bool,

    shadow_color: Option<ShadowColor>,
    // TODO: Interactivity
}

impl TextComponent {
    #[must_use]
    pub fn new_text(text: impl Into<String>) -> Self {
        Self {
            children:      Vec::new(),
            content:       ContentType::Text { text: text.into() },
            color:         None,
            font:          default_font(),
            bold:          false,
            italic:        false,
            underlined:    false,
            strikethrough: false,
            obfuscated:    false,
            shadow_color:  None,
        }
    }

    #[must_use]
    pub fn new_keybind(keybind: impl Into<String>) -> Self {
        Self {
            children:      Vec::new(),
            content:       ContentType::Keybind {
                keybind: keybind.into(),
            },
            color:         None,
            font:          default_font(),
            bold:          false,
            italic:        false,
            underlined:    false,
            strikethrough: false,
            obfuscated:    false,
            shadow_color:  None,
        }
    }

    pub fn add_child(
        &mut self,
        child: Self,
    ) {
        self.children.push(child);
    }

    #[must_use]
    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    #[must_use]
    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    #[must_use]
    pub const fn underlined(mut self) -> Self {
        self.underlined = true;
        self
    }

    #[must_use]
    pub const fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    #[must_use]
    pub const fn obfuscated(mut self) -> Self {
        self.obfuscated = true;
        self
    }

    #[must_use]
    pub fn color(
        mut self,
        color: impl Into<String>,
    ) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl From<&ParsingFormat> for TextComponent {
    fn from(val: &ParsingFormat) -> Self {
        match val {
            ParsingFormat::Standalone(text) => Self::new_text(text),
            ParsingFormat::Component(component) => component.clone(),
            ParsingFormat::List(list) => {
                let mut root: Self = list.first().expect("No front to list").into();
                for child in &list[1..] {
                    root.add_child(child.into());
                }
                root
            },
        }
    }
}

// The fun part bout this format is that it has many forms it can take
// - standalone string -> default formatting
// - component array -> later components are children to first component
// - full description -> just the normal spec

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum ParsingFormat {
    Standalone(String),
    Component(TextComponent),
    List(Vec<ParsingFormat>),
}

impl From<ParsingFormat> for TextComponent {
    fn from(val: ParsingFormat) -> Self {
        match val {
            ParsingFormat::Standalone(text) => Self::new_text(text),
            ParsingFormat::Component(component) => component,
            ParsingFormat::List(list) => {
                let mut root: Self = list.first().expect("No front to list").into();
                for child in &list[1..] {
                    root.add_child(child.into());
                }
                root
            },
        }
    }
}

#[allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]
pub fn write_json_text_component<E, T: Into<TextComponent> + serde::Serialize>(
    value: &T
) -> impl SerializeFn<E> {
    let json = serde_json::to_string(value).expect("Failed to encode TextComponent");
    move |buf| bounded_string::<262_144, E>(&json).generate_in_place(buf)
}

#[allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]
pub fn parse_json_text_component(data: &[u8]) -> IResult<&[u8], TextComponent> {
    let (data, json) = parse_string::<262_144>(data)?;
    let parsed: ParsingFormat = serde_json::from_str(json).expect("Failed to parse TextComponent");
    Ok((data, parsed.into()))
}

#[cfg(test)]
mod tests {
    // TODO: Obtain some example JSON text components to test with
}
