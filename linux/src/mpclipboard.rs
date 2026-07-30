use anyhow::Result;
use mpclipboard_generic_client::{Config, ConfigReadOption, Output};
use rustix::event::{PollFd, PollFlags};

pub(crate) struct MPClipboard {
    mpclipboard: mpclipboard_generic_client::MPClipboard,
}

const CONFIG_READ_OPTION: ConfigReadOption = if cfg!(debug_assertions) {
    ConfigReadOption::FromLocalFile
} else {
    ConfigReadOption::FromXdgConfigDir
};

impl MPClipboard {
    pub(crate) fn init() -> Result<()> {
        mpclipboard_generic_client::MPClipboard::init()
    }

    pub(crate) fn new() -> Result<Self> {
        let config = Config::read(CONFIG_READ_OPTION)?;
        let mpclipboard = mpclipboard_generic_client::MPClipboard::new(config)?;

        Ok(Self { mpclipboard })
    }

    pub(crate) fn as_pollfd(&self) -> PollFd<'_> {
        PollFd::new(&self.mpclipboard, PollFlags::IN)
    }

    pub(crate) fn read(&mut self) -> Result<Option<Output>> {
        self.mpclipboard.read()
    }

    pub(crate) fn push_text(&mut self, text: String) -> Result<bool> {
        self.mpclipboard.push_text(text)
    }
}
