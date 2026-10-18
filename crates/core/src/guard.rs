//! Keeps a panicking worker thread from leaving the UI stuck mid-operation.
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Shown to the user when a worker panics; the real cause is on stderr.
pub const INTERNAL_ERROR: &str =
    "Erreur interne : l'opération a échoué de façon inattendue. Relancez l'application.";

/// Runs `work`, turning a panic into a user-facing error instead of unwinding.
pub fn run_guarded<T>(work: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(work)).unwrap_or_else(|_| Err(INTERNAL_ERROR.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_successful_result_passes_through() {
        assert_eq!(run_guarded(|| Ok::<_, String>(7)), Ok(7));
    }

    #[test]
    fn an_error_passes_through_unchanged() {
        assert_eq!(
            run_guarded(|| Err::<(), _>("boom".to_owned())),
            Err("boom".to_owned())
        );
    }

    #[test]
    fn a_panic_becomes_the_internal_error_message() {
        let result = run_guarded(|| -> Result<(), String> { panic!("zbus exploded") });
        assert_eq!(result, Err(INTERNAL_ERROR.to_owned()));
    }
}
