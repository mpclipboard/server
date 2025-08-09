use crate::clip::Clip;

#[derive(Default)]
pub(crate) struct Store {
    clip: Option<Clip>,
}

impl Store {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn current(&self) -> Option<Clip> {
        self.clip.clone()
    }

    #[must_use]
    pub(crate) fn add(&mut self, clip: &Clip) -> bool {
        match self.clip.as_mut() {
            Some(current) => {
                if clip.timestamp > current.timestamp && clip.text != current.text {
                    *current = clip.clone();
                    true
                } else {
                    false
                }
            }
            None => {
                self.clip = Some(clip.clone());
                true
            }
        }
    }
}
