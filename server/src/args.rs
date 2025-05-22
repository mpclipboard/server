use anyhow::Result;

#[derive(Debug)]
pub(crate) enum Args {
    Generate,
    Start,
}

impl Args {
    const HELP: &str = r#"
Usage: shared-clipboard-server [OPTIONS]

Options:

  --generate   Generates a starting config and prints it to STDOUT
  --start      Starts a server
"#;

    pub(crate) fn parse() -> Result<Self> {
        let command = std::env::args().nth(1);

        match command.as_deref() {
            Some("--generate") => Ok(Self::Generate),
            Some("--start") => Ok(Self::Start),
            _ => {
                eprintln!("{}", Self::HELP);
                std::process::exit(1)
            }
        }
    }
}
