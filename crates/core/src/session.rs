//! Turns the raw choices made in the UI into a validated plan.
use thiserror::Error;

use crate::backend::{BackendError, DiskBackend};
use crate::fs::FsType;
use crate::plan::{build_plan, EraseMode, FormatPlan, FormatRequest, PlanError};
use crate::preset::TableKind;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("unknown option: {0}")]
    UnknownOption(String),
    #[error("{0} was not found")]
    DiskNotFound(String),
    #[error(transparent)]
    Backend(#[from] BackendError),
    #[error(transparent)]
    Plan(#[from] PlanError),
}

/// Raw, string-typed choices coming from the UI.
#[derive(Debug, Clone, Copy)]
pub struct PlanInput<'a> {
    pub device: &'a str,
    pub fs_id: &'a str,
    pub table_id: &'a str,
    pub label: &'a str,
    pub erase_id: &'a str,
}

/// Looks the disk up afresh, parses the choices and applies every guardrail.
pub fn prepare_plan<B: DiskBackend + ?Sized>(
    backend: &B,
    available: &[FsType],
    input: &PlanInput,
) -> Result<FormatPlan, SessionError> {
    let unknown = |what: &str, id: &str| SessionError::UnknownOption(format!("{what}={id}"));
    let fs = FsType::from_udisks_name(input.fs_id).ok_or_else(|| unknown("fs", input.fs_id))?;
    let table =
        TableKind::from_id(input.table_id).ok_or_else(|| unknown("table", input.table_id))?;
    let erase =
        EraseMode::from_id(input.erase_id).ok_or_else(|| unknown("erase", input.erase_id))?;

    let disk = backend
        .list_disks()?
        .into_iter()
        .find(|d| d.device == input.device)
        .ok_or_else(|| SessionError::DiskNotFound(input.device.to_owned()))?;

    let request = FormatRequest {
        disk,
        fs,
        table,
        label: input.label.to_owned(),
        erase,
    };
    Ok(build_plan(&request, available)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::fake::FakeBackend;
    use crate::model::fixtures::*;
    use crate::plan::Step;

    fn input<'a>() -> PlanInput<'a> {
        PlanInput {
            device: "/dev/sdb",
            fs_id: "exfat",
            table_id: "gpt",
            label: "KEY",
            erase_id: "quick",
        }
    }

    fn backend() -> FakeBackend {
        FakeBackend::new(vec![usb_stick(), system_disk()])
    }

    #[test]
    fn valid_choices_produce_a_plan_for_the_chosen_disk() {
        let plan = prepare_plan(&backend(), &FsType::ALL, &input()).unwrap();
        assert_eq!(plan.disk().device, "/dev/sdb");
        assert_eq!(
            plan.steps().last(),
            Some(&Step::CreatePartition {
                fs: FsType::Exfat,
                label: "KEY".into(),
                table: TableKind::Gpt
            })
        );
    }

    #[test]
    fn erase_choice_is_carried_into_the_plan() {
        let zero = PlanInput {
            erase_id: "zero",
            ..input()
        };
        let plan = prepare_plan(&backend(), &FsType::ALL, &zero).unwrap();
        assert!(plan.steps().contains(&Step::CreateTable {
            kind: TableKind::Gpt,
            erase: EraseMode::Zero
        }));
    }

    #[test]
    fn unknown_ids_are_rejected_before_touching_anything() {
        for bad in [
            PlanInput {
                fs_id: "zfs",
                ..input()
            },
            PlanInput {
                table_id: "dos",
                ..input()
            },
            PlanInput {
                erase_id: "shred",
                ..input()
            },
        ] {
            let err = prepare_plan(&backend(), &FsType::ALL, &bad).unwrap_err();
            assert!(matches!(err, SessionError::UnknownOption(_)), "{err:?}");
        }
    }

    #[test]
    fn missing_disk_is_reported() {
        let missing = PlanInput {
            device: "/dev/sdz",
            ..input()
        };
        let err = prepare_plan(&backend(), &FsType::ALL, &missing).unwrap_err();
        assert!(matches!(err, SessionError::DiskNotFound(d) if d == "/dev/sdz"));
    }

    #[test]
    fn system_disk_is_refused_even_if_the_ui_let_it_through() {
        let sys = PlanInput {
            device: "/dev/nvme0n1",
            ..input()
        };
        let err = prepare_plan(&backend(), &FsType::ALL, &sys).unwrap_err();
        assert!(matches!(err, SessionError::Plan(PlanError::SystemDisk(_))));
    }

    #[test]
    fn unavailable_file_system_is_refused() {
        let ntfs = PlanInput {
            fs_id: "ntfs",
            ..input()
        };
        let available: Vec<_> = FsType::ALL
            .into_iter()
            .filter(|f| *f != FsType::Ntfs)
            .collect();
        let err = prepare_plan(&backend(), &available, &ntfs).unwrap_err();
        assert!(matches!(
            err,
            SessionError::Plan(PlanError::FsUnavailable(FsType::Ntfs))
        ));
    }
}
