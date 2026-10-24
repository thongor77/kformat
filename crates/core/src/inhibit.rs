//! Keeps the machine awake while a plan runs.

/// Holds a sleep/shutdown inhibitor (a logind lock fd in the real backend)
/// until it is dropped, so the lock is released on success, failure or panic alike.
pub struct SleepGuard {
    _holder: Box<dyn Send>,
}

impl SleepGuard {
    pub(crate) fn new(holder: impl Send + 'static) -> Self {
        Self {
            _holder: Box::new(holder),
        }
    }
}
