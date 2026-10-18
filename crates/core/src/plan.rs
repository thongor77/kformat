use thiserror::Error;

use crate::fs::{FsType, LabelError};
use crate::model::Disk;
use crate::preset::TableKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraseMode {
    /// Rewrite only the table and file system structures.
    Quick,
    /// Overwrite the whole disk with zeros first.
    Zero,
}

impl EraseMode {
    /// Stable identifier shared with the UI.
    pub fn id(self) -> &'static str {
        match self {
            EraseMode::Quick => "quick",
            EraseMode::Zero => "zero",
        }
    }

    pub fn from_id(id: &str) -> Option<EraseMode> {
        [EraseMode::Quick, EraseMode::Zero]
            .into_iter()
            .find(|mode| mode.id() == id)
    }
}

/// What the user asked for, before any safety check.
#[derive(Debug, Clone)]
pub struct FormatRequest {
    pub disk: Disk,
    pub fs: FsType,
    pub table: TableKind,
    pub label: String,
    pub erase: EraseMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Unmount {
        device: String,
    },
    CreateTable {
        kind: TableKind,
        erase: EraseMode,
    },
    CreatePartition {
        fs: FsType,
        label: String,
        table: TableKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlanError {
    #[error("{0} holds the running system and cannot be formatted")]
    SystemDisk(String),
    #[error("{} cannot be created: {} is not installed", .0.display_name(), .0.mkfs_binary())]
    FsUnavailable(FsType),
    #[error("invalid label: {0}")]
    InvalidLabel(#[from] LabelError),
    #[error("the disk is too small for {}", .0.display_name())]
    DiskTooSmall(FsType),
    #[error("an MBR table cannot address a disk this large; use GPT")]
    DiskTooLargeForMbr,
    #[error("the disk has no usable device name")]
    InvalidDisk,
    #[error("confirmation does not match the disk name")]
    ConfirmationMismatch,
}

/// A validated plan, waiting for the user to type the disk name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatPlan {
    disk: Disk,
    steps: Vec<Step>,
}

/// A plan the user has explicitly confirmed; the only thing a backend can run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedPlan {
    disk: Disk,
    steps: Vec<Step>,
}

/// Applies every guardrail to `request` and returns the steps to run.
pub fn build_plan(request: &FormatRequest, available: &[FsType]) -> Result<FormatPlan, PlanError> {
    let disk = &request.disk;
    if disk.name().is_empty() {
        return Err(PlanError::InvalidDisk);
    }
    if disk.is_system() {
        return Err(PlanError::SystemDisk(disk.display_name()));
    }
    if !available.contains(&request.fs) {
        return Err(PlanError::FsUnavailable(request.fs));
    }
    request.fs.validate_label(&request.label)?;
    if disk.size < request.fs.min_size() {
        return Err(PlanError::DiskTooSmall(request.fs));
    }
    if request
        .table
        .max_disk_size()
        .is_some_and(|max| disk.size > max)
    {
        return Err(PlanError::DiskTooLargeForMbr);
    }

    let unmounts = disk
        .mounted_partitions()
        .into_iter()
        .map(|p| Step::Unmount {
            device: p.device.clone(),
        });
    let steps = unmounts
        .chain([
            Step::CreateTable {
                kind: request.table,
                erase: request.erase,
            },
            Step::CreatePartition {
                fs: request.fs,
                label: request.label.clone(),
                table: request.table,
            },
        ])
        .collect();
    Ok(FormatPlan {
        disk: disk.clone(),
        steps,
    })
}

impl FormatPlan {
    pub fn disk(&self) -> &Disk {
        &self.disk
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// The text the user must type to confirm: the kernel name of the disk.
    pub fn confirmation_phrase(&self) -> &str {
        self.disk.name()
    }

    /// Succeeds only if `typed` is exactly the disk's kernel name.
    pub fn confirm(self, typed: &str) -> Result<ConfirmedPlan, PlanError> {
        if typed != self.confirmation_phrase() {
            return Err(PlanError::ConfirmationMismatch);
        }
        Ok(ConfirmedPlan {
            disk: self.disk,
            steps: self.steps,
        })
    }
}

impl ConfirmedPlan {
    pub fn disk(&self) -> &Disk {
        &self.disk
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::fixtures::*;

    const ALL: [FsType; 7] = FsType::ALL;

    fn request(disk: Disk) -> FormatRequest {
        FormatRequest {
            disk,
            fs: FsType::Exfat,
            table: TableKind::Gpt,
            label: "KEY".into(),
            erase: EraseMode::Quick,
        }
    }

    #[test]
    fn erase_mode_ids_round_trip() {
        for mode in [EraseMode::Quick, EraseMode::Zero] {
            assert_eq!(EraseMode::from_id(mode.id()), Some(mode));
        }
        assert_eq!(EraseMode::from_id("shred"), None);
    }

    #[test]
    fn system_disk_is_refused() {
        let err = build_plan(&request(system_disk()), &ALL).unwrap_err();
        assert!(matches!(err, PlanError::SystemDisk(_)));
    }

    #[test]
    fn disk_without_a_device_name_is_refused() {
        let mut disk = usb_stick();
        disk.device = "/dev/".into();
        assert_eq!(
            build_plan(&request(disk), &ALL).unwrap_err(),
            PlanError::InvalidDisk
        );
    }

    #[test]
    fn disk_with_root_on_a_stacked_device_is_refused() {
        let mut disk = usb_stick();
        disk.holder_mounts = vec!["/".into()];
        assert!(matches!(
            build_plan(&request(disk), &ALL).unwrap_err(),
            PlanError::SystemDisk(_)
        ));
    }

    #[test]
    fn unavailable_filesystem_is_refused() {
        let req = FormatRequest {
            fs: FsType::Ntfs,
            ..request(usb_stick())
        };
        let available: Vec<_> = ALL.into_iter().filter(|f| *f != FsType::Ntfs).collect();
        assert_eq!(
            build_plan(&req, &available).unwrap_err(),
            PlanError::FsUnavailable(FsType::Ntfs)
        );
    }

    #[test]
    fn invalid_label_is_refused() {
        let req = FormatRequest {
            label: "x".repeat(40),
            ..request(usb_stick())
        };
        assert!(matches!(
            build_plan(&req, &ALL).unwrap_err(),
            PlanError::InvalidLabel(LabelError::TooLong { .. })
        ));
    }

    #[test]
    fn disk_below_filesystem_minimum_is_refused() {
        let mut disk = usb_stick();
        disk.size = 100 * 1024 * 1024;
        let req = FormatRequest {
            fs: FsType::Xfs,
            ..request(disk)
        };
        assert_eq!(
            build_plan(&req, &ALL).unwrap_err(),
            PlanError::DiskTooSmall(FsType::Xfs)
        );
    }

    #[test]
    fn mbr_on_a_four_tebibyte_disk_is_refused() {
        let mut disk = usb_stick();
        disk.size = 4 << 40;
        let req = FormatRequest {
            table: TableKind::Mbr,
            ..request(disk)
        };
        assert_eq!(
            build_plan(&req, &ALL).unwrap_err(),
            PlanError::DiskTooLargeForMbr
        );
    }

    #[test]
    fn plan_unmounts_each_mounted_partition_before_formatting() {
        let mut disk = usb_stick();
        disk.partitions = vec![
            partition("/dev/sdb1", &["/run/media/me/A"]),
            partition("/dev/sdb2", &[]),
            partition("/dev/sdb3", &["/run/media/me/C"]),
        ];
        let plan = build_plan(&request(disk), &ALL).unwrap();
        assert_eq!(
            plan.steps(),
            &[
                Step::Unmount {
                    device: "/dev/sdb1".into()
                },
                Step::Unmount {
                    device: "/dev/sdb3".into()
                },
                Step::CreateTable {
                    kind: TableKind::Gpt,
                    erase: EraseMode::Quick
                },
                Step::CreatePartition {
                    fs: FsType::Exfat,
                    label: "KEY".into(),
                    table: TableKind::Gpt
                },
            ]
        );
    }

    #[test]
    fn confirmation_requires_the_exact_disk_name() {
        let plan = build_plan(&request(usb_stick()), &ALL).unwrap();
        assert_eq!(plan.confirmation_phrase(), "sdb");
        assert_eq!(
            plan.clone().confirm("sdc").unwrap_err(),
            PlanError::ConfirmationMismatch
        );
        assert_eq!(
            plan.confirm(" sdb ").unwrap_err(),
            PlanError::ConfirmationMismatch
        );
    }

    #[test]
    fn exact_confirmation_yields_an_executable_plan() {
        let plan = build_plan(&request(usb_stick()), &ALL).unwrap();
        let steps = plan.steps().to_vec();
        let confirmed = plan.confirm("sdb").unwrap();
        assert_eq!(confirmed.steps(), steps.as_slice());
        assert_eq!(confirmed.disk().device, "/dev/sdb");
    }
}
