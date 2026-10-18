//! Presentation data for the UI: sizes, labels and blocking reasons, as JSON.
use serde::Serialize;

use crate::backend::BackendError;
use crate::fs::{FsType, LabelError};
use crate::model::{Disk, Partition};
use crate::plan::{EraseMode, FormatPlan, PlanError, Step};
use crate::preset::{Preset, TableKind};
use crate::session::SessionError;

/// Why a disk cannot be selected.
const SYSTEM_DISK_REASON: &str = "Ce disque contient le système en cours d'exécution.";

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

/// Decimal, French-style size: `32 Go`, `1,5 To`.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [(&str, u64); 3] = [
        ("To", 1_000_000_000_000),
        ("Go", 1_000_000_000),
        ("Mo", 1_000_000),
    ];
    let Some(&(unit, factor)) = UNITS.iter().find(|(_, factor)| bytes >= *factor) else {
        return format!("{bytes} o");
    };
    // Tenths, rounded: 1_550_000_000 B -> 16 tenths of a Go.
    let tenths = (bytes * 10 + factor / 2) / factor;
    match (tenths / 10, tenths % 10) {
        (whole, 0) => format!("{whole} {unit}"),
        (whole, tenth) => format!("{whole},{tenth} {unit}"),
    }
}

impl PartitionView {
    fn new(partition: &Partition) -> Self {
        Self {
            device: partition.device.clone(),
            label: partition.label.clone(),
            fs_type: partition.fs_type.clone(),
            size_text: human_size(partition.size),
            mounted: !partition.mount_points.is_empty(),
        }
    }
}

impl DiskView {
    pub fn new(disk: &Disk) -> Self {
        let blocked = disk.is_system();
        Self {
            device: disk.device.clone(),
            name: disk.name().to_owned(),
            title: disk.display_name(),
            size_text: human_size(disk.size),
            removable: disk.removable,
            selectable: !blocked,
            blocked_reason: if blocked {
                SYSTEM_DISK_REASON.to_owned()
            } else {
                String::new()
            },
            partitions: disk.partitions.iter().map(PartitionView::new).collect(),
        }
    }
}

/// Removable disks first, system disks last, then by device name.
pub fn disks_to_json(disks: &[Disk]) -> String {
    let mut views: Vec<DiskView> = disks.iter().map(DiskView::new).collect();
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

fn unavailable_reason(fs: FsType) -> String {
    format!(
        "{} est introuvable : installez le paquet {} pour utiliser {}.",
        fs.mkfs_binary(),
        fs.package(),
        fs.display_name()
    )
}

fn table_name(kind: TableKind) -> &'static str {
    match kind {
        TableKind::Gpt => "GPT (recommandé)",
        TableKind::Mbr => "MBR (appareils anciens)",
    }
}

/// Everything the "choose a format" page needs, given the installed `mkfs` tools.
pub fn options_json(available: &[FsType]) -> String {
    let reason = |fs: FsType| {
        if available.contains(&fs) {
            String::new()
        } else {
            unavailable_reason(fs)
        }
    };
    let options = OptionsView {
        presets: Preset::ALL
            .iter()
            .map(|preset| PresetView {
                id: preset.id().to_owned(),
                title: preset.title().to_owned(),
                description: preset.description().to_owned(),
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
                name: table_name(kind).to_owned(),
            })
            .collect(),
    };
    // Serializing plain structs of strings and bools cannot fail.
    serde_json::to_string(&options).unwrap_or_else(|_| "{}".to_owned())
}

fn label_error_text(fs: Option<FsType>, error: &LabelError) -> String {
    let for_fs = fs
        .map(|fs| format!(" pour {}", fs.display_name()))
        .unwrap_or_default();
    match error {
        LabelError::TooLong { max } => {
            format!("Le nom ne peut pas dépasser {max} caractères{for_fs}.")
        }
        LabelError::ForbiddenChar(c) => {
            format!(
                "Le caractère « {} » n'est pas autorisé dans ce nom{for_fs}.",
                c.escape_debug()
            )
        }
    }
}

/// French message for an invalid label, or an empty string when it is valid.
pub fn label_message(fs_id: &str, label: &str) -> String {
    let Some(fs) = FsType::from_udisks_name(fs_id) else {
        return "Système de fichiers inconnu.".to_owned();
    };
    match fs.validate_label(label) {
        Ok(()) => String::new(),
        Err(err) => label_error_text(Some(fs), &err),
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
    pub will_erase: Vec<PartitionView>,
    pub steps: Vec<String>,
}

/// One-line French description of a step, shown in the summary and while running.
pub fn step_text(step: &Step) -> String {
    match step {
        Step::Unmount { device } => format!("Démontage de {device}"),
        Step::CreateTable { kind, erase } => {
            let table = table_short_name(*kind);
            match erase {
                EraseMode::Quick => format!("Création de la table de partitions {table}"),
                EraseMode::Zero => {
                    format!("Effacement complet par des zéros, puis création de la table {table}")
                }
            }
        }
        Step::CreatePartition { fs, .. } => {
            format!(
                "Création de la partition et formatage en {}",
                fs.display_name()
            )
        }
    }
}

fn table_short_name(kind: TableKind) -> &'static str {
    match kind {
        TableKind::Gpt => "GPT",
        TableKind::Mbr => "MBR",
    }
}

fn erase_text(erase: EraseMode) -> &'static str {
    match erase {
        EraseMode::Quick => "Effacement rapide : les anciennes données restent récupérables avec des outils spécialisés.",
        EraseMode::Zero => "Effacement complet par des zéros : les anciennes données sont irrécupérables (long).",
    }
}

/// What the confirmation screen shows: the disk, its contents and the steps.
pub fn plan_json(plan: &FormatPlan) -> String {
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
                table_name = table_name_of(*kind);
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
        size_text: human_size(disk.size),
        confirmation_phrase: plan.confirmation_phrase().to_owned(),
        fs_name,
        table_name,
        label,
        erase_text: erase_text(erase).to_owned(),
        will_erase: disk.partitions.iter().map(PartitionView::new).collect(),
        steps: plan.steps().iter().map(step_text).collect(),
    };
    // Serializing plain structs of strings and bools cannot fail.
    serde_json::to_string(&view).unwrap_or_else(|_| "{}".to_owned())
}

fn table_name_of(kind: TableKind) -> String {
    table_short_name(kind).to_owned()
}

pub fn plan_error_message(error: &PlanError) -> String {
    match error {
        PlanError::SystemDisk(name) => {
            format!(
                "{name} contient le système en cours d'exécution : il ne peut pas être formaté."
            )
        }
        PlanError::FsUnavailable(fs) => unavailable_reason(*fs),
        PlanError::InvalidLabel(err) => label_error_text(None, err),
        PlanError::DiskTooSmall(fs) => format!(
            "Ce disque est trop petit pour être formaté en {}.",
            fs.display_name()
        ),
        PlanError::DiskTooLargeForMbr => {
            "Une table MBR ne gère pas un disque de cette taille : choisissez GPT.".to_owned()
        }
        PlanError::InvalidDisk => {
            "Ce disque n'a pas de nom de périphérique exploitable.".to_owned()
        }
        PlanError::ConfirmationMismatch => "Le nom saisi ne correspond pas au disque.".to_owned(),
    }
}

pub fn backend_error_message(error: &BackendError) -> String {
    match error {
        BackendError::Bus(reason) => format!("Impossible de communiquer avec udisks2 : {reason}"),
        BackendError::StepFailed {
            step,
            total,
            reason,
        } => {
            let partial = if *step > 1 {
                " Le disque peut être dans un état incomplet : relancez le formatage."
            } else {
                ""
            };
            format!("L'étape {step} sur {total} a échoué : {reason}.{partial}")
        }
        BackendError::DiskGone(device) => format!("{device} n'est plus connecté."),
        BackendError::DiskBecameSystem(device) => {
            format!("{device} est désormais utilisé par le système : opération annulée.")
        }
        BackendError::DiskChanged(device) => {
            format!("{device} n'est plus le disque que vous avez confirmé : opération annulée.")
        }
        BackendError::Interrupted {
            completed,
            total,
            cause,
        } => format!(
            "{} Le formatage s'est arrêté après {completed} étape(s) sur {total} : le disque peut être dans un état incomplet, relancez le formatage.",
            backend_error_message(cause)
        ),
        BackendError::Parse { what, reason } => {
            format!("Réponse inattendue d'udisks2 ({what}) : {reason}")
        }
    }
}

pub fn session_error_message(error: &SessionError) -> String {
    match error {
        SessionError::UnknownOption(option) => format!("Option inconnue : {option}."),
        SessionError::DiskNotFound(device) => {
            format!("{device} est introuvable. Actualisez la liste des disques.")
        }
        SessionError::Backend(err) => backend_error_message(err),
        SessionError::Plan(err) => plan_error_message(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::fixtures::*;

    #[test]
    fn sizes_use_decimal_french_units() {
        assert_eq!(human_size(32_000_000_000), "32 Go");
        assert_eq!(human_size(1_000_000_000_000), "1 To");
        assert_eq!(human_size(512_000_000), "512 Mo");
        assert_eq!(human_size(1_500_000_000), "1,5 Go");
        assert_eq!(human_size(900), "900 o");
    }

    #[test]
    fn usb_stick_view_is_selectable_and_titled_with_its_model() {
        let view = DiskView::new(&usb_stick());
        assert_eq!(view.title, "SanDisk Ultra");
        assert_eq!(view.name, "sdb");
        assert_eq!(view.size_text, "32 Go");
        assert!(view.selectable);
        assert!(view.blocked_reason.is_empty());
    }

    #[test]
    fn system_disk_view_is_blocked_with_a_reason() {
        let view = DiskView::new(&system_disk());
        assert!(!view.selectable);
        assert_eq!(view.blocked_reason, SYSTEM_DISK_REASON);
    }

    #[test]
    fn partition_view_reports_whether_it_is_mounted() {
        let mut disk = usb_stick();
        disk.partitions = vec![partition("/dev/sdb1", &["/run/media/me/KEY"])];
        let view = DiskView::new(&disk);
        assert!(view.partitions[0].mounted);
        assert_eq!(view.partitions[0].size_text, "1,1 Go");
    }

    #[test]
    fn json_lists_removable_first_and_system_last_in_camel_case() {
        let json = disks_to_json(&[system_disk(), usb_stick()]);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[0]["device"], "/dev/sdb");
        assert_eq!(parsed[1]["device"], "/dev/nvme0n1");
        assert_eq!(parsed[0]["sizeText"], "32 Go");
        assert_eq!(parsed[1]["selectable"], false);
    }

    #[test]
    fn empty_list_is_an_empty_json_array() {
        assert_eq!(disks_to_json(&[]), "[]");
    }

    fn without_ntfs() -> Vec<FsType> {
        FsType::ALL
            .into_iter()
            .filter(|f| *f != FsType::Ntfs)
            .collect()
    }

    fn options(available: &[FsType]) -> serde_json::Value {
        serde_json::from_str(&options_json(available)).unwrap()
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
        assert_eq!(label_message("ext4", "DATA"), "");
        assert_eq!(label_message("ext4", ""), "");
    }

    #[test]
    fn too_long_label_message_names_the_limit_and_the_file_system() {
        let message = label_message("exfat", &"x".repeat(16));
        assert!(
            message.contains("15") && message.contains("exFAT"),
            "{message}"
        );
    }

    #[test]
    fn forbidden_character_message_names_the_character() {
        assert!(label_message("vfat", "A.B").contains('.'));
    }

    #[test]
    fn unknown_file_system_has_a_message() {
        assert!(!label_message("zfs", "x").is_empty());
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
        serde_json::from_str(&plan_json(&plan(erase))).unwrap()
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
        let text = step_text(&Step::Unmount {
            device: "/dev/sdb1".into(),
        });
        assert!(text.contains("/dev/sdb1"), "{text}");
    }

    #[test]
    fn plan_errors_are_explained_in_french() {
        assert!(plan_error_message(&PlanError::SystemDisk("Samsung".into())).contains("système"));
        assert!(plan_error_message(&PlanError::ConfirmationMismatch).contains("ne correspond pas"));
        assert!(plan_error_message(&PlanError::DiskTooLargeForMbr).contains("GPT"));
        assert!(plan_error_message(&PlanError::DiskTooSmall(FsType::Xfs)).contains("XFS"));
        assert!(plan_error_message(&PlanError::FsUnavailable(FsType::Ntfs)).contains("ntfs-3g"));
        assert!(
            plan_error_message(&PlanError::InvalidLabel(LabelError::TooLong { max: 11 }))
                .contains("11")
        );
    }

    #[test]
    fn backend_errors_are_explained_in_french() {
        let failed = BackendError::StepFailed {
            step: 2,
            total: 3,
            reason: "busy".into(),
        };
        let text = backend_error_message(&failed);
        assert!(
            text.contains('2') && text.contains('3') && text.contains("busy"),
            "{text}"
        );
        assert!(
            backend_error_message(&BackendError::DiskChanged("/dev/sdb".into()))
                .contains("confirmé")
        );
        assert!(
            backend_error_message(&BackendError::DiskGone("/dev/sdb".into())).contains("/dev/sdb")
        );
        assert!(
            backend_error_message(&BackendError::DiskBecameSystem("/dev/sdb".into()))
                .contains("système")
        );
    }

    #[test]
    fn a_failure_after_the_first_step_warns_the_disk_may_be_incomplete() {
        let late = BackendError::StepFailed {
            step: 2,
            total: 3,
            reason: "busy".into(),
        };
        assert!(backend_error_message(&late).contains("incomplet"));
        let first = BackendError::StepFailed {
            step: 1,
            total: 3,
            reason: "busy".into(),
        };
        assert!(!backend_error_message(&first).contains("incomplet"));
    }

    #[test]
    fn an_interrupted_run_explains_the_cause_and_warns_about_the_disk_state() {
        let interrupted = BackendError::Interrupted {
            completed: 2,
            total: 3,
            cause: Box::new(BackendError::DiskChanged("/dev/sda".into())),
        };
        let text = backend_error_message(&interrupted);
        assert!(text.contains("/dev/sda"), "{text}");
        assert!(text.contains('2') && text.contains('3'), "{text}");
        assert!(text.contains("incomplet"), "{text}");
    }

    #[test]
    fn session_errors_delegate_to_the_specific_messages() {
        let err = SessionError::Plan(PlanError::ConfirmationMismatch);
        assert_eq!(
            session_error_message(&err),
            plan_error_message(&PlanError::ConfirmationMismatch)
        );
        assert!(
            session_error_message(&SessionError::DiskNotFound("/dev/sdz".into()))
                .contains("/dev/sdz")
        );
        assert!(session_error_message(&SessionError::UnknownOption("x".into())).contains('x'));
    }
}
