use std::path::Path;

pub struct ConfigParser;

impl ConfigParser {
    pub fn parse<const N: usize>(
        path: impl AsRef<Path>,
        keys: [&'static str; N],
    ) -> std::io::Result<[String; N]> {
        let path = path.as_ref();

        let contents = std::fs::read_to_string(path)?;
        let toml =
            boml::parse(&contents).map_err(|err| std::io::Error::other(format!("{err:?}")))?;

        let mut values = core::array::from_fn(|_| String::new());

        for (idx, key) in keys.iter().enumerate() {
            let value = toml
                .get_string(key)
                .map_err(|err| std::io::Error::other(format!("failed to get {key}: {err:?}")))?;

            values[idx] = value.to_string();
        }

        Ok(values)
    }
}
