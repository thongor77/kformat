//! Real udisks2 round trip on a throw-away loop image.
//! Run with: `cargo test -p kformat-core --test loop_device -- --ignored`
use std::path::PathBuf;
use std::process::Command;

use kformat_core::backend::DiskBackend;
use kformat_core::fs::{available_filesystems, binary_in_path, FsType};
use kformat_core::plan::{build_plan, EraseMode, FormatRequest};
use kformat_core::preset::TableKind;
use kformat_core::udisks::UdisksBackend;

/// Loop device backed by a temp file; detached and deleted on drop.
struct LoopImage {
    image: PathBuf,
    device: String,
}

impl LoopImage {
    fn create() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let image =
            std::env::temp_dir().join(format!("kformat-test-{}-{nonce}.img", std::process::id()));
        std::fs::File::create(&image)
            .unwrap()
            .set_len(512 * 1024 * 1024)
            .unwrap();
        let out = Command::new("udisksctl")
            .args(["loop-setup", "--no-user-interaction", "-f"])
            .arg(&image)
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let device = text
            .trim()
            .rsplit(' ')
            .next()
            .map(|d| d.trim_end_matches('.').to_owned())
            .filter(|d| d.starts_with("/dev/loop"))
            .unwrap_or_else(|| {
                panic!(
                    "loop-setup failed: {text} {}",
                    String::from_utf8_lossy(&out.stderr)
                )
            });
        Self { image, device }
    }
}

impl Drop for LoopImage {
    fn drop(&mut self) {
        let _ = Command::new("udisksctl")
            .args(["loop-delete", "--no-user-interaction", "-b", &self.device])
            .status();
        let _ = std::fs::remove_file(&self.image);
    }
}

fn format_and_read_back(
    backend: &UdisksBackend,
    device: &str,
    fs: FsType,
    table: TableKind,
) -> (String, String) {
    let disk = backend
        .list_disks()
        .unwrap()
        .into_iter()
        .find(|d| d.device == device)
        .expect("loop device listed");
    let request = FormatRequest {
        disk,
        fs,
        table,
        label: "KTEST".into(),
        erase: EraseMode::Quick,
    };
    let available = available_filesystems(&binary_in_path);
    let plan = build_plan(&request, &available).unwrap();
    let phrase = plan.confirmation_phrase().to_owned();
    backend
        .execute(&plan.confirm(&phrase).unwrap(), &mut |_| {})
        .unwrap();

    let started = std::time::Instant::now();
    loop {
        let after = backend
            .list_disks()
            .unwrap()
            .into_iter()
            .find(|d| d.device == device)
            .unwrap();
        // Wait for udisks to publish the new layout, not a stale one.
        if let Some(part) = after
            .partitions
            .first()
            .filter(|p| p.fs_type == fs.udisks_name() && p.label == "KTEST")
        {
            return (part.fs_type.clone(), part.label.clone());
        }
        assert!(
            started.elapsed().as_secs() < 5,
            "no partition after 5s: {after:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[test]
#[ignore = "needs udisks2 and creates a loop device"]
fn formats_a_loop_image_with_gpt_exfat_then_mbr_fat32() {
    let loop_image = LoopImage::create();
    let backend = UdisksBackend::connect(true).unwrap();

    let (fs, label) =
        format_and_read_back(&backend, &loop_image.device, FsType::Exfat, TableKind::Gpt);
    assert_eq!((fs.as_str(), label.as_str()), ("exfat", "KTEST"));

    let (fs, label) =
        format_and_read_back(&backend, &loop_image.device, FsType::Vfat, TableKind::Mbr);
    assert_eq!((fs.as_str(), label.as_str()), ("vfat", "KTEST"));

    let parttype = Command::new("lsblk")
        .args(["-no", "PARTTYPE", &format!("{}p1", loop_image.device)])
        .output()
        .unwrap();
    let parttype = String::from_utf8_lossy(&parttype.stdout).trim().to_owned();
    assert_eq!(
        parttype, "0xc",
        "FAT32 on MBR must carry the FAT32 LBA type"
    );
}

#[test]
#[ignore = "needs udisks2 on the system bus"]
fn the_disk_holding_the_running_system_is_detected() {
    let disks = UdisksBackend::connect(false).unwrap().list_disks().unwrap();
    let system: Vec<_> = disks.iter().filter(|d| d.is_system()).collect();
    assert!(!system.is_empty(), "no system disk found among {disks:?}");
    println!(
        "system disks: {:?}",
        system.iter().map(|d| d.name()).collect::<Vec<_>>()
    );
}
