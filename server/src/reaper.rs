const REAP_STRANGER_AFTER_IN_SECS: u64 = 3;

pub trait CanBeReaped {
    fn last_activity_at(&self) -> u64;

    fn must_be_reaped(&self, now: u64) -> bool {
        let inactive_for = now
            .checked_sub(self.last_activity_at())
            .unwrap_or_else(|| unreachable!("time goes backwards"));
        inactive_for > REAP_STRANGER_AFTER_IN_SECS
    }
}
