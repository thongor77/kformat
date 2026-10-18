//! Real backend: talks to udisks2 over the system bus (blocking zbus).
//!
//! Parsing is strict: anything unreadable is an error, never a silently
//! missing partition or mount point, because a missed mount could make a
//! system disk look safe to format.
use std::collections::{HashMap, HashSet, VecDeque};

use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

use crate::backend::sealed::StepRunner;
use crate::backend::{BackendError, DiskBackend};
use crate::model::{Disk, Partition, SWAP_MOUNT};
use crate::plan::{EraseMode, Step};
use crate::preset::TableKind;

const SERVICE: &str = "org.freedesktop.UDisks2";
const ROOT_PATH: &str = "/org/freedesktop/UDisks2";
const BLOCK_PREFIX: &str = "/org/freedesktop/UDisks2/block_devices/";
const IFACE_BLOCK: &str = "org.freedesktop.UDisks2.Block";
const IFACE_DRIVE: &str = "org.freedesktop.UDisks2.Drive";
const IFACE_PARTITION: &str = "org.freedesktop.UDisks2.Partition";
const IFACE_TABLE: &str = "org.freedesktop.UDisks2.PartitionTable";
const IFACE_FILESYSTEM: &str = "org.freedesktop.UDisks2.Filesystem";
const IFACE_SWAP: &str = "org.freedesktop.UDisks2.Swapspace";
const NO_DRIVE: &str = "/";
/// First partition starts at 1 MiB, the usual alignment.
const FIRST_PARTITION_OFFSET: u64 = 1024 * 1024;

pub(crate) type Props = HashMap<String, OwnedValue>;
pub(crate) type Interfaces = HashMap<String, Props>;
pub(crate) type Objects = HashMap<OwnedObjectPath, Interfaces>;

/// Kernel names of the devices stacked directly on `name` (LUKS, LVM, md…).
pub(crate) type HoldersFn<'a> = &'a dyn Fn(&str) -> Result<Vec<String>, BackendError>;

fn parse_error(what: &str, reason: impl ToString) -> BackendError {
    BackendError::Parse {
        what: what.to_owned(),
        reason: reason.to_string(),
    }
}

/// Optional, cosmetic property: absent or mistyped values become `None`.
fn prop<T: TryFrom<OwnedValue>>(props: &Props, key: &str) -> Option<T> {
    props.get(key)?.try_clone().ok()?.try_into().ok()
}

/// Mandatory property: absent or mistyped values are an error.
fn required<T: TryFrom<OwnedValue>>(props: &Props, key: &str) -> Result<T, BackendError> {
    let value = props.get(key).ok_or_else(|| parse_error(key, "missing"))?;
    let owned = value.try_clone().map_err(|e| parse_error(key, e))?;
    owned
        .try_into()
        .map_err(|_| parse_error(key, "unexpected type"))
}

fn c_string(bytes: Vec<u8>) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn device_of(block: &Props) -> Result<String, BackendError> {
    let device = c_string(required::<Vec<u8>>(block, "Device")?);
    if device.is_empty() {
        return Err(parse_error("Device", "empty"));
    }
    Ok(device)
}

/// Mount points of a block object, plus the swap marker when swap is active.
fn mounts_of(ifaces: &Interfaces) -> Result<Vec<String>, BackendError> {
    let mut mounts: Vec<String> = match ifaces.get(IFACE_FILESYSTEM) {
        Some(fs) => required::<Vec<Vec<u8>>>(fs, "MountPoints")?
            .into_iter()
            .map(c_string)
            .collect(),
        None => vec![],
    };
    if let Some(swap) = ifaces.get(IFACE_SWAP) {
        if required::<bool>(swap, "Active")? {
            mounts.push(SWAP_MOUNT.to_owned());
        }
    }
    Ok(mounts)
}

struct BlockObject<'a> {
    path: &'a OwnedObjectPath,
    block: &'a Props,
    ifaces: &'a Interfaces,
    device: String,
}

fn partition_of(b: &BlockObject) -> Result<Partition, BackendError> {
    Ok(Partition {
        device: b.device.clone(),
        label: prop(b.block, "IdLabel").unwrap_or_default(),
        fs_type: prop(b.block, "IdType").unwrap_or_default(),
        size: required(b.block, "Size")?,
        mount_points: mounts_of(b.ifaces)?,
    })
}

/// Mount points of everything stacked (transitively) on the given devices.
fn stacked_mounts(
    roots: Vec<String>,
    by_device: &HashMap<String, &BlockObject>,
    holders: HoldersFn,
) -> Result<Vec<String>, BackendError> {
    let mut seen: HashSet<String> = roots.iter().cloned().collect();
    let mut queue: VecDeque<String> = roots.into();
    let mut mounts = vec![];
    while let Some(name) = queue.pop_front() {
        for holder in holders(&name)? {
            if !seen.insert(holder.clone()) {
                continue;
            }
            if let Some(object) = by_device.get(&format!("/dev/{holder}")) {
                mounts.extend(mounts_of(object.ifaces)?);
            }
            queue.push_back(holder);
        }
    }
    Ok(mounts)
}

/// Builds the disk list from a `GetManagedObjects` reply.
pub(crate) fn parse_disks(
    objects: &Objects,
    include_loop: bool,
    holders: HoldersFn,
) -> Result<Vec<Disk>, BackendError> {
    let blocks = objects
        .iter()
        .filter_map(|(path, ifaces)| Some((path, ifaces.get(IFACE_BLOCK)?, ifaces)))
        .map(|(path, block, ifaces)| {
            Ok(BlockObject {
                path,
                block,
                ifaces,
                device: device_of(block)?,
            })
        })
        .collect::<Result<Vec<_>, BackendError>>()?;
    let by_device: HashMap<String, &BlockObject> =
        blocks.iter().map(|b| (b.device.clone(), b)).collect();

    let mut disks = vec![];
    for disk in blocks
        .iter()
        .filter(|b| !b.ifaces.contains_key(IFACE_PARTITION))
    {
        let size: u64 = required(disk.block, "Size")?;
        let drive: OwnedObjectPath = required(disk.block, "Drive")?;
        let has_drive = drive.as_str() != NO_DRIVE;
        let is_loop = disk.device.starts_with("/dev/loop");
        if size == 0 || !(has_drive || (include_loop && is_loop)) {
            continue;
        }

        let drive_props = objects.get(&drive).and_then(|i| i.get(IFACE_DRIVE));
        let drive_text = |key: &str| {
            drive_props
                .and_then(|p| prop::<String>(p, key))
                .unwrap_or_default()
        };

        let mut partitions = vec![];
        for part in blocks
            .iter()
            .filter(|b| b.ifaces.contains_key(IFACE_PARTITION))
        {
            let table: OwnedObjectPath = required(&part.ifaces[IFACE_PARTITION], "Table")?;
            if &table == disk.path {
                partitions.push(partition_of(part)?);
            }
        }
        // A file system written straight on the disk (no partition table).
        if disk.ifaces.contains_key(IFACE_FILESYSTEM) && !disk.ifaces.contains_key(IFACE_TABLE) {
            partitions.push(partition_of(disk)?);
        }
        partitions.sort_by(|a, b| a.device.cmp(&b.device));

        let mut roots = vec![disk.device.trim_start_matches("/dev/").to_owned()];
        roots.extend(
            partitions
                .iter()
                .map(|p| p.device.trim_start_matches("/dev/").to_owned()),
        );

        disks.push(Disk {
            device: disk.device.clone(),
            // The drive path embeds vendor, model and serial and survives re-formatting.
            // `Block.Id` must not be used: udisks derives it from the disk's contents.
            id: if has_drive {
                drive.as_str().to_owned()
            } else {
                String::new()
            },
            vendor: drive_text("Vendor"),
            model: drive_text("Model"),
            size,
            removable: drive_props
                .and_then(|p| prop::<bool>(p, "Removable"))
                .unwrap_or(false),
            partitions,
            holder_mounts: stacked_mounts(roots, &by_device, holders)?,
        });
    }
    disks.sort_by(|a, b| a.device.cmp(&b.device));
    Ok(disks)
}

/// Direct holders of `name`, read from `/sys/class/block/<name>/holders`.
fn sysfs_holders(name: &str) -> Result<Vec<String>, BackendError> {
    let dir = format!("/sys/class/block/{name}/holders");
    match std::fs::read_dir(&dir) {
        Ok(entries) => Ok(entries
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .collect()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(err) => Err(parse_error(&dir, err)),
    }
}

pub struct UdisksBackend {
    conn: Connection,
    include_loop: bool,
}

fn bus_error(err: zbus::Error) -> BackendError {
    BackendError::Bus(err.to_string())
}

fn block_path(device: &str) -> String {
    format!(
        "{BLOCK_PREFIX}{}",
        device.strip_prefix("/dev/").unwrap_or(device)
    )
}

impl UdisksBackend {
    /// `include_loop` also lists loop devices (used by tests, hidden in the UI).
    pub fn connect(include_loop: bool) -> Result<Self, BackendError> {
        let conn = Connection::system().map_err(bus_error)?;
        Ok(Self { conn, include_loop })
    }

    fn call<B>(
        &self,
        path: &str,
        iface: &str,
        method: &str,
        body: &B,
    ) -> Result<zbus::Message, BackendError>
    where
        B: serde::Serialize + zbus::zvariant::DynamicType,
    {
        self.conn
            .call_method(Some(SERVICE), path, Some(iface), method, body)
            .map_err(bus_error)
    }
}

impl DiskBackend for UdisksBackend {
    fn list_disks(&self) -> Result<Vec<Disk>, BackendError> {
        let reply = self.call(
            ROOT_PATH,
            "org.freedesktop.DBus.ObjectManager",
            "GetManagedObjects",
            &(),
        )?;
        let objects: Objects = reply.body().deserialize().map_err(bus_error)?;
        parse_disks(&objects, self.include_loop, &sysfs_holders)
    }
}

impl StepRunner for UdisksBackend {
    fn run_step(&self, disk: &Disk, step: &Step) -> Result<(), BackendError> {
        let no_options: HashMap<&str, Value<'_>> = HashMap::new();
        match step {
            Step::Unmount { device } => {
                if !disk.partitions.iter().any(|p| &p.device == device) {
                    return Err(BackendError::Parse {
                        what: "Unmount".into(),
                        reason: format!("{device} is not a partition of {}", disk.device),
                    });
                }
                self.call(
                    &block_path(device),
                    IFACE_FILESYSTEM,
                    "Unmount",
                    &(no_options,),
                )?;
            }
            Step::CreateTable { kind, erase } => {
                let mut options = no_options;
                if *erase == EraseMode::Zero {
                    options.insert("erase", Value::from("zero"));
                }
                self.call(
                    &block_path(&disk.device),
                    IFACE_BLOCK,
                    "Format",
                    &(kind.udisks_name(), options),
                )?;
            }
            Step::CreatePartition { fs, label, table } => {
                // GPT lets udisks pick the type; MBR needs it spelled out per file system.
                let partition_type = match table {
                    TableKind::Mbr => fs.mbr_type(),
                    TableKind::Gpt => "",
                };
                let format_options = HashMap::from([("label", Value::from(label.as_str()))]);
                self.call(
                    &block_path(&disk.device),
                    IFACE_TABLE,
                    "CreatePartitionAndFormat",
                    &(
                        FIRST_PARTITION_OFFSET,
                        0u64,
                        partition_type,
                        "",
                        no_options,
                        fs.udisks_name(),
                        format_options,
                    ),
                )?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use zbus::zvariant::ObjectPath;

    use super::*;

    const DRIVE: &str = "/org/freedesktop/UDisks2/drives/D";
    const DISK_PATH: &str = "/org/freedesktop/UDisks2/block_devices/sdb";

    fn val<'a>(v: impl Into<Value<'a>>) -> OwnedValue {
        OwnedValue::try_from(v.into()).unwrap()
    }

    fn opath(p: &str) -> OwnedObjectPath {
        ObjectPath::try_from(p.to_owned()).unwrap().into()
    }

    fn block(device: &str, drive: &str, size: u64) -> Props {
        Props::from([
            ("Device".into(), val([device.as_bytes(), &[0]].concat())),
            (
                "Drive".into(),
                val(ObjectPath::try_from(drive.to_owned()).unwrap()),
            ),
            ("Size".into(), val(size)),
            ("Id".into(), val("by-id-test")),
        ])
    }

    fn mounts(points: &[&str]) -> Props {
        let list: Vec<Vec<u8>> = points
            .iter()
            .map(|m| format!("{m}\0").into_bytes())
            .collect();
        Props::from([("MountPoints".into(), val(list))])
    }

    fn partition_ifaces(
        device: &str,
        table: Value<'static>,
        extra: Vec<(&str, Props)>,
    ) -> Interfaces {
        let mut ifaces = Interfaces::from([
            (IFACE_BLOCK.into(), block(device, "/", 500)),
            (
                IFACE_PARTITION.into(),
                Props::from([("Table".into(), val(table))]),
            ),
        ]);
        ifaces.extend(extra.into_iter().map(|(k, v)| (k.to_owned(), v)));
        ifaces
    }

    fn table_value() -> Value<'static> {
        Value::from(ObjectPath::try_from(DISK_PATH.to_owned()).unwrap())
    }

    fn sample_objects(sdb1_extra: Vec<(&str, Props)>) -> Objects {
        let drive = Props::from([
            ("Vendor".into(), val("SanDisk")),
            ("Model".into(), val("Ultra")),
            ("Removable".into(), val(true)),
        ]);
        Objects::from([
            (
                opath(DRIVE),
                Interfaces::from([(IFACE_DRIVE.into(), drive)]),
            ),
            (
                opath(DISK_PATH),
                Interfaces::from([
                    (IFACE_BLOCK.into(), block("/dev/sdb", DRIVE, 1000)),
                    (IFACE_TABLE.into(), Props::new()),
                ]),
            ),
            (
                opath("/org/freedesktop/UDisks2/block_devices/sdb1"),
                partition_ifaces("/dev/sdb1", table_value(), sdb1_extra),
            ),
            (
                opath("/org/freedesktop/UDisks2/block_devices/loop3"),
                Interfaces::from([(IFACE_BLOCK.into(), block("/dev/loop3", "/", 4096))]),
            ),
        ])
    }

    fn no_holders(_: &str) -> Result<Vec<String>, BackendError> {
        Ok(vec![])
    }

    fn parse(objects: &Objects) -> Result<Vec<Disk>, BackendError> {
        parse_disks(objects, false, &no_holders)
    }

    #[test]
    fn parses_disk_with_drive_info_and_mounted_partition() {
        let objects = sample_objects(vec![(IFACE_FILESYSTEM, mounts(&["/run/media/me/KEY"]))]);
        let disks = parse(&objects).unwrap();
        assert_eq!(disks.len(), 1);
        let disk = &disks[0];
        assert_eq!(disk.device, "/dev/sdb");
        assert_eq!(
            disk.id, DRIVE,
            "identity must come from the drive, not the block"
        );
        assert_eq!(disk.display_name(), "SanDisk Ultra");
        assert!(disk.removable);
        assert_eq!(disk.partitions[0].mount_points, vec!["/run/media/me/KEY"]);
        assert!(!disk.is_system());
    }

    #[test]
    fn identity_survives_a_change_of_the_block_id_after_formatting() {
        // Real udisks: Block.Id is derived from what is on the disk and changes
        // (here to empty) once the partition table is rewritten.
        let before = sample_objects(vec![]);
        let mut after = sample_objects(vec![]);
        after
            .get_mut(&opath(DISK_PATH))
            .and_then(|ifaces| ifaces.get_mut(IFACE_BLOCK))
            .map(|block| block.insert("Id".into(), val("")));
        let id_of = |objects: &Objects| parse(objects).unwrap()[0].id.clone();
        assert_eq!(id_of(&before), id_of(&after));
    }

    #[test]
    fn different_drives_have_different_identities() {
        let mut other = sample_objects(vec![]);
        let drive_ifaces = other.remove(&opath(DRIVE)).unwrap();
        let other_drive = "/org/freedesktop/UDisks2/drives/OTHER";
        other.insert(opath(other_drive), drive_ifaces);
        other
            .get_mut(&opath(DISK_PATH))
            .and_then(|ifaces| ifaces.get_mut(IFACE_BLOCK))
            .map(|block| {
                block.insert(
                    "Drive".into(),
                    val(ObjectPath::try_from(other_drive.to_owned()).unwrap()),
                )
            });
        let original = parse(&sample_objects(vec![])).unwrap()[0].id.clone();
        assert_ne!(parse(&other).unwrap()[0].id, original);
    }

    #[test]
    fn loop_devices_are_hidden_unless_requested() {
        let objects = sample_objects(vec![]);
        assert!(parse(&objects)
            .unwrap()
            .iter()
            .all(|d| d.device != "/dev/loop3"));
        let with_loops = parse_disks(&objects, true, &no_holders).unwrap();
        assert!(with_loops.iter().any(|d| d.device == "/dev/loop3"));
    }

    #[test]
    fn zero_sized_blocks_are_ignored() {
        let mut objects = sample_objects(vec![]);
        objects.insert(
            opath("/org/freedesktop/UDisks2/block_devices/sr0"),
            Interfaces::from([(IFACE_BLOCK.into(), block("/dev/sr0", DRIVE, 0))]),
        );
        assert!(parse(&objects)
            .unwrap()
            .iter()
            .all(|d| d.device != "/dev/sr0"));
    }

    #[test]
    fn unreadable_partition_table_property_is_an_error_not_a_missing_partition() {
        let mut objects = sample_objects(vec![]);
        objects.insert(
            opath("/org/freedesktop/UDisks2/block_devices/sdb1"),
            partition_ifaces("/dev/sdb1", Value::from("not-a-path"), vec![]),
        );
        assert!(matches!(parse(&objects), Err(BackendError::Parse { .. })));
    }

    #[test]
    fn unreadable_mount_points_are_an_error() {
        let bad = Props::from([("MountPoints".into(), val("oops"))]);
        let objects = sample_objects(vec![(IFACE_FILESYSTEM, bad)]);
        assert!(matches!(parse(&objects), Err(BackendError::Parse { .. })));
    }

    #[test]
    fn empty_device_name_is_an_error() {
        let mut objects = sample_objects(vec![]);
        objects.insert(
            opath("/org/freedesktop/UDisks2/block_devices/weird"),
            Interfaces::from([(IFACE_BLOCK.into(), block("", DRIVE, 10))]),
        );
        assert!(matches!(parse(&objects), Err(BackendError::Parse { .. })));
    }

    #[test]
    fn root_on_luks_is_attributed_to_the_backing_disk() {
        let mut objects = sample_objects(vec![]);
        objects.insert(
            opath("/org/freedesktop/UDisks2/block_devices/dm_2d0"),
            Interfaces::from([
                (IFACE_BLOCK.into(), block("/dev/dm-0", "/", 900)),
                (IFACE_FILESYSTEM.into(), mounts(&["/"])),
            ]),
        );
        let holders = |name: &str| {
            Ok(if name == "sdb1" {
                vec!["dm-0".to_owned()]
            } else {
                vec![]
            })
        };
        let disks = parse_disks(&objects, false, &holders).unwrap();
        assert_eq!(disks[0].holder_mounts, vec!["/"]);
        assert!(disks[0].is_system());
    }

    #[test]
    fn mounts_two_levels_up_the_stack_are_found() {
        let mut objects = sample_objects(vec![]);
        for (name, mount) in [("dm_2d0", None), ("dm_2d1", Some("/home"))] {
            let mut ifaces = Interfaces::from([(
                IFACE_BLOCK.into(),
                block(&format!("/dev/{}", name.replace("_2d", "-")), "/", 900),
            )]);
            if let Some(m) = mount {
                ifaces.insert(IFACE_FILESYSTEM.into(), mounts(&[m]));
            }
            objects.insert(
                opath(&format!("/org/freedesktop/UDisks2/block_devices/{name}")),
                ifaces,
            );
        }
        let holders = |name: &str| {
            Ok(match name {
                "sdb1" => vec!["dm-0".to_owned()],
                "dm-0" => vec!["dm-1".to_owned()],
                _ => vec![],
            })
        };
        let disks = parse_disks(&objects, false, &holders).unwrap();
        assert_eq!(disks[0].holder_mounts, vec!["/home"]);
    }

    #[test]
    fn active_swap_marks_the_partition_as_in_use() {
        let swap = Props::from([("Active".into(), val(true))]);
        let disks = parse(&sample_objects(vec![(IFACE_SWAP, swap)])).unwrap();
        assert_eq!(disks[0].partitions[0].mount_points, vec![SWAP_MOUNT]);
        assert!(disks[0].is_system());
    }

    #[test]
    fn inactive_swap_is_not_a_mount() {
        let swap = Props::from([("Active".into(), val(false))]);
        let disks = parse(&sample_objects(vec![(IFACE_SWAP, swap)])).unwrap();
        assert!(disks[0].partitions[0].mount_points.is_empty());
    }

    #[test]
    fn holder_lookup_failure_is_an_error() {
        let failing = |_: &str| Err(BackendError::Bus("sysfs unreadable".into()));
        assert!(parse_disks(&sample_objects(vec![]), false, &failing).is_err());
    }

    #[test]
    fn block_path_uses_the_kernel_name() {
        assert_eq!(
            block_path("/dev/sdb1"),
            "/org/freedesktop/UDisks2/block_devices/sdb1"
        );
    }
}
