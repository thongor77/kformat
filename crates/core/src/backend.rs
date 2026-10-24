use thiserror::Error;

use crate::inhibit::SleepGuard;
use crate::model::Disk;
use crate::plan::ConfirmedPlan;

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("cannot talk to udisks2: {0}")]
    Bus(String),
    #[error("step {step} of {total} failed: {reason}")]
    StepFailed {
        step: usize,
        total: usize,
        reason: String,
    },
    #[error("{0} is no longer present")]
    DiskGone(String),
    #[error("{0} now holds the running system; aborting")]
    DiskBecameSystem(String),
    #[error("{0} is no longer the disk that was confirmed; aborting")]
    DiskChanged(String),
    #[error("stopped after {completed} of {total} steps: {cause}")]
    Interrupted {
        completed: usize,
        total: usize,
        cause: Box<BackendError>,
    },
    #[error("cannot read udisks2 data ({what}): {reason}")]
    Parse { what: String, reason: String },
}

/// Crate-private supertrait: only this crate can implement a backend or run a
/// single step, so every write goes through a confirmed plan.
pub(crate) mod sealed {
    use super::BackendError;
    use crate::model::Disk;
    use crate::plan::Step;

    pub trait StepRunner {
        /// Runs one step against `disk`; implementations do the actual I/O.
        fn run_step(&self, disk: &Disk, step: &Step) -> Result<(), BackendError>;
    }
}

/// How far a plan has progressed, in whole steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub completed: usize,
    pub total: usize,
}

impl Progress {
    pub fn fraction(self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            self.completed as f64 / self.total as f64
        }
    }
}

/// Reason shown by the system while it refuses to sleep or shut down.
pub const INHIBIT_REASON: &str = "Formatage d'un disque en cours";

pub trait DiskBackend: sealed::StepRunner {
    fn list_disks(&self) -> Result<Vec<Disk>, BackendError>;

    /// Asks the system not to sleep or shut down until the guard is dropped.
    /// `None` means the protection is unavailable; callers must carry on without it.
    fn inhibit_sleep(&self, _reason: &str) -> Option<SleepGuard> {
        None
    }

    /// Whether `inhibit_sleep` works right now (takes and immediately drops a lock).
    fn sleep_protection_available(&self) -> bool {
        self.inhibit_sleep(INHIBIT_REASON).is_some()
    }

    /// Runs every step of a confirmed plan, stopping at the first failure.
    ///
    /// Before each step the disk is looked up again: it must still be the same
    /// drive and must not have become a system disk. Before the first step the
    /// partition layout must also match what the user saw.
    fn execute(
        &self,
        plan: &ConfirmedPlan,
        on_progress: &mut dyn FnMut(Progress),
    ) -> Result<(), BackendError> {
        // Held until this function returns (success, error or panic): dropping releases it.
        let _sleep_guard = self.inhibit_sleep(INHIBIT_REASON);
        let total = plan.steps().len();
        for (index, step) in plan.steps().iter().enumerate() {
            let current = verify_unchanged(self, plan.disk(), index == 0).map_err(|cause| {
                if index == 0 {
                    cause
                } else {
                    BackendError::Interrupted {
                        completed: index,
                        total,
                        cause: Box::new(cause),
                    }
                }
            })?;
            sealed::StepRunner::run_step(self, &current, step).map_err(|err| {
                BackendError::StepFailed {
                    step: index + 1,
                    total,
                    reason: err.to_string(),
                }
            })?;
            on_progress(Progress {
                completed: index + 1,
                total,
            });
        }
        Ok(())
    }
}

/// Fresh view of `planned`, or an error if it is gone, swapped or now in use by the system.
fn verify_unchanged<B: DiskBackend + ?Sized>(
    backend: &B,
    planned: &Disk,
    check_layout: bool,
) -> Result<Disk, BackendError> {
    let current = backend
        .list_disks()?
        .into_iter()
        .find(|d| d.device == planned.device)
        .ok_or_else(|| BackendError::DiskGone(planned.device.clone()))?;
    let unchanged = if check_layout {
        current.same_media(planned)
    } else {
        current.same_drive(planned)
    };
    if !unchanged {
        return Err(BackendError::DiskChanged(current.device));
    }
    if current.is_system() {
        return Err(BackendError::DiskBecameSystem(current.device));
    }
    Ok(current)
}

#[cfg(test)]
pub(crate) mod fake {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::sealed::StepRunner;
    use super::*;
    use crate::plan::Step;

    /// In-memory backend recording every step it is asked to run.
    ///
    /// `listings` is consumed one entry per `list_disks` call; the last entry repeats.
    pub struct FakeBackend {
        pub listings: RefCell<VecDeque<Vec<Disk>>>,
        pub fail_at: Option<usize>,
        pub calls: RefCell<Vec<Step>>,
        /// Whether `inhibit_sleep` succeeds.
        pub inhibit_available: bool,
        /// Locks currently held.
        pub locks_held: Arc<AtomicUsize>,
        /// Locks held at the moment each step ran.
        pub locks_at_step: RefCell<Vec<usize>>,
    }

    /// Counts a held lock; releases it when dropped.
    struct CountedLock(Arc<AtomicUsize>);

    impl Drop for CountedLock {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }

    impl FakeBackend {
        pub fn new(disks: Vec<Disk>) -> Self {
            Self::with_listings(vec![disks])
        }

        pub fn with_listings(listings: Vec<Vec<Disk>>) -> Self {
            Self {
                listings: RefCell::new(listings.into()),
                fail_at: None,
                calls: RefCell::new(vec![]),
                inhibit_available: true,
                locks_held: Arc::new(AtomicUsize::new(0)),
                locks_at_step: RefCell::new(vec![]),
            }
        }
    }

    impl DiskBackend for FakeBackend {
        fn inhibit_sleep(&self, _reason: &str) -> Option<SleepGuard> {
            if !self.inhibit_available {
                return None;
            }
            self.locks_held.fetch_add(1, Ordering::SeqCst);
            Some(SleepGuard::new(CountedLock(self.locks_held.clone())))
        }

        fn list_disks(&self) -> Result<Vec<Disk>, BackendError> {
            let mut listings = self.listings.borrow_mut();
            if listings.len() > 1 {
                Ok(listings.pop_front().unwrap_or_default())
            } else {
                Ok(listings.front().cloned().unwrap_or_default())
            }
        }
    }

    impl StepRunner for FakeBackend {
        fn run_step(&self, _disk: &Disk, step: &Step) -> Result<(), BackendError> {
            let index = self.calls.borrow().len();
            self.locks_at_step
                .borrow_mut()
                .push(self.locks_held.load(Ordering::SeqCst));
            self.calls.borrow_mut().push(step.clone());
            match self.fail_at {
                Some(at) if at == index => Err(BackendError::Bus("boom".into())),
                _ => Ok(()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::fake::FakeBackend;
    use super::*;
    use crate::fs::FsType;
    use crate::model::fixtures::*;
    use crate::plan::{build_plan, EraseMode, FormatRequest};
    use crate::preset::TableKind;

    fn confirmed(disk: Disk) -> ConfirmedPlan {
        let request = FormatRequest {
            disk,
            fs: FsType::Ext4,
            table: TableKind::Gpt,
            label: "DATA".into(),
            erase: EraseMode::Quick,
        };
        let plan = build_plan(&request, &FsType::ALL).unwrap();
        let phrase = plan.confirmation_phrase().to_owned();
        plan.confirm(&phrase).unwrap()
    }

    #[test]
    fn execute_runs_all_steps_in_order_and_reports_progress() {
        let backend = FakeBackend::new(vec![usb_stick()]);
        let plan = confirmed(usb_stick());
        let mut seen = vec![];
        backend.execute(&plan, &mut |p| seen.push(p)).unwrap();
        assert_eq!(backend.calls.borrow().as_slice(), plan.steps());
        assert_eq!(
            seen,
            vec![
                Progress {
                    completed: 1,
                    total: 2
                },
                Progress {
                    completed: 2,
                    total: 2
                },
            ]
        );
    }

    #[test]
    fn sleep_is_inhibited_during_every_step_and_released_afterwards() {
        let backend = FakeBackend::new(vec![usb_stick()]);
        backend
            .execute(&confirmed(usb_stick()), &mut |_| {})
            .unwrap();
        assert_eq!(*backend.locks_at_step.borrow(), vec![1, 1]);
        assert_eq!(backend.locks_held.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn the_lock_is_released_when_a_step_fails() {
        let mut backend = FakeBackend::new(vec![usb_stick()]);
        backend.fail_at = Some(0);
        assert!(backend
            .execute(&confirmed(usb_stick()), &mut |_| {})
            .is_err());
        assert_eq!(backend.locks_held.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn the_lock_is_released_when_the_disk_check_aborts_the_run() {
        let backend = FakeBackend::new(vec![]);
        assert!(backend
            .execute(&confirmed(usb_stick()), &mut |_| {})
            .is_err());
        assert_eq!(backend.locks_held.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn formatting_proceeds_without_the_lock_when_it_is_unavailable() {
        let mut backend = FakeBackend::new(vec![usb_stick()]);
        backend.inhibit_available = false;
        backend
            .execute(&confirmed(usb_stick()), &mut |_| {})
            .unwrap();
        assert_eq!(*backend.locks_at_step.borrow(), vec![0, 0]);
        assert_eq!(backend.calls.borrow().len(), 2);
    }

    #[test]
    fn probing_protection_leaves_no_lock_behind() {
        let backend = FakeBackend::new(vec![usb_stick()]);
        assert!(backend.sleep_protection_available());
        assert_eq!(backend.locks_held.load(Ordering::SeqCst), 0);
        let mut without = FakeBackend::new(vec![usb_stick()]);
        without.inhibit_available = false;
        assert!(!without.sleep_protection_available());
    }

    #[test]
    fn execute_stops_at_the_first_failing_step() {
        let mut backend = FakeBackend::new(vec![usb_stick()]);
        backend.fail_at = Some(0);
        let err = backend
            .execute(&confirmed(usb_stick()), &mut |_| {})
            .unwrap_err();
        assert!(matches!(
            err,
            BackendError::StepFailed {
                step: 1,
                total: 2,
                ..
            }
        ));
        assert_eq!(backend.calls.borrow().len(), 1);
    }

    #[test]
    fn execute_aborts_if_the_disk_vanished() {
        let backend = FakeBackend::new(vec![]);
        let err = backend
            .execute(&confirmed(usb_stick()), &mut |_| {})
            .unwrap_err();
        assert!(matches!(err, BackendError::DiskGone(_)));
        assert!(backend.calls.borrow().is_empty());
    }

    #[test]
    fn execute_aborts_if_the_disk_became_a_system_disk() {
        let mut now_system = usb_stick();
        now_system.partitions = vec![partition("/dev/sdb1", &["/"])];
        let backend = FakeBackend::new(vec![now_system]);
        let err = backend
            .execute(&confirmed(usb_stick()), &mut |_| {})
            .unwrap_err();
        assert!(matches!(err, BackendError::DiskBecameSystem(_)));
        assert!(backend.calls.borrow().is_empty());
    }

    #[test]
    fn execute_aborts_if_another_drive_took_the_device_name() {
        let mut impostor = usb_stick();
        impostor.id = "usb-Kingston_DT-9".into();
        let backend = FakeBackend::new(vec![impostor]);
        let err = backend
            .execute(&confirmed(usb_stick()), &mut |_| {})
            .unwrap_err();
        assert!(matches!(err, BackendError::DiskChanged(_)));
        assert!(backend.calls.borrow().is_empty());
    }

    #[test]
    fn execute_aborts_if_the_partition_layout_changed_before_the_first_step() {
        let mut other = usb_stick();
        other.partitions[0].fs_type = "ntfs".into();
        let backend = FakeBackend::new(vec![other]);
        let err = backend
            .execute(&confirmed(usb_stick()), &mut |_| {})
            .unwrap_err();
        assert!(matches!(err, BackendError::DiskChanged(_)));
    }

    #[test]
    fn execute_rechecks_the_disk_between_steps() {
        let mut now_system = usb_stick();
        now_system.holder_mounts = vec!["/".into()];
        let backend = FakeBackend::with_listings(vec![vec![usb_stick()], vec![now_system]]);
        let err = backend
            .execute(&confirmed(usb_stick()), &mut |_| {})
            .unwrap_err();
        assert!(
            matches!(
                &err,
                BackendError::Interrupted { completed: 1, total: 2, cause }
                    if matches!(**cause, BackendError::DiskBecameSystem(_))
            ),
            "{err:?}"
        );
        assert_eq!(
            backend.calls.borrow().len(),
            1,
            "only the first step may have run"
        );
    }

    #[test]
    fn fraction_of_an_empty_plan_is_complete() {
        assert_eq!(
            Progress {
                completed: 0,
                total: 0
            }
            .fraction(),
            1.0
        );
    }
}
