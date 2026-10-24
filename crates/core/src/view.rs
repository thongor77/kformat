//! Presentation data for the UI — sizes, labels and messages, in the user's language.
use serde::Serialize;

use crate::backend::BackendError;
use crate::fs::{FsType, LabelError};
use crate::i18n::Lang;
use crate::messages::{tr, Msg};
use crate::model::{Disk, Partition};
use crate::plan::{EraseMode, FormatPlan, PlanError, Step};
use crate::preset::{Preset, TableKind};
use crate::session::SessionError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartitionView {
    pub device: String,
    pub label: String,
    pub fs_type: String,
    pub size_text: String,
    pub mounted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskView {
    pub device: String,
    pub name: String,
    pub title: String,
    pub size_text: String,
    pub removable: bool,
    pub selectable: bool,
    pub blocked_reason: String,
    pub partitions: Vec<PartitionView>,
}

/// Decimal size with the language's unit names and decimal mark: `32 Go`, `1.5 GB`.
pub fn human_size(lang: Lang, bytes: u64) -> String {
    let [byte_unit, mb, gb, tb] = lang.size_units();
    let units = [
        (tb, 1_000_000_000_000),
        (gb, 1_000_000_000),
        (mb, 1_000_000),
    ];
    let Some(&(unit, factor)) = units.iter().find(|(_, factor)| bytes >= *factor) else {
        return format!("{bytes} {byte_unit}");
    };
    // Tenths, rounded: 1_550_000_000 B -> 16 tenths of a GB.
    let tenths = (bytes * 10 + factor / 2) / factor;
    match (tenths / 10, tenths % 10) {
        (whole, 0) => format!("{whole} {unit}"),
        (whole, tenth) => format!("{whole}{}{tenth} {unit}", lang.decimal_separator()),
    }
}

impl PartitionView {
    fn new(lang: Lang, partition: &Partition) -> Self {
        Self {
            device: partition.device.clone(),
            label: partition.label.clone(),
            fs_type: partition.fs_type.clone(),
            size_text: human_size(lang, partition.size),
            mounted: !partition.mount_points.is_empty(),
        }
    }
}

impl DiskView {
    pub fn new(lang: Lang, disk: &Disk) -> Self {
        let blocked = disk.is_system();
        Self {
            device: disk.device.clone(),
            name: disk.name().to_owned(),
            title: disk.display_name(),
            size_text: human_size(lang, disk.size),
            removable: disk.removable,
            selectable: !blocked,
            blocked_reason: if blocked {
                tr(lang, Msg::SystemDiskReason, &[])
            } else {
                String::new()
            },
            partitions: disk
                .partitions
                .iter()
                .map(|p| PartitionView::new(lang, p))
                .collect(),
        }
    }
}

/// Removable disks first, system disks last, then by device name.
pub fn disks_to_json(lang: Lang, disks: &[Disk]) -> String {
    let mut views: Vec<DiskView> = disks.iter().map(|d| DiskView::new(lang, d)).collect();
    views.sort_by(|a, b| {
        (!a.selectable, !a.removable, &a.device).cmp(&(!b.selectable, !b.removable, &b.device))
    });
    // Serializing plain structs of strings and bools cannot fail.
    serde_json::to_string(&views).unwrap_or_else(|_| "[]".to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetView {
    pub id: String,
    pub title: String,
    pub description: String,
    pub fs: String,
    pub table: String,
    pub available: bool,
    pub unavailable_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsView {
    pub id: String,
    pub name: String,
    pub available: bool,
    pub unavailable_reason: String,
    pub max_label_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableView {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionsView {
    pub presets: Vec<PresetView>,
    pub filesystems: Vec<FsView>,
    pub tables: Vec<TableView>,
}

fn unavailable_reason(lang: Lang, fs: FsType) -> String {
    tr(
        lang,
        Msg::FsUnavailable,
        &[fs.mkfs_binary(), fs.package(), fs.display_name()],
    )
}

fn table_name(lang: Lang, kind: TableKind) -> String {
    let msg = match kind {
        TableKind::Gpt => Msg::TableGptName,
        TableKind::Mbr => Msg::TableMbrName,
    };
    tr(lang, msg, &[])
}

/// Everything the "choose a format" page needs, given the installed `mkfs` tools.
pub fn options_json(lang: Lang, available: &[FsType]) -> String {
    let reason = |fs: FsType| {
        if available.contains(&fs) {
            String::new()
        } else {
            unavailable_reason(lang, fs)
        }
    };
    let options = OptionsView {
        presets: Preset::ALL
            .iter()
            .map(|preset| PresetView {
                id: preset.id().to_owned(),
                title: tr(lang, preset.title_msg(), &[]),
                description: tr(lang, preset.description_msg(), &[]),
                fs: preset.fs().udisks_name().to_owned(),
                table: preset.table().id().to_owned(),
                available: available.contains(&preset.fs()),
                unavailable_reason: reason(preset.fs()),
            })
            .collect(),
        filesystems: FsType::ALL
            .iter()
            .map(|&fs| FsView {
                id: fs.udisks_name().to_owned(),
                name: fs.display_name().to_owned(),
                available: available.contains(&fs),
                unavailable_reason: reason(fs),
                max_label_len: fs.max_label_len(),
            })
            .collect(),
        tables: TableKind::ALL
            .iter()
            .map(|&kind| TableView {
                id: kind.id().to_owned(),
                name: table_name(lang, kind),
            })
            .collect(),
    };
    // Serializing plain structs of strings and bools cannot fail.
    serde_json::to_string(&options).unwrap_or_else(|_| "{}".to_owned())
}

fn label_error_text(lang: Lang, fs: Option<FsType>, error: &LabelError) -> String {
    let for_fs = fs
        .map(|fs| tr(lang, Msg::LabelForFs, &[fs.display_name()]))
        .unwrap_or_default();
    match error {
        LabelError::TooLong { max } => tr(lang, Msg::LabelTooLong, &[&max.to_string(), &for_fs]),
        LabelError::ForbiddenChar(c) => tr(
            lang,
            Msg::LabelForbiddenChar,
            &[&c.escape_debug().to_string(), &for_fs],
        ),
    }
}

/// Message for an invalid label, or an empty string when it is valid.
pub fn label_message(lang: Lang, fs_id: &str, label: &str) -> String {
    let Some(fs) = FsType::from_udisks_name(fs_id) else {
        return tr(lang, Msg::UnknownFs, &[]);
    };
    match fs.validate_label(label) {
        Ok(()) => String::new(),
        Err(err) => label_error_text(lang, Some(fs), &err),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanView {
    pub device: String,
    pub name: String,
    pub title: String,
    pub size_text: String,
    pub confirmation_phrase: String,
    pub fs_name: String,
    pub table_name: String,
    pub label: String,
    pub erase_text: String,
    /// False when the system could not be told to stay awake during the run.
    pub sleep_protected: bool,
    pub will_erase: Vec<PartitionView>,
    pub steps: Vec<String>,
}

fn table_short_name(kind: TableKind) -> &'static str {
    match kind {
        TableKind::Gpt => "GPT",
        TableKind::Mbr => "MBR",
    }
}

/// One-line description of a step, shown in the summary and while running.
pub fn step_text(lang: Lang, step: &Step) -> String {
    match step {
        Step::Unmount { device } => tr(lang, Msg::StepUnmount, &[device]),
        Step::CreateTable { kind, erase } => {
            let msg = match erase {
                EraseMode::Quick => Msg::StepTableQuick,
                EraseMode::Zero => Msg::StepTableZero,
            };
            tr(lang, msg, &[table_short_name(*kind)])
        }
        Step::CreatePartition { fs, .. } => tr(lang, Msg::StepPartition, &[fs.display_name()]),
    }
}

fn erase_text(lang: Lang, erase: EraseMode) -> String {
    let msg = match erase {
        EraseMode::Quick => Msg::EraseQuickText,
        EraseMode::Zero => Msg::EraseZeroText,
    };
    tr(lang, msg, &[])
}

/// What the confirmation screen shows: the disk, its contents and the steps.
pub fn plan_json(lang: Lang, plan: &FormatPlan, sleep_protected: bool) -> String {
    let disk = plan.disk();
    let (mut fs_name, mut label, mut table_name, mut erase) = (
        String::new(),
        String::new(),
        String::new(),
        EraseMode::Quick,
    );
    for step in plan.steps() {
        match step {
            Step::CreateTable { kind, erase: mode } => {
                table_name = table_short_name(*kind).to_owned();
                erase = *mode;
            }
            Step::CreatePartition {
                fs: chosen,
                label: text,
                ..
            } => {
                fs_name = chosen.display_name().to_owned();
                label = text.clone();
            }
            Step::Unmount { .. } => {}
        }
    }
    let view = PlanView {
        device: disk.device.clone(),
        name: disk.name().to_owned(),
        title: disk.display_name(),
        size_text: human_size(lang, disk.size),
        confirmation_phrase: plan.confirmation_phrase().to_owned(),
        fs_name,
        table_name,
        label,
        erase_text: erase_text(lang, erase),
        sleep_protected,
        will_erase: disk
            .partitions
            .iter()
            .map(|p| PartitionView::new(lang, p))
            .collect(),
        steps: plan.steps().iter().map(|s| step_text(lang, s)).collect(),
    };
    // Serializing plain structs of strings and bools cannot fail.
    serde_json::to_string(&view).unwrap_or_else(|_| "{}".to_owned())
}

pub fn plan_error_message(lang: Lang, error: &PlanError) -> String {
    match error {
        PlanError::SystemDisk(name) => tr(lang, Msg::PlanSystemDisk, &[name]),
        PlanError::FsUnavailable(fs) => unavailable_reason(lang, *fs),
        PlanError::InvalidLabel(err) => label_error_text(lang, None, err),
        PlanError::DiskTooSmall(fs) => tr(lang, Msg::PlanDiskTooSmall, &[fs.display_name()]),
        PlanError::DiskTooLargeForMbr => tr(lang, Msg::PlanMbrTooLarge, &[]),
        PlanError::InvalidDisk => tr(lang, Msg::PlanInvalidDisk, &[]),
        PlanError::ConfirmationMismatch => tr(lang, Msg::PlanConfirmationMismatch, &[]),
    }
}

pub fn backend_error_message(lang: Lang, error: &BackendError) -> String {
    match error {
        BackendError::Bus(reason) => tr(lang, Msg::BackendBus, &[reason]),
        BackendError::StepFailed {
            step,
            total,
            reason,
        } => {
            let partial = if *step > 1 {
                tr(lang, Msg::BackendPartialWarning, &[])
            } else {
                String::new()
            };
            tr(
                lang,
                Msg::BackendStepFailed,
                &[&step.to_string(), &total.to_string(), reason, &partial],
            )
        }
        BackendError::DiskGone(device) => tr(lang, Msg::BackendDiskGone, &[device]),
        BackendError::DiskBecameSystem(device) => tr(lang, Msg::BackendDiskBecameSystem, &[device]),
        BackendError::DiskChanged(device) => tr(lang, Msg::BackendDiskChanged, &[device]),
        BackendError::Interrupted {
            completed,
            total,
            cause,
        } => tr(
            lang,
            Msg::BackendInterrupted,
            &[
                &backend_error_message(lang, cause),
                &completed.to_string(),
                &total.to_string(),
            ],
        ),
        BackendError::Parse { what, reason } => tr(lang, Msg::BackendParse, &[what, reason]),
    }
}

pub fn session_error_message(lang: Lang, error: &SessionError) -> String {
    match error {
        SessionError::UnknownOption(option) => tr(lang, Msg::SessionUnknownOption, &[option]),
        SessionError::DiskNotFound(device) => tr(lang, Msg::SessionDiskNotFound, &[device]),
        SessionError::Backend(err) => backend_error_message(lang, err),
        SessionError::Plan(err) => plan_error_message(lang, err),
    }
}

/// Message for a rejected `--device` value (see `cli::parse_device_arg`).
pub fn device_arg_error_message(lang: Lang, value: &str) -> String {
    tr(lang, Msg::CliInvalidDevice, &[value])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::fixtures::*;

    #[test]
    fn sizes_use_decimal_french_units() {
        assert_eq!(human_size(Lang::Fr, 32_000_000_000), "32 Go");
        assert_eq!(human_size(Lang::Fr, 1_000_000_000_000), "1 To");
        assert_eq!(human_size(Lang::Fr, 512_000_000), "512 Mo");
        assert_eq!(human_size(Lang::Fr, 1_500_000_000), "1,5 Go");
        assert_eq!(human_size(Lang::Fr, 900), "900 o");
    }

    #[test]
    fn usb_stick_view_is_selectable_and_titled_with_its_model() {
        let view = DiskView::new(Lang::Fr, &usb_stick());
        assert_eq!(view.title, "SanDisk Ultra");
        assert_eq!(view.name, "sdb");
        assert_eq!(view.size_text, "32 Go");
        assert!(view.selectable);
        assert!(view.blocked_reason.is_empty());
    }

    #[test]
    fn system_disk_view_is_blocked_with_a_reason() {
        let view = DiskView::new(Lang::Fr, &system_disk());
        assert!(!view.selectable);
        assert_eq!(
            view.blocked_reason,
            tr(Lang::Fr, Msg::SystemDiskReason, &[])
        );
    }

    #[test]
    fn partition_view_reports_whether_it_is_mounted() {
        let mut disk = usb_stick();
        disk.partitions = vec![partition("/dev/sdb1", &["/run/media/me/KEY"])];
        let view = DiskView::new(Lang::Fr, &disk);
        assert!(view.partitions[0].mounted);
        assert_eq!(view.partitions[0].size_text, "1,1 Go");
    }

    #[test]
    fn json_lists_removable_first_and_system_last_in_camel_case() {
        let json = disks_to_json(Lang::Fr, &[system_disk(), usb_stick()]);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[0]["device"], "/dev/sdb");
        assert_eq!(parsed[1]["device"], "/dev/nvme0n1");
        assert_eq!(parsed[0]["sizeText"], "32 Go");
        assert_eq!(parsed[1]["selectable"], false);
    }

    #[test]
    fn empty_list_is_an_empty_json_array() {
        assert_eq!(disks_to_json(Lang::Fr, &[]), "[]");
    }

    fn without_ntfs() -> Vec<FsType> {
        FsType::ALL
            .into_iter()
            .filter(|f| *f != FsType::Ntfs)
            .collect()
    }

    fn options(available: &[FsType]) -> serde_json::Value {
        serde_json::from_str(&options_json(Lang::Fr, available)).unwrap()
    }

    #[test]
    fn options_list_every_preset_with_its_file_system_and_table() {
        let json = options(&FsType::ALL);
        let presets = json["presets"].as_array().unwrap();
        assert_eq!(presets.len(), 4);
        assert_eq!(presets[0]["id"], "universal-usb");
        assert_eq!(presets[0]["fs"], "exfat");
        assert_eq!(presets[0]["table"], "gpt");
        assert_eq!(presets[3]["table"], "mbr");
        assert!(presets.iter().all(|p| p["available"] == true));
    }

    #[test]
    fn missing_mkfs_disables_the_preset_and_the_file_system_with_the_package_name() {
        let json = options(&without_ntfs());
        let windows = &json["presets"][2];
        assert_eq!(windows["id"], "windows");
        assert_eq!(windows["available"], false);
        let reason = windows["unavailableReason"].as_str().unwrap();
        assert!(
            reason.contains("mkfs.ntfs") && reason.contains("ntfs-3g"),
            "{reason}"
        );

        let ntfs = json["filesystems"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["id"] == "ntfs")
            .unwrap();
        assert_eq!(ntfs["available"], false);
        assert_eq!(ntfs["maxLabelLen"], 128);
    }

    #[test]
    fn options_offer_both_partition_tables() {
        let json = options(&FsType::ALL);
        let ids: Vec<_> = json["tables"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["id"].clone())
            .collect();
        assert_eq!(ids, vec!["gpt", "mbr"]);
    }

    #[test]
    fn valid_label_has_no_message() {
        assert_eq!(label_message(Lang::Fr, "ext4", "DATA"), "");
        assert_eq!(label_message(Lang::Fr, "ext4", ""), "");
    }

    #[test]
    fn too_long_label_message_names_the_limit_and_the_file_system() {
        let message = label_message(Lang::Fr, "exfat", &"x".repeat(16));
        assert!(
            message.contains("15") && message.contains("exFAT"),
            "{message}"
        );
    }

    #[test]
    fn forbidden_character_message_names_the_character() {
        assert!(label_message(Lang::Fr, "vfat", "A.B").contains('.'));
    }

    #[test]
    fn unknown_file_system_has_a_message() {
        assert!(!label_message(Lang::Fr, "zfs", "x").is_empty());
    }

    fn plan(erase: EraseMode) -> FormatPlan {
        use crate::plan::{build_plan, FormatRequest};
        let request = FormatRequest {
            disk: usb_stick(),
            fs: FsType::Exfat,
            table: TableKind::Gpt,
            label: "KEY".into(),
            erase,
        };
        build_plan(&request, &FsType::ALL).unwrap()
    }

    fn plan_value(erase: EraseMode) -> serde_json::Value {
        serde_json::from_str(&plan_json(Lang::Fr, &plan(erase), true)).unwrap()
    }

    #[test]
    fn plan_json_describes_the_disk_and_the_phrase_to_type() {
        let json = plan_value(EraseMode::Quick);
        assert_eq!(json["name"], "sdb");
        assert_eq!(json["confirmationPhrase"], "sdb");
        assert_eq!(json["title"], "SanDisk Ultra");
        assert_eq!(json["sizeText"], "32 Go");
        assert_eq!(json["fsName"], "exFAT");
        assert_eq!(json["label"], "KEY");
        assert!(json["tableName"].as_str().unwrap().contains("GPT"));
    }

    #[test]
    fn plan_json_lists_what_will_be_erased_and_every_step() {
        let json = plan_value(EraseMode::Quick);
        assert_eq!(json["willErase"].as_array().unwrap().len(), 1);
        assert_eq!(json["willErase"][0]["device"], "/dev/sdb1");
        let steps: Vec<_> = json["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(steps.len(), 2);
        assert!(steps[0].contains("GPT"), "{steps:?}");
        assert!(steps[1].contains("exFAT"), "{steps:?}");
    }

    #[test]
    fn plan_json_reports_whether_sleep_is_blocked_during_the_run() {
        let value = |protected: bool| -> serde_json::Value {
            serde_json::from_str(&plan_json(Lang::Fr, &plan(EraseMode::Quick), protected)).unwrap()
        };
        assert_eq!(value(true)["sleepProtected"], true);
        assert_eq!(value(false)["sleepProtected"], false);
    }

    #[test]
    fn zero_erase_is_called_out_in_the_summary() {
        let quick = plan_value(EraseMode::Quick)["eraseText"]
            .as_str()
            .unwrap()
            .to_owned();
        let zero = plan_value(EraseMode::Zero)["eraseText"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(zero.contains("zéros"), "{zero}");
        assert_ne!(quick, zero);
    }

    #[test]
    fn unmount_step_names_the_partition() {
        let text = step_text(
            Lang::Fr,
            &Step::Unmount {
                device: "/dev/sdb1".into(),
            },
        );
        assert!(text.contains("/dev/sdb1"), "{text}");
    }

    #[test]
    fn plan_errors_are_explained_in_french() {
        assert!(
            plan_error_message(Lang::Fr, &PlanError::SystemDisk("Samsung".into()))
                .contains("système")
        );
        assert!(
            plan_error_message(Lang::Fr, &PlanError::ConfirmationMismatch)
                .contains("ne correspond pas")
        );
        assert!(plan_error_message(Lang::Fr, &PlanError::DiskTooLargeForMbr).contains("GPT"));
        assert!(
            plan_error_message(Lang::Fr, &PlanError::DiskTooSmall(FsType::Xfs)).contains("XFS")
        );
        assert!(
            plan_error_message(Lang::Fr, &PlanError::FsUnavailable(FsType::Ntfs))
                .contains("ntfs-3g")
        );
        assert!(plan_error_message(
            Lang::Fr,
            &PlanError::InvalidLabel(LabelError::TooLong { max: 11 })
        )
        .contains("11"));
    }

    #[test]
    fn backend_errors_are_explained_in_french() {
        let failed = BackendError::StepFailed {
            step: 2,
            total: 3,
            reason: "busy".into(),
        };
        let text = backend_error_message(Lang::Fr, &failed);
        assert!(
            text.contains('2') && text.contains('3') && text.contains("busy"),
            "{text}"
        );
        assert!(
            backend_error_message(Lang::Fr, &BackendError::DiskChanged("/dev/sdb".into()))
                .contains("confirmé")
        );
        assert!(
            backend_error_message(Lang::Fr, &BackendError::DiskGone("/dev/sdb".into()))
                .contains("/dev/sdb")
        );
        assert!(backend_error_message(
            Lang::Fr,
            &BackendError::DiskBecameSystem("/dev/sdb".into())
        )
        .contains("système"));
    }

    #[test]
    fn a_failure_after_the_first_step_warns_the_disk_may_be_incomplete() {
        let late = BackendError::StepFailed {
            step: 2,
            total: 3,
            reason: "busy".into(),
        };
        assert!(backend_error_message(Lang::Fr, &late).contains("incomplet"));
        let first = BackendError::StepFailed {
            step: 1,
            total: 3,
            reason: "busy".into(),
        };
        assert!(!backend_error_message(Lang::Fr, &first).contains("incomplet"));
    }

    #[test]
    fn an_interrupted_run_explains_the_cause_and_warns_about_the_disk_state() {
        let interrupted = BackendError::Interrupted {
            completed: 2,
            total: 3,
            cause: Box::new(BackendError::DiskChanged("/dev/sda".into())),
        };
        let text = backend_error_message(Lang::Fr, &interrupted);
        assert!(text.contains("/dev/sda"), "{text}");
        assert!(text.contains('2') && text.contains('3'), "{text}");
        assert!(text.contains("incomplet"), "{text}");
    }

    #[test]
    fn session_errors_delegate_to_the_specific_messages() {
        let err = SessionError::Plan(PlanError::ConfirmationMismatch);
        assert_eq!(
            session_error_message(Lang::Fr, &err),
            plan_error_message(Lang::Fr, &PlanError::ConfirmationMismatch)
        );
        assert!(
            session_error_message(Lang::Fr, &SessionError::DiskNotFound("/dev/sdz".into()))
                .contains("/dev/sdz")
        );
        assert!(
            session_error_message(Lang::Fr, &SessionError::UnknownOption("x".into())).contains('x')
        );
    }

    #[test]
    fn sizes_follow_the_language_units_and_decimal_mark() {
        assert_eq!(human_size(Lang::En, 1_500_000_000), "1.5 GB");
        assert_eq!(human_size(Lang::Es, 1_500_000_000), "1,5 GB");
        assert_eq!(human_size(Lang::Fr, 1_500_000_000), "1,5 Go");
        assert_eq!(human_size(Lang::En, 900), "900 B");
        assert_eq!(human_size(Lang::En, 32_000_000_000), "32 GB");
    }

    #[test]
    fn the_system_disk_reason_is_translated() {
        let reason = |lang| DiskView::new(lang, &system_disk()).blocked_reason;
        assert_eq!(reason(Lang::En), "This disk holds the running system.");
        assert_eq!(
            reason(Lang::Es),
            "Este disco contiene el sistema en ejecución."
        );
        assert!(reason(Lang::Fr).starts_with("Ce disque"));
    }

    #[test]
    fn presets_and_tables_are_translated() {
        let title = |lang: Lang| {
            let json: serde_json::Value =
                serde_json::from_str(&options_json(lang, &FsType::ALL)).unwrap();
            (
                json["presets"][0]["title"].as_str().unwrap().to_owned(),
                json["tables"][0]["name"].as_str().unwrap().to_owned(),
            )
        };
        assert_eq!(
            title(Lang::En),
            ("Universal USB stick".into(), "GPT (recommended)".into())
        );
        assert_eq!(
            title(Lang::Fr),
            ("Clé USB universelle".into(), "GPT (recommandé)".into())
        );
        assert_eq!(
            title(Lang::Es),
            ("Memoria USB universal".into(), "GPT (recomendado)".into())
        );
    }

    #[test]
    fn a_missing_tool_names_the_package_in_every_language() {
        for lang in Lang::ALL {
            let json: serde_json::Value =
                serde_json::from_str(&options_json(lang, &without_ntfs())).unwrap();
            let reason = json["presets"][2]["unavailableReason"]
                .as_str()
                .unwrap()
                .to_owned();
            assert!(
                reason.contains("mkfs.ntfs") && reason.contains("ntfs-3g"),
                "{lang:?}: {reason}"
            );
        }
    }

    #[test]
    fn label_messages_are_translated() {
        let long = "x".repeat(16);
        assert_eq!(
            label_message(Lang::En, "exfat", &long),
            "The name cannot be longer than 15 characters for exFAT."
        );
        assert_eq!(
            label_message(Lang::Es, "exfat", &long),
            "El nombre no puede superar los 15 caracteres para exFAT."
        );
        assert!(label_message(Lang::Es, "vfat", "A.B").contains("«.»"));
        assert!(label_message(Lang::En, "vfat", "A.B").contains("“.”"));
        assert_eq!(label_message(Lang::En, "zfs", "x"), "Unknown file system.");
    }

    #[test]
    fn steps_and_errors_are_translated() {
        let unmount = Step::Unmount {
            device: "/dev/sda1".into(),
        };
        assert_eq!(step_text(Lang::En, &unmount), "Unmounting /dev/sda1");
        assert_eq!(step_text(Lang::Es, &unmount), "Desmontando /dev/sda1");
        assert_eq!(
            plan_error_message(Lang::En, &PlanError::ConfirmationMismatch),
            "The name you typed does not match the disk."
        );
        assert_eq!(
            plan_error_message(Lang::Es, &PlanError::ConfirmationMismatch),
            "El nombre escrito no coincide con el disco."
        );
        let failed = BackendError::StepFailed {
            step: 2,
            total: 3,
            reason: "busy".into(),
        };
        assert!(backend_error_message(Lang::En, &failed).contains("incomplete state"));
        assert!(backend_error_message(Lang::Es, &failed).contains("incompleto"));
    }

    #[test]
    fn an_interrupted_run_is_translated_with_its_cause() {
        let interrupted = BackendError::Interrupted {
            completed: 1,
            total: 3,
            cause: Box::new(BackendError::DiskGone("/dev/sda".into())),
        };
        let en = backend_error_message(Lang::En, &interrupted);
        assert!(en.starts_with("/dev/sda is no longer connected."), "{en}");
        assert!(en.contains("1 of 3"), "{en}");
        let es = backend_error_message(Lang::Es, &interrupted);
        assert!(es.starts_with("/dev/sda ya no está conectado."), "{es}");
    }

    #[test]
    fn device_argument_errors_are_translated() {
        assert!(device_arg_error_message(Lang::En, "../x").contains("Invalid device"));
        assert!(device_arg_error_message(Lang::Fr, "../x").contains("Périphérique invalide"));
        assert!(device_arg_error_message(Lang::Es, "../x").contains("no válido"));
    }

    #[test]
    fn the_plan_summary_is_translated() {
        let json = |lang| -> serde_json::Value {
            serde_json::from_str(&plan_json(lang, &plan(EraseMode::Zero), true)).unwrap()
        };
        assert!(json(Lang::En)["eraseText"]
            .as_str()
            .unwrap()
            .contains("zeros"));
        assert!(json(Lang::Es)["eraseText"]
            .as_str()
            .unwrap()
            .contains("ceros"));
        assert!(json(Lang::Fr)["eraseText"]
            .as_str()
            .unwrap()
            .contains("zéros"));
        assert_eq!(json(Lang::En)["sizeText"], "32 GB");
    }
}
