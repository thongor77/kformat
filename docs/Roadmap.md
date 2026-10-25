# Roadmap

## Done

- **Phase 0 — spikes** (2026-10-18): udisks2 `Format` for each file system, cxx-qt + Kirigami, progress. See [`Phase0-Resultats.md`](Phase0-Resultats.md).
- **Phase 1 — `core`** (2026-10-18): disk model, presets, validated plans with guardrails, sealed backend trait, udisks2 backend; code review done and its blocking findings fixed.
- **Phase 2 — `app`** (2026-10-18): three-step wizard (disk list, format choice, confirmation and progress), headless end-to-end test on a loop image (`--features smoke`), code review done and fixed.
- **First real-hardware run** (2026-10-18): a full format of a USB stick, after fixing a false positive in the disk-identity guardrail (see the decisions log).
- **Phase 4 — desktop integration** (2026-10-18 → 24): `--device /dev/sdX`, application launcher, *"Format with kformat…"* Solid action in the Devices widget (verified), per-user install script, AUR `PKGBUILD` (built and inspected with `makepkg`).
- **Sleep/shutdown inhibitor** (2026-10-24): a logind lock is held while a plan runs; verified during a real format.
- **Translations** (2026-10-24): English, French, Spanish through a compiled-in catalogue.
- **English documentation** (2026-10-24).
- **Application icon** (2026-10-25): own SVG icon, installed by the user script and the AUR package.
- **v0.1.0 released** (2026-10-24): public repository, tag, GitHub Release and AUR package.

## Open

- [x] **v0.1.1** (application icon): tag and GitHub Release published; the AUR `PKGBUILD` carries the checksum and was built from the real tag archive.
- [ ] Push the 0.1.1 update to the AUR repository (`packaging/aur/README.md`).

- [ ] Native-speaker review of the Spanish translation.
- [x] Tag `v0.1.0` (2026-10-24); the AUR `PKGBUILD` carries its checksum and was built from the real tag archive.
- [x] AUR package published (2026-10-24): https://aur.archlinux.org/packages/kformat
- [ ] Review follow-ups (non-blocking):
  - [ ] stronger confirmation for non-removable disks;
  - [ ] keep error sources (`#[source]`) in `BackendError`;
  - [ ] retry between `Format` and `CreatePartitionAndFormat`;
  - [ ] timeout / cancellation for the zero-fill erase;
  - [ ] natural sort (`sdb2` before `sdb10`);
  - [ ] a partition mounted in several places;
  - [ ] run `load_options` off the UI thread.

## Ideas

- v2: partitioning (resize, several partitions).
