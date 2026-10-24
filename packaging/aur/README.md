# AUR package for kformat

`PKGBUILD` builds kformat from the archive of the GitHub tag `v<pkgver>`.

## Test the package locally (without publishing)

Until the tag exists you can still test the `PKGBUILD` against a local archive:

```bash
# from the directory that contains kformat/: archive the project without build folders
tar --exclude='kformat/target' --exclude='kformat/spikes/*/target' \
    --transform 's,^kformat,kformat-0.1.0,' -czf /tmp/kformat-0.1.0.tar.gz kformat
# copy the PKGBUILD to a work folder and point `source=` at the local file
mkdir /tmp/kformat-aur && cp kformat/packaging/aur/PKGBUILD /tmp/kformat-aur/
sed -i 's|^source=.*|source=("$pkgname-$pkgver.tar.gz::file:///tmp/kformat-0.1.0.tar.gz")|' /tmp/kformat-aur/PKGBUILD
cd /tmp/kformat-aur && makepkg -f      # produces kformat-0.1.0-1-x86_64.pkg.tar.zst
pacman -Qlp kformat-0.1.0-1-x86_64.pkg.tar.zst   # inspect the contents
```

## Publish (maintainer)

The `PKGBUILD` in this folder already carries the checksum of the `v0.1.0` tag archive, and the
package was built from that archive with `makepkg` (tests pass, no warnings). To publish:

1. In a clean work folder: copy `PKGBUILD` and generate the metadata with
   `makepkg --printsrcinfo > .SRCINFO`.
2. Clone `ssh://aur@aur.archlinux.org/kformat.git` (an empty repository for a new package), copy
   `PKGBUILD` and `.SRCINFO` into it, commit and push to `master`.

For each new version: bump `pkgver` (and reset `pkgrel=1`), push a new tag, run `updpkgsums`,
regenerate `.SRCINFO`, check with `makepkg -f`, and push to the AUR repository.

## Dependencies

- Runtime: `qt6-base`, `qt6-declarative`, `kirigami`, `udisks2`, `polkit`.
- Build: `cargo` (plus a C++ compiler and `qmake6`, provided by `base-devel` and `qt6-base`).
- Optional: the `mkfs` tools of each file system (`ntfs-3g`, `exfatprogs`, `f2fs-tools`, `xfsprogs`,
  `btrfs-progs`, …); kformat greys out a format whose tool is missing and names the package to install.
