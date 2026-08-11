use boml::Toml;
use std::path::Path;

pub struct ConfigParser;

impl ConfigParser {
    pub fn parse<const N: usize>(
        path: impl AsRef<Path>,
        keys: [&'static str; N],
    ) -> Result<[String; N], ConfigParserError> {
        let path = path.as_ref();

        let contents = std::fs::read_to_string(path).map_err(ConfigParserError::IoError)?;
        let toml = boml::parse(&contents).map_err(|err| {
            ConfigParserError::TomlError(format!("failed to parse TOML: {err:?}"))
        })?;

        fn get_str<'a>(toml: &'a Toml<'_>, key: &'static str) -> Result<String, ConfigParserError> {
            toml.get_string(key)
                .map(ToString::to_string)
                .map_err(|err| {
                    ConfigParserError::TomlError(format!("failed to get {key} key: {err:?}"))
                })
        }

        let values: [String; N] = keys
            .iter()
            .map(|key| get_str(&toml, *key))
            .collect::<Result<Vec<String>, ConfigParserError>>()?
            .try_into()
            .unwrap_or_else(|_| unreachable!());

        Ok(values)
    }
}

#[derive(Debug)]
pub enum ConfigParserError {
    IoError(std::io::Error),
    TomlError(String),
}

impl core::fmt::Display for ConfigParserError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "IoError({err})"),
            Self::TomlError(message) => write!(f, "TomlError({message})"),
        }
    }
}

impl core::error::Error for ConfigParserError {}
