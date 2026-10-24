# Architecture

## Problem

Formatting a disk is a common task, but the available tools are either heavy partition managers
or easy to misuse. kformat does one thing — format a whole disk — and makes it hard to destroy
the wrong one.

## Users

Plasma users from beginner to advanced who want to format a medium without knowing the `mkfs` family.

## Use cases

1. Format a USB stick so it works everywhere (exFAT).
2. Prepare a Linux disk (ext4 / Btrfs / XFS).
3. Prepare a disk for Windows (NTFS).
4. Wipe a medium before handing it over (overwrite with zeros).
5. Start from a device: *"Format with kformat…"* in Plasma's Devices widget, or `--device /dev/sdX`.

## Overview

```
kformat/
├── crates/
│   ├── core/   # disk model, presets, plans and guardrails, udisks2 backend, messages (i18n)
│   └── app/    # cxx-qt bridge (DiskController) and the Kirigami QML pages
├── packaging/  # desktop launcher, Solid device action, AUR PKGBUILD
├── scripts/    # per-user install / uninstall
├── spikes/     # throw-away experiments from phase 0 (not part of the workspace)
└── docs/
```

### Data flow

```
QML page ─▶ DiskController (app) ─▶ core::session::prepare_plan ─▶ core::plan::build_plan
                │                                                      │ (guardrails)
                │                                                      ▼
                │                                              FormatPlan ──confirm(typed disk name)──▶ ConfirmedPlan
                ▼                                                                                          │
        phase / progress / messages                                              DiskBackend::execute ◀────┘
                                                                                          │
                                                                                udisks2 (D-Bus) ─▶ polkit
```

- **`core`** has no UI dependency. The disk list, plans, error messages and the JSON the UI shows
  are all produced and unit-tested there. The backend is a trait; tests use an in-memory fake, and
  the real implementation talks to udisks2 with `zbus`.
- **`app`** is deliberately thin: a `DiskController` QObject that runs `core` on a worker thread
  and publishes the results as properties, and QML pages that only display them.

### Safety model

- No privileged code of our own: udisks2 performs the writes and polkit authorizes them.
- `build_plan` rejects system disks, missing tools, invalid labels, too-small disks and MBR on
  disks larger than 2 TiB. Only `FormatPlan::confirm(<exact disk name>)` yields a `ConfirmedPlan`,
  and only a `ConfirmedPlan` can be executed. The backend trait is sealed, so a single step cannot
  be run from outside the crate.
- `execute` re-reads the disk before **every** step (same physical drive, not a system disk) and,
  before the first step, also the partition layout. A failure after the first step is reported as
  an interrupted run.
- A system disk is recognised by a *whitelist* of user mounts (`/run/media`, `/media`, `/mnt`) —
  anything else counts as the system — plus mounts of devices stacked on the disk (LUKS, LVM, md)
  and active swap.
- While a plan runs, a systemd-logind inhibitor lock blocks suspend and shutdown.

### Interface

- The controller exposes JSON properties (`disksJson`, `optionsJson`, `planJson`) and a small
  state machine: `idle → planning → ready → running → done | failed`. The validated plan is kept on
  the Rust side and never rebuilt from QML.
- Worker threads are wrapped so a panic becomes an error message instead of a stuck window;
  stale plan results are dropped with a request counter.
- Pages: disk list → format choice → confirmation → progress/result. The window refuses to close,
  and the progress page refuses "back", while a disk is being written.

### Internationalisation

One compiled-in catalogue (`core::messages`): each entry must provide English, French and Spanish,
so a missing translation is a compile error. The language comes from `KFORMAT_LANG`, then the
`LC_*` / `LANG` / `LANGUAGE` variables, falling back to English. QML asks the controller for its
text (`controller.tr("Key")`), and a test checks that every key used in QML exists and receives the
right number of arguments.

## Open questions

- Partitioning (resize, several partitions) is out of scope for v1.
- Progress is reported per step; udisks2 exposes a real progress value only while erasing with zeros.
