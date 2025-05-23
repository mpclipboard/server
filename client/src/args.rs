use crate::Profile;
use anyhow::Result;

pub(crate) struct Args {
    pub(crate) profile: Profile,
    pub(crate) url: String,
    pub(crate) token: String,
}

fn print_help_and_exit() -> ! {
    eprintln!("USAGE: client <PROFILE> <URL> <TOKEN>");
    std::process::exit(1);
}

fn arg_at(n: usize) -> String {
    std::env::args()
        .nth(n)
        .unwrap_or_else(|| print_help_and_exit())
}

impl Args {
    pub(crate) fn parse() -> Result<Self> {
        let profile = Profile::try_from(arg_at(1))?;
        let url = arg_at(2);
        let token = arg_at(3);
        Ok(Self {
            profile,
            url,
            token,
        })
    }
}

impl std::fmt::Debug for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Args")
            .field("profile", &self.profile)
            .field("url", &self.url)
            .field("token", &"*****")
            .finish()
    }
}
