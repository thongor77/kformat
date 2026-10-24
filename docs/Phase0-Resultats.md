# Phase 0 — spike results (2026-10-18)

Environment: Arch Linux, Qt 6.12, Kirigami 6.30, udisks2 2.11.2, stable Rust. All tests ran on a throw-away loop image, never on a real disk.

## Spike 1 — udisks2 `Block.Format` for each file system

| File system | Result |
|---|---|
| ext4, btrfs, xfs, f2fs, exfat, vfat | OK; `IdType` and `IdLabel` correct, the `label` option is honoured |
| ntfs | Clean failure: "executable mkfs.ntfs not found" (the `ntfs-3g` package is missing) |

- No polkit prompt on a loop device (confirmed later on a real USB stick).
- Partition-table flow: `Block.Format('gpt')` followed by `PartitionTable.CreatePartitionAndFormat(offset, size = 0 → whole space, type, name, {}, fs, {label})` works in a single call.
- Useful signature: `CreatePartitionAndFormat` is `ttssa{sv}sa{sv}`.
- Consequence: NTFS has to be detected at run time (is `mkfs.ntfs` present?) and greyed out with an explanation when it is not.

## Spike 2 — cxx-qt 0.10 + Kirigami

- Pure Cargo build, no CMake (`qmake6` + `ninja` + `clang`/`g++` are enough). No compilation problems.
- QML module `org.kformat.spike` loads, and a `Kirigami.ApplicationWindow` opens under Wayland without warnings.
- A Rust QObject (`DiskModel`) exposes a property and a method callable from QML; a udisks2 D-Bus call succeeds.
- `cxx-qt-lib` 0.10 only exposes `QGuiApplication` (no `QApplication`). The `org.kde.desktop` and `org.kde.breeze` styles start without errors, and the default style looks right under Plasma/Wayland.

## Spike 3 — progress

- During `Format` with `erase=zero`, udisks2 creates a `Job` object (`/org/freedesktop/UDisks2/jobs/N`) with `Operation="format-erase"`, `Bytes`, `Progress` and `ProgressValid=true`.
- Progress is therefore observable while erasing (the sparse test image finished before any useful sampling, so it still needs a large real medium).
- Without erasing, the operation is nearly instantaneous: an indeterminate indicator is enough.

## Resulting decision: zbus 5, not zbus 6

zbus 6.0.0 is a rewrite: no `blocking` module, renamed types (`OwnedVariant`). We stay on **zbus 5.x** (mature, blocking API available) and will reconsider once 6.x settles.

## Remaining unknowns at the time

- Progress on a large real medium.
- A blocking D-Bus call on the UI thread: moved to a worker thread (`CxxQtThread`) in phase 2.

The spike code lives in `spikes/hello/` (throw-away, outside the workspace).
