use crate::fs::FsType;
use crate::messages::Msg;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    Gpt,
    Mbr,
}

impl TableKind {
    pub const ALL: [TableKind; 2] = [TableKind::Gpt, TableKind::Mbr];

    /// Stable identifier shared with the UI.
    pub fn id(self) -> &'static str {
        match self {
            TableKind::Gpt => "gpt",
            TableKind::Mbr => "mbr",
        }
    }

    pub fn from_id(id: &str) -> Option<TableKind> {
        Self::ALL.into_iter().find(|kind| kind.id() == id)
    }

    /// Name understood by udisks2 `Block.Format`.
    pub fn udisks_name(self) -> &'static str {
        match self {
            TableKind::Gpt => "gpt",
            TableKind::Mbr => "dos",
        }
    }

    /// Largest disk the table can address (`None` = no practical limit).
    pub fn max_disk_size(self) -> Option<u64> {
        match self {
            TableKind::Gpt => None,
            TableKind::Mbr => Some(2 * 1024 * 1024 * 1024 * 1024),
        }
    }
}

/// A use case that maps to a sensible file system and partition table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    UniversalUsb,
    Linux,
    Windows,
    LegacyFat32,
}

impl Preset {
    pub const ALL: [Preset; 4] = [
        Preset::UniversalUsb,
        Preset::Linux,
        Preset::Windows,
        Preset::LegacyFat32,
    ];

    /// Stable identifier shared with the UI.
    pub fn id(self) -> &'static str {
        match self {
            Preset::UniversalUsb => "universal-usb",
            Preset::Linux => "linux",
            Preset::Windows => "windows",
            Preset::LegacyFat32 => "legacy-fat32",
        }
    }

    pub fn fs(self) -> FsType {
        match self {
            Preset::UniversalUsb => FsType::Exfat,
            Preset::Linux => FsType::Ext4,
            Preset::Windows => FsType::Ntfs,
            Preset::LegacyFat32 => FsType::Vfat,
        }
    }

    pub fn table(self) -> TableKind {
        match self {
            Preset::LegacyFat32 => TableKind::Mbr,
            _ => TableKind::Gpt,
        }
    }

    pub fn title_msg(self) -> Msg {
        match self {
            Preset::UniversalUsb => Msg::PresetUniversalUsbTitle,
            Preset::Linux => Msg::PresetLinuxTitle,
            Preset::Windows => Msg::PresetWindowsTitle,
            Preset::LegacyFat32 => Msg::PresetLegacyFat32Title,
        }
    }

    pub fn description_msg(self) -> Msg {
        match self {
            Preset::UniversalUsb => Msg::PresetUniversalUsbDescription,
            Preset::Linux => Msg::PresetLinuxDescription,
            Preset::Windows => Msg::PresetWindowsDescription,
            Preset::LegacyFat32 => Msg::PresetLegacyFat32Description,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn universal_usb_is_exfat_on_gpt() {
        assert_eq!(Preset::UniversalUsb.fs(), FsType::Exfat);
        assert_eq!(Preset::UniversalUsb.table(), TableKind::Gpt);
    }

    #[test]
    fn table_ids_round_trip() {
        for kind in TableKind::ALL {
            assert_eq!(TableKind::from_id(kind.id()), Some(kind));
        }
        assert_eq!(TableKind::from_id("dos"), None);
    }

    #[test]
    fn preset_ids_are_unique() {
        let ids: std::collections::HashSet<_> = Preset::ALL.iter().map(|p| p.id()).collect();
        assert_eq!(ids.len(), Preset::ALL.len());
    }

    #[test]
    fn legacy_fat32_uses_mbr() {
        assert_eq!(Preset::LegacyFat32.table(), TableKind::Mbr);
    }

    #[test]
    fn mbr_is_limited_to_two_tebibytes() {
        assert_eq!(TableKind::Mbr.max_disk_size(), Some(2 << 40));
        assert_eq!(TableKind::Gpt.max_disk_size(), None);
    }
}
