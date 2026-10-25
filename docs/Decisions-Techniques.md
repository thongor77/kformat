# Technical decisions

A log of the design decisions, newest sections last, with the reason behind each. All dates are 2026.

## 10-18 — Interface: cxx-qt + Kirigami
The official Rust/Qt bridge (KDAB), with a native Plasma look. Rejected: qmetaobject-rs (less maintained), iced/egui (not native).

## 10-18 — Backend: udisks2 through zbus
udisks2 already lists, unmounts, formats and authorizes through polkit, so kformat needs no privileged daemon of its own. Falling back to `mkfs.*` + `pkexec` would only be needed for what udisks2 does not cover.

## 10-18 — Scope of the first version: formatting only
Partitioning (resizing, several partitions) doubles the complexity; postponed to a later version.

## 10-18 — zbus 5.x, not 6.0
zbus 6.0.0 removes the `blocking` API and renames types. We stay on 5.x until 6 settles.

## 10-18 — NTFS is conditional
`mkfs.ntfs` is not installed by default: NTFS is detected at run time and greyed out with an explanation (and the package to install).

## 10-18 — Guardrails enforced by types
`build_plan` validates (system disk, available file system, label, sizes, MBR above 2 TiB) and returns a `FormatPlan`. Only `FormatPlan::confirm(disk_name)` produces a `ConfirmedPlan`, and only a `ConfirmedPlan` can be executed by a backend. `DiskBackend::execute` re-reads the disk list just before writing: a vanished disk, or one that became a system disk, aborts the run.

## 10-18 — Explicit MBR partition type
Without a type, udisks2 uses 0x83 (Linux) even for FAT32 on MBR. `FsType::mbr_type()` enforces 0x0c (FAT32), 0x07 (exFAT/NTFS) and 0x83 otherwise. Verified on a loop image.

## 10-18 — The real udisks2 layout
A partition's `Table` property is on the `Partition` interface, not on `Block`. A wrong synthetic test had hidden it; the integration test on a loop image revealed it. Synthetic tests must mirror the layout observed on the real bus.

## 10-18 — Phase 1 code review: "system disk" detection inverted
The review's verdict on the first version was BLOCK. Fixed:
- A mount counts as user data only under `/run/media/`, `/media/` or `/mnt/`; everything else (`/var/lib/…`, `/home/…`, swap) counts as the system. A whitelist of user mounts instead of a blacklist of system paths.
- Root on LUKS/LVM/md: mounts of devices stacked on the disk (`/sys/class/block/<n>/holders`, transitively) are attributed to it (`Disk.holder_mounts`).
- Swap is read from the udisks2 `Swapspace.Active` property (the string `[SWAP]` is an `lsblk` convention that udisks2 never provides).
- Identity: before EVERY step the disk must still be the same drive and not a system disk, and before the first step its partition layout must also match (`same_drive` / `same_media`). This protects against a disk that is unplugged and replaced under the same name.
- The `DiskBackend` trait is sealed (`sealed::StepRunner`): `run_step` cannot be called from outside, so `build_plan` → `confirm` → `execute` cannot be bypassed.
- Strict udisks2 parsing: an unreadable property is an error (`BackendError::Parse`), never a silently lost partition or mount; an empty device name is refused.
- `Unmount` checks that the partition belongs to the planned disk.

## Review follow-ups (medium/low severity) — see the roadmap
Stronger confirmation for non-removable disks, `#[source]` on errors, a retry between `Format` and `CreatePartitionAndFormat`, a timeout/cancellation for the zero-fill erase, natural sorting (`sdb2` before `sdb10`), `#[non_exhaustive]`/public docs, and `Unmount` of a partition mounted in several places.

## 10-18 — Kirigami rendering confirmed
The spike (QGuiApplication + Kirigami, default style) looks right under Plasma/Wayland, so there is no need to work around the missing `QApplication`. Note: run the binary directly rather than through `cargo run`.

## 10-18 — UI: JSON instead of a Qt list model
`core::view` serializes disks to JSON (camelCase), exposed as the `disksJson` property of `DiskController` and read with `JSON.parse` in QML. Simpler than a `QAbstractListModel` for a handful of items, and the presentation logic (sizes, blocking reason, ordering) is tested without Qt. To be revisited if the list becomes dynamic or large.

## 10-18 — D-Bus calls off the UI thread
`DiskController::refresh` starts a `std::thread` and returns the result through `CxxQtThread::queue`. `busy` prevents concurrent refreshes.

## 10-18 — Controller state machine
`phase`: idle → planning → ready → running → done | failed. The validated plan is kept on the Rust side (`Option<FormatPlan>`) and never rebuilt from QML. `confirmAndRun(typed)` calls `FormatPlan::confirm`: even if the interface let a wrong entry through, `core` refuses it. `reset()` and `preparePlan()` are ignored while `running`, and the window refuses to close while `running`.

## 10-18 — Headless end-to-end test (the `smoke` feature, off by default)
Code review: a `--smoke` hook reachable in production would have allowed, with `QT_QPA_PLATFORM=offscreen`, formatting a disk without any typed confirmation. The hook now only exists in a `--features smoke` build (the test QML is not even packaged otherwise) and refuses any device outside `/dev/loop*`. Dedicated build: `CARGO_TARGET_DIR=target/smoke cargo build -p kformat-app --features smoke`.
`kformat-app --smoke <loop>` (offscreen platform) runs prepare → wrong confirmation (refused) → right confirmation → write, through the same invokables the interface uses. The flag only adds loop devices to what `UdisksBackend` lists; the guardrails of `core` stay active (a system disk is refused).

## 10-18 — Phase 2 code review (verdict WARNING, fixed)
- A panic in a worker thread: `core::guard::run_guarded` turns any panic into an error message (refresh, prepare, execution). Otherwise the app stayed stuck in `running` with a window that could not be closed.
- Stale plan: `request_id` is bumped by `prepare_plan` and `reset`, and a result that no longer matches is ignored; `OptionsPage` calls `reset()` when created and checks the plan received is for its own disk.
- "Back" during a write: `ConfirmPage` is replaced (`replace`) by `RunPage`, which refuses `backRequested` while `running`.
- A failure after the first step: the message says the disk may be left in an incomplete state.

## 10-18 — Disk identity: the Drive object, never `Block.Id` (bug found on a real stick)
On the first real run (a 31 GB USB stick) the "disk changed" guardrail aborted the format at the last step, after the GPT table had been created, leaving the stick half-formatted. Cause: `Disk.id` came from `Block.Id`, which udisks2 derives from the *contents* of the disk (it became empty once the table was rewritten). Loop images have no Drive, so they could not reveal the bug.
Fix: `Disk.id` is the Drive object path (vendor/model/serial, stable across re-formatting), empty for a loop device. Regression tests: the identity survives a change of `Block.Id`, and two distinct Drives give two identities.
In addition, a check that fails *after* the first step returns `BackendError::Interrupted` (steps done / total + cause), with a message saying the disk may be left in an incomplete state. **Lesson: every identity guardrail must be tested on real hardware, not only on loop images.**

## 10-18 — Desktop integration: a Solid action, not a Dolphin service menu
Dolphin service menus only apply to files and folders, not to entries of the "Devices" section. The KDE mechanism for a device is a Solid action (`solid/actions/*.desktop`), offered in the Devices widget. The format follows KDE's own actions (`/usr/share/solid/actions/`). Pitfall avoided: in `Exec`, `%f` is the mount point and `%d` the device node (`/dev/sda1`); we use `%d`. `desktop-file-validate` reports errors on this kind of file, including KDE's own: noise from the generic validator.

## 10-18 — `--device`: strict validation
Only plain `/dev/<name>` nodes are accepted (no `..`, sub-directory, space or `;`). The option bypasses nothing: the disk is re-read, the guardrails of `core` apply, and the typed confirmation stays mandatory.

## 10-18 — MIT license
`Cargo.toml` originally announced GPL-3.0-or-later without a deliberate choice and without a LICENSE file. Replaced by MIT (the `LICENSE` file), consistent with the maintainer's other KDE tool `netmnt`.

## 10-18 — AUR package
`packaging/aur/PKGBUILD` follows the conventions of `netmnt` (archive of the GitHub tag, `prepare` with `cargo fetch --locked`, a frozen `build`, `check` = unit tests only because the udisks2 tests are `#[ignore]`d and cannot run in a build chroot, a hand-written `package`). Build-directory paths are rewritten (`-ffile-prefix-map`, `--remap-path-prefix`) so no `$srcdir` stays in the binary. Runtime dependencies: qt6-base, qt6-declarative, kirigami, udisks2, polkit; the `mkfs` tools are optional (kformat greys the format out and names the package). Tested with `makepkg` from a local archive; `sha256sums=('SKIP')` until the tag exists.

## 10-24 — Sleep/shutdown inhibitor while writing
`DiskBackend::inhibit_sleep` (default: `None`); `execute` takes the lock before the first step and keeps it until it returns (the `SleepGuard` is dropped on success, error or panic, which closes the file descriptor and releases the lock). Real implementation: `org.freedesktop.login1.Manager.Inhibit("shutdown:sleep", "kformat", reason, "block")` on the system bus.
- Non-blocking: if logind refuses, formatting proceeds anyway (a safety net must never prevent formatting); the `sleep_protection_available()` probe at plan time feeds `sleepProtected`, and the confirmation screen only warns when sleep is not blocked.
- It does not protect against a `kill`, a crash, a power cut or unplugging; session logout is handled by the desktop, not logind. The real benefit: suspend (closed lid, idle) and shutdown/reboot.
- Tested: a fake backend (lock held at each step, released after success / failure / vanished disk, formatting without a lock), a real `systemd-inhibit --list` check (present with shutdown/sleep/block, gone after the drop), and **a real USB format** (28.9 GB, 3 steps) sampled every 40 ms: lock present while writing, absent afterwards.

## 10-24 — The Solid action did not show up: user directory not watched
After a per-user install, "Format with kformat…" did not appear in the Devices widget. The predicate was correct (`solid-hardware6 query` returns the stick) and identical to Gwenview's, which did show. Cause: the widget has a "predicates monitor" that only watches the `solid/actions` directories that exist when `plasmashell` starts; `~/.local/share/solid/actions` was created by the install, after that (checked in `/proc/<pid>/fdinfo`: 1 inotify watch on `/usr/share/solid/actions`, 0 on the user directory). Remedy: restart the shell (`systemctl --user restart plasma-plasmashell.service`) or log back in, once. `install-user.sh` now warns when it creates that directory. The AUR package installs into `/usr/share/solid/actions`, which is already watched, so it is not affected.

## 10-24 — Translations (en, fr, es): a catalogue compiled into the program
Need: the interface was 100 % French. Rejected: Qt `.ts`/`.qm` files (cxx-qt-lib 0.10 does not expose `QTranslator`, and text produced by Rust would have needed a second path), and gettext as in `netmnt` (an extra QML binding layer, `.mo` files to install, completeness only checked at run time).
Chosen: `core::messages` — a `messages!` macro in which every entry MUST give `en`, `fr` and `es` (a missing translation is a compile error). Positional placeholders `{0}`, `{1}`…; a test checks they are identical in the three languages and have no gaps, that keys are unique and that at least 80 % of the texts differ between languages. `core::i18n::Lang` picks the language (`KFORMAT_LANG`, then `LC_ALL`/`LC_MESSAGES`/`LANG`, then the `LANGUAGE` list unless the locale is `C`/`POSIX`, falling back to English) and carries the size units (`GB`/`Go`) and the decimal mark.
On the QML side there is a single path: `controller.tr(key)`, `tr1`, `tr2`, `tr3`. The test `crates/core/tests/qml_messages.rs` checks that every key used in QML exists and receives exactly the right number of arguments (it lives in `core` because the `app` crate's test binaries cannot link the generated Qt code).
Consequences: adding a language is one `Lang` variant plus one text per message (the compiler lists what is missing); there is no translation file to install. Limit: no community translation through `.po` files; to be revisited if contributors ask for it.
`--device`: `cli::parse_device_arg` returns the rejected value, and the message is built per language (`view::device_arg_error_message`). `session::UnknownOption` carries a neutral text (`fs=zfs`).
The Spanish translation was written without review by a native speaker.

## 10-24 — Documentation in English
README and `docs/` are in English for visibility; `CLAUDE.md` (notes for the maintainer's coding assistant) stays in French, as in the maintainer's other projects.

## 10-25 — Application icon
kformat used the generic `drive-removable-media` theme icon. It now has its own: a white USB stick with a green "verified" badge on a Breeze-style blue tile, drawn as a single SVG (`packaging/icons/hicolor/scalable/apps/kformat.svg`) plus 512 px and 256 px PNG exports for web listings and the README. It follows the freedesktop icon-theme layout, so the launcher and the Solid action just say `Icon=kformat`; `scripts/install-user.sh` installs it under `~/.local/share/icons/hicolor/scalable/apps/` and the AUR package under `/usr/share/icons/hicolor/scalable/apps/`. No raster sizes are shipped: the SVG scales, and was checked down to 22 px on light and dark backgrounds.
