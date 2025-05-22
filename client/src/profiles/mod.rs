use std::time::Duration;

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

const HELP: &str = r#"
USAGE: client <PROFILE>

Profiles:

  alice     -- correct auth, interval = 1s, correct
  bob       -- correct auth, interval = 5s, correct
  hacker    -- incorrect auth
  malformed -- correct auth, interval = 1s, incorrect (sends malformed clips)
"#;

pub(crate) fn select_profile() -> Profile {
    let name = std::env::args().nth(1);

    match name.as_deref() {
        Some("alice") => ALICE,
        Some("bob") => BOB,
        Some("hacker") => HACKER,
        Some("malformed") => MALFORMED,
        _ => {
            eprintln!("{}", HELP);
            std::process::exit(1);
        }
    }
}
