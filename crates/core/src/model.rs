/// Only mounts under these prefixes are known to be user data. Anything else
/// (including swap, `/var/lib/…`, `/home/…`) counts as the running system.
const USER_MOUNT_PREFIXES: [&str; 3] = ["/run/media/", "/media/", "/mnt/"];

/// Mount point reported for an active swap area.
pub const SWAP_MOUNT: &str = "[SWAP]";

fn is_user_mount(mount: &str) -> bool {
    USER_MOUNT_PREFIXES
        .iter()
        .any(|prefix| mount.starts_with(prefix))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub device: String,
    pub label: String,
    pub fs_type: String,
    pub size: u64,
    pub mount_points: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disk {
    /// Device node, e.g. `/dev/sdb`.
    pub device: String,
    /// Stable hardware identity (serial/WWN based) used to detect a swapped disk.
    pub id: String,
    pub vendor: String,
    pub model: String,
    pub size: u64,
    pub removable: bool,
    pub partitions: Vec<Partition>,
    /// Mount points of devices stacked on this disk (LUKS, LVM, md…).
    pub holder_mounts: Vec<String>,
}

impl Disk {
    /// Kernel name without the `/dev/` prefix, e.g. `sdb`.
    pub fn name(&self) -> &str {
        self.device.strip_prefix("/dev/").unwrap_or(&self.device)
    }

    pub fn display_name(&self) -> String {
        let text = format!("{} {}", self.vendor.trim(), self.model.trim());
        match text.trim() {
            "" => self.name().to_owned(),
            text => text.to_owned(),
        }
    }

    pub fn mounted_partitions(&self) -> Vec<&Partition> {
        self.partitions
            .iter()
            .filter(|p| !p.mount_points.is_empty())
            .collect()
    }

    /// True when the disk, or anything stacked on it, is used by the running system.
    pub fn is_system(&self) -> bool {
        self.partitions
            .iter()
            .flat_map(|p| p.mount_points.iter())
            .chain(self.holder_mounts.iter())
            .any(|mount| !is_user_mount(mount))
    }

    /// Same physical drive: device node, hardware id and size all match.
    pub fn same_drive(&self, other: &Disk) -> bool {
        self.device == other.device && self.id == other.id && self.size == other.size
    }

    /// Same drive and same partition layout (mount state is ignored).
    pub fn same_media(&self, other: &Disk) -> bool {
        let layout = |d: &Disk| {
            d.partitions
                .iter()
                .map(|p| (p.device.clone(), p.fs_type.clone(), p.label.clone(), p.size))
                .collect::<Vec<_>>()
        };
        self.same_drive(other) && layout(self) == layout(other)
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;

    pub fn partition(device: &str, mounts: &[&str]) -> Partition {
        Partition {
            device: device.into(),
            label: String::new(),
            fs_type: "ext4".into(),
            size: 1 << 30,
            mount_points: mounts.iter().map(|m| m.to_string()).collect(),
        }
    }

    pub fn usb_stick() -> Disk {
        Disk {
            device: "/dev/sdb".into(),
            id: "usb-SanDisk_Ultra-1".into(),
            vendor: "SanDisk".into(),
            model: "Ultra".into(),
            size: 32_000_000_000,
            removable: true,
            partitions: vec![partition("/dev/sdb1", &[])],
            holder_mounts: vec![],
        }
    }

    pub fn system_disk() -> Disk {
        Disk {
            device: "/dev/nvme0n1".into(),
            id: "nvme-SAMSUNG-1".into(),
            vendor: String::new(),
            model: "SAMSUNG SSD".into(),
            size: 1_000_000_000_000,
            removable: false,
            partitions: vec![
                partition("/dev/nvme0n1p1", &["/efi"]),
                partition("/dev/nvme0n1p2", &["/"]),
            ],
            holder_mounts: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;

    #[test]
    fn disk_holding_root_is_a_system_disk() {
        assert!(system_disk().is_system());
    }

    #[test]
    fn unmounted_usb_stick_is_not_a_system_disk() {
        assert!(!usb_stick().is_system());
    }

    #[test]
    fn nested_boot_mount_is_a_system_mount() {
        let mut disk = usb_stick();
        disk.partitions = vec![partition("/dev/sdb1", &["/boot/efi"])];
        assert!(disk.is_system());
    }

    #[test]
    fn mounts_outside_user_prefixes_are_system_mounts() {
        for mount in [
            "/var/lib/docker",
            "/home/me",
            "/opt",
            "/usr/local",
            "/srv",
            "[SWAP]",
            "/nix",
        ] {
            let mut disk = usb_stick();
            disk.partitions = vec![partition("/dev/sdb1", &[mount])];
            assert!(disk.is_system(), "{mount} must count as system");
        }
    }

    #[test]
    fn user_data_mounts_are_not_system_mounts() {
        for mount in ["/run/media/me/KEY", "/media/KEY", "/mnt/data"] {
            let mut disk = usb_stick();
            disk.partitions = vec![partition("/dev/sdb1", &[mount])];
            assert!(!disk.is_system(), "{mount} must not count as system");
        }
    }

    #[test]
    fn root_mounted_on_a_stacked_device_makes_the_disk_a_system_disk() {
        let mut disk = usb_stick();
        disk.holder_mounts = vec!["/".into()];
        assert!(disk.is_system());
    }

    #[test]
    fn mount_state_does_not_change_media_identity() {
        let mut mounted = usb_stick();
        mounted.partitions = vec![partition("/dev/sdb1", &["/run/media/me/KEY"])];
        assert!(usb_stick().same_media(&mounted));
    }

    #[test]
    fn a_different_hardware_id_is_a_different_drive() {
        let mut other = usb_stick();
        other.id = "usb-Kingston_DT-9".into();
        assert!(!usb_stick().same_drive(&other));
    }

    #[test]
    fn a_different_layout_is_different_media() {
        let mut other = usb_stick();
        other.partitions[0].fs_type = "ntfs".into();
        assert!(usb_stick().same_drive(&other));
        assert!(!usb_stick().same_media(&other));
    }

    #[test]
    fn name_strips_dev_prefix() {
        assert_eq!(usb_stick().name(), "sdb");
    }

    #[test]
    fn display_name_falls_back_to_kernel_name() {
        let mut disk = usb_stick();
        disk.vendor.clear();
        disk.model.clear();
        assert_eq!(disk.display_name(), "sdb");
    }
}
