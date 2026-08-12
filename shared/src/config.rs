use anyhow::{Context, Result, anyhow};
use boml::Toml;
use std::path::Path;

pub struct ConfigParser;

impl ConfigParser {
    pub fn parse<const N: usize>(
        path: impl AsRef<Path>,
        keys: [&'static str; N],
    ) -> Result<[String; N]> {
        let path = path.as_ref();

        let contents = std::fs::read_to_string(path).context("failed to read config file")?;
        let toml =
            boml::parse(&contents).map_err(|err| anyhow!("failed to parse TOML: {err:?}"))?;

        fn get_str<'a>(toml: &'a Toml<'_>, key: &'static str) -> Result<String> {
            toml.get_string(key)
                .map(ToString::to_string)
                .map_err(|err| anyhow!("failed to get {key} key: {err:?}"))
        }

        let values: [String; N] = keys
            .iter()
            .map(|key| get_str(&toml, *key))
            .collect::<Result<Vec<String>>>()?
            .try_into()
            .unwrap_or_else(|_| unreachable!());

        Ok(values)
    }
}
