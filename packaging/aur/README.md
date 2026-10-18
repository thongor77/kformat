# Paquet AUR de kformat

`PKGBUILD` construit kformat depuis l'archive du tag GitHub `v<pkgver>`.

## Tester le paquet localement (sans publier)

Le tag n'existe pas tant que le dépôt n'est pas publié. Pour tester le `PKGBUILD` quand même :

```bash
# depuis ~/claude-projects : archive du projet sans les dossiers de build
tar --exclude='kformat/target' --exclude='kformat/spikes/*/target' \
    --transform 's,^kformat,kformat-0.1.0,' -czf /tmp/kformat-0.1.0.tar.gz kformat
# copier le PKGBUILD dans un dossier de travail et pointer `source=` vers le fichier local
mkdir /tmp/kformat-aur && cp kformat/packaging/aur/PKGBUILD /tmp/kformat-aur/
sed -i 's|^source=.*|source=("$pkgname-$pkgver.tar.gz::file:///tmp/kformat-0.1.0.tar.gz")|' /tmp/kformat-aur/PKGBUILD
cd /tmp/kformat-aur && makepkg -f      # produit kformat-0.1.0-1-x86_64.pkg.tar.zst
pacman -Qlp kformat-0.1.0-1-x86_64.pkg.tar.zst   # vérifier le contenu
```

## Publier (à faire par le mainteneur)

1. Créer le dépôt GitHub `thongor77/kformat` et y pousser le code.
2. Poser le tag : `git tag v0.1.0 && git push origin v0.1.0`.
3. Dans un dossier de travail propre : copier `PKGBUILD`, puis `updpkgsums`
   (remplace `SKIP` par le vrai `sha256sum`).
4. Régénérer les métadonnées : `makepkg --printsrcinfo > .SRCINFO`.
5. Cloner `ssh://aur@aur.archlinux.org/kformat.git`, y copier `PKGBUILD` et `.SRCINFO`, commit, push.

À chaque nouvelle version : incrémenter `pkgver` (et remettre `pkgrel=1`), nouveau tag, `updpkgsums`, `.SRCINFO`, push.

## Dépendances

- Exécution : `qt6-base`, `qt6-declarative`, `kirigami`, `udisks2`, `polkit`.
- Compilation : `cargo` (+ compilateur C++ et `qmake6`, fournis par `base-devel` et `qt6-base`).
- Optionnelles : les outils `mkfs` de chaque système de fichiers (`ntfs-3g`, `exfatprogs`, `f2fs-tools`,
  `xfsprogs`, `btrfs-progs`, …) ; kformat grise un format dont l'outil manque et indique le paquet à installer.
