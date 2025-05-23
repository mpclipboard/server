use std::time::Duration;

use anyhow::bail;

#[derive(Debug)]
pub(crate) struct Profile {
    pub(crate) name: &'static str,
    pub(crate) auth: bool,
    pub(crate) interval: Duration,
    pub(crate) correct: bool,
}

const ALICE: Profile = Profile {
    name: "alice",
    auth: true,
    interval: Duration::from_secs(1),
    correct: true,
};

const BOB: Profile = Profile {
    name: "bob",
    auth: true,
    interval: Duration::from_secs(5),
    correct: true,
};

const HACKER: Profile = Profile {
    name: "hacker",
    auth: false,
    interval: Duration::from_secs(1),
    correct: true,
};

const MALFORMED: Profile = Profile {
    name: "malformed",
    auth: true,
    interval: Duration::from_secs(1),
    correct: false,
};

impl TryFrom<String> for Profile {
    type Error = anyhow::Error;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        match &name[..] {
            "alice" => Ok(ALICE),
            "bob" => Ok(BOB),
            "hacker" => Ok(HACKER),
            "malformed" => Ok(MALFORMED),
            other => bail!("unknown profile {other:?}, known: alice, bob, hacker, malformed"),
        }
    }
}
