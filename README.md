# kformat

**Format disks, SSDs and USB sticks from KDE — simply and safely.**

kformat is a small KDE Plasma application with a three-step wizard: pick a disk, pick what you
want to do with it, confirm by typing the disk's name. It is written in Rust, with a Kirigami
interface, and talks to [udisks2](https://www.freedesktop.org/wiki/Software/udisks/) — the same
service Dolphin and GNOME Disks use — so it needs no privileged daemon of its own and
authorization goes through polkit.

## Why

Existing tools (GNOME Disks, KDE Partition Manager) are complete, but heavy for the everyday task:
*"format this stick in that format, without wiping the wrong disk."* kformat does only that, and
puts safety first.

## Features

- **Three steps:** choose the disk → choose the format → review what will be erased and confirm.
- **Plain-language presets:** universal USB stick (exFAT), Linux disk (ext4), Windows disk (NTFS),
  maximum compatibility (FAT32) — or pick the file system, partition table (GPT/MBR) and erase
  mode (quick, or overwrite with zeros) yourself.
- **Formats:** ext4, Btrfs, XFS, F2FS, exFAT, FAT32 and NTFS. A format whose tool is not installed
  is greyed out and tells you which package to install.
- **Hard to wipe the wrong disk** — see [Safety](#safety).
- **Stays awake while it works:** it asks systemd-logind not to suspend or shut down during a format.
- **Desktop integration:** application launcher, and a *"Format with kformat…"* action in
  Plasma's Devices (device notifier) widget that opens kformat straight on that device.
- **Translated:** English, French and Spanish. It follows your system language; override with
  `KFORMAT_LANG=en|fr|es`.

## Safety

Formatting is irreversible, so the guardrails live in the core library, not in the UI:

- A disk that holds the running system is never offered. Detection covers `/`, `/boot`, `/home`,
  `/var`, active swap, and root on LUKS/LVM/RAID (devices stacked on the disk are followed).
- Nothing runs until you **type the disk's exact name**. The type system enforces it: only a
  confirmed plan can be executed by a backend.
- Right before each step the disk is looked up again: it must still be the same physical drive
  (identified by its drive object, not by `/dev/sdX`) and must not have become a system disk.
- If something fails after the first step, the message says the disk may be left in an
  incomplete state.
- No privileged helper: writes go through udisks2 and polkit.

## Usage

Start **kformat** from the application launcher, or from a terminal:

```bash
kformat                     # open on the list of disks
kformat --device /dev/sdb   # open straight on that disk (a partition such as /dev/sdb1 also works)
```

With a stick plugged in, open Plasma's *Devices* widget in the system tray and choose
**Format with kformat…**.

## Installation

### Arch Linux

Install it from the [AUR](https://aur.archlinux.org/packages/kformat):

```bash
yay -S kformat        # or: paru -S kformat
```

(The recipe lives in [`packaging/aur`](packaging/aur).) You can also build from source, below.

### From source

Requirements: Rust (stable), a C++ compiler, Qt 6 (`qt6-base` with `qmake6`, `qt6-declarative`),
`kirigami`, and at runtime `udisks2` and `polkit`.

```bash
cargo build --release -p kformat-app
./target/release/kformat-app
```

To install it for your user only (binary, launcher and device action; no root needed):

```bash
scripts/install-user.sh              # build and install into ~/.local
scripts/install-user.sh --uninstall  # remove it again
```

The first time the device action is installed, restart the Plasma shell so the Devices widget
notices it: `systemctl --user restart plasma-plasmashell.service` (or log out and back in).

The file-system tools are optional and only needed for the formats you use: `exfatprogs`,
`ntfs-3g`, `f2fs-tools`, `xfsprogs`, `btrfs-progs`, `dosfstools`, `e2fsprogs`.

## Development

```bash
cargo test --workspace                                      # unit tests
cargo test -p kformat-core --test loop_device -- --ignored  # real udisks2 / logind tests (use a throw-away loop image)
cargo clippy --workspace --all-targets -- -D warnings
```

The workspace has two crates: `core` (disk model, guardrails, udisks2 backend, messages — testable
without any UI) and `app` (cxx-qt bridge and the Kirigami QML pages). See
[`docs/Architecture.md`](docs/Architecture.md), [`docs/Decisions-Techniques.md`](docs/Decisions-Techniques.md)
(the design decisions and why), and [`docs/Roadmap.md`](docs/Roadmap.md).

All user-visible text lives in `crates/core/src/messages.rs`, with English, French and Spanish
side by side; a missing translation does not compile. To fix or add a translation, edit that file.

## Status

Version 0.1.0. It formats real USB sticks end to end. Partitioning (resizing, several partitions)
is deliberately out of scope for now. The Spanish translation has not been reviewed by a native
speaker yet — corrections are welcome.

## License

[MIT](LICENSE)
