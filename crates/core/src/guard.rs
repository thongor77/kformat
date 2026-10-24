//! Keeps a panicking worker thread from leaving the UI stuck mid-operation.
use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::i18n::Lang;
use crate::messages::{tr, Msg};

/// Runs `work`, turning a panic into a user-facing error (in `lang`) instead of unwinding.
/// The real cause is printed on stderr by the panic hook.
pub fn run_guarded<T>(lang: Lang, work: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(work)).unwrap_or_else(|_| Err(tr(lang, Msg::InternalError, &[])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_successful_result_passes_through() {
        assert_eq!(run_guarded(Lang::En, || Ok::<_, String>(7)), Ok(7));
    }

    #[test]
    fn an_error_passes_through_unchanged() {
        assert_eq!(
            run_guarded(Lang::En, || Err::<(), _>("boom".to_owned())),
            Err("boom".to_owned())
        );
    }

    #[test]
    fn a_panic_becomes_the_internal_error_message() {
        let result = run_guarded(Lang::Fr, || -> Result<(), String> {
            panic!("zbus exploded")
        });
        assert_eq!(result, Err(tr(Lang::Fr, Msg::InternalError, &[])));
    }
}
