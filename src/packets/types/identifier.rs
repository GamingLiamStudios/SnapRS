use super::BoundedString;

pub struct Identifier(BoundedString<32767>);

impl From<String> for Identifier {
    fn from(value: String) -> Self {
        // Add default namespace if none is specified
        let mut parts = value.split(':');
        let mut namespace = parts.next().unwrap();
        let path = parts.next().unwrap_or_else(|| {
            let path = namespace.clone();
            namespace = "minecraft";
            path
        });

        // Check if Identifier is valid
        let value = format!("{}:{}", namespace, path);
        for part in value.split(':') {
            assert!(
                part.chars().all(|c| c.is_ascii_lowercase()
                    || c.is_ascii_digit()
                    || c == '_'
                    || c == '.'
                    || c == '/'
                    || c == '-'),
                "Invalid Identifier"
            );
        }

        Self(BoundedString::<32767>::from(value))
    }
}

impl From<&str> for Identifier {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

impl From<Identifier> for String {
    fn from(value: Identifier) -> Self {
        value.0.value
    }
}
