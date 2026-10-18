use thiserror::Error;

const MIB: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FsType {
    Ext4,
    Btrfs,
    Xfs,
    F2fs,
    Exfat,
    Vfat,
    Ntfs,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LabelError {
    #[error("label is longer than {max} characters")]
    TooLong { max: usize },
    #[error("label contains the forbidden character '{0}'")]
    ForbiddenChar(char),
}

const VFAT_FORBIDDEN: &str = "*?.,;:/\\|+=<>[]\"";

impl FsType {
    pub const ALL: [FsType; 7] = [
        FsType::Ext4,
        FsType::Btrfs,
        FsType::Xfs,
        FsType::F2fs,
        FsType::Exfat,
        FsType::Vfat,
        FsType::Ntfs,
    ];

    /// Name understood by udisks2 `Block.Format`.
    pub fn udisks_name(self) -> &'static str {
        match self {
            FsType::Ext4 => "ext4",
            FsType::Btrfs => "btrfs",
            FsType::Xfs => "xfs",
            FsType::F2fs => "f2fs",
            FsType::Exfat => "exfat",
            FsType::Vfat => "vfat",
            FsType::Ntfs => "ntfs",
        }
    }

    pub fn from_udisks_name(name: &str) -> Option<FsType> {
        Self::ALL.into_iter().find(|fs| fs.udisks_name() == name)
    }

    /// Package that usually provides the `mkfs` binary.
    pub fn package(self) -> &'static str {
        match self {
            FsType::Ext4 => "e2fsprogs",
            FsType::Btrfs => "btrfs-progs",
            FsType::Xfs => "xfsprogs",
            FsType::F2fs => "f2fs-tools",
            FsType::Exfat => "exfatprogs",
            FsType::Vfat => "dosfstools",
            FsType::Ntfs => "ntfs-3g",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            FsType::Vfat => "FAT32",
            FsType::Exfat => "exFAT",
            FsType::Ntfs => "NTFS",
            FsType::Ext4 => "ext4",
            FsType::Btrfs => "Btrfs",
            FsType::Xfs => "XFS",
            FsType::F2fs => "F2FS",
        }
    }

    /// Binary udisks2 needs to create this file system.
    pub fn mkfs_binary(self) -> &'static str {
        match self {
            FsType::Ext4 => "mkfs.ext4",
            FsType::Btrfs => "mkfs.btrfs",
            FsType::Xfs => "mkfs.xfs",
            FsType::F2fs => "mkfs.f2fs",
            FsType::Exfat => "mkfs.exfat",
            FsType::Vfat => "mkfs.vfat",
            FsType::Ntfs => "mkfs.ntfs",
        }
    }

    /// MBR partition type id; devices such as TVs and car stereos rely on it.
    pub fn mbr_type(self) -> &'static str {
        match self {
            FsType::Vfat => "0x0c",
            FsType::Exfat | FsType::Ntfs => "0x07",
            _ => "0x83",
        }
    }

    pub fn max_label_len(self) -> usize {
        match self {
            FsType::Ext4 => 16,
            FsType::Btrfs => 255,
            FsType::Xfs => 12,
            FsType::F2fs => 512,
            FsType::Exfat => 15,
            FsType::Vfat => 11,
            FsType::Ntfs => 128,
        }
    }

    /// Smallest partition the file system can be created on.
    pub fn min_size(self) -> u64 {
        match self {
            FsType::Xfs => 300 * MIB,
            FsType::Btrfs => 128 * MIB,
            FsType::F2fs => 64 * MIB,
            _ => 8 * MIB,
        }
    }

    pub fn validate_label(self, label: &str) -> Result<(), LabelError> {
        if label.chars().count() > self.max_label_len() {
            return Err(LabelError::TooLong {
                max: self.max_label_len(),
            });
        }
        let forbidden =
            |c: char| c.is_control() || (self == FsType::Vfat && VFAT_FORBIDDEN.contains(c));
        match label.chars().find(|&c| forbidden(c)) {
            Some(c) => Err(LabelError::ForbiddenChar(c)),
            None => Ok(()),
        }
    }
}

/// File systems whose `mkfs` binary satisfies `has_binary`.
pub fn available_filesystems(has_binary: &dyn Fn(&str) -> bool) -> Vec<FsType> {
    FsType::ALL
        .into_iter()
        .filter(|fs| has_binary(fs.mkfs_binary()))
        .collect()
}

/// Looks for `binary` in the usual system directories and `PATH`.
pub fn binary_in_path(binary: &str) -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .chain(["/usr/sbin", "/sbin"].map(Into::into))
        .any(|dir| dir.join(binary).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vfat_label_rejects_forbidden_characters() {
        assert_eq!(
            FsType::Vfat.validate_label("A.B"),
            Err(LabelError::ForbiddenChar('.'))
        );
    }

    #[test]
    fn label_longer_than_limit_is_rejected() {
        assert_eq!(
            FsType::Exfat.validate_label(&"x".repeat(16)),
            Err(LabelError::TooLong { max: 15 })
        );
    }

    #[test]
    fn label_at_limit_is_accepted() {
        assert!(FsType::Ext4.validate_label(&"x".repeat(16)).is_ok());
    }

    #[test]
    fn control_characters_are_rejected_everywhere() {
        assert!(FsType::Ext4.validate_label("a\nb").is_err());
    }

    #[test]
    fn mbr_partition_types_match_the_file_system() {
        assert_eq!(FsType::Vfat.mbr_type(), "0x0c");
        assert_eq!(FsType::Exfat.mbr_type(), "0x07");
        assert_eq!(FsType::Ntfs.mbr_type(), "0x07");
        assert_eq!(FsType::Ext4.mbr_type(), "0x83");
    }

    #[test]
    fn udisks_names_round_trip() {
        for fs in FsType::ALL {
            assert_eq!(FsType::from_udisks_name(fs.udisks_name()), Some(fs));
        }
        assert_eq!(FsType::from_udisks_name("zfs"), None);
    }

    #[test]
    fn available_filesystems_follow_the_probe() {
        let probe = |bin: &str| bin != "mkfs.ntfs";
        let available = available_filesystems(&probe);
        assert!(!available.contains(&FsType::Ntfs));
        assert_eq!(available.len(), 6);
    }
}
