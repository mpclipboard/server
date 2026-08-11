const REAP_STRANGER_AFTER_IN_SECS: u64 = 3;

pub trait CanBeReaped {
    fn last_activity_at(&self) -> u64;

    fn must_be_reaped(&self, now: u64) -> bool {
        now - self.last_activity_at() > REAP_STRANGER_AFTER_IN_SECS
    }
}
