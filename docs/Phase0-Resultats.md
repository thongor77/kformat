# Phase 0 — Résultats des spikes (2026-10-18)

Environnement : Arch Linux, Qt 6.12, Kirigami 6.30, udisks2 2.11.2, Rust stable. Tests sur image loop (jamais sur un vrai disque).

## Spike 1 — `Block.Format` d'udisks2 par système de fichiers

| FS | Résultat |
|---|---|
| ext4, btrfs, xfs, f2fs, exfat, vfat | OK, `IdType` et `IdLabel` corrects, option `label` honorée |
| ntfs | Échec propre : « executable mkfs.ntfs not found » (paquet `ntfs-3g` absent) |

- Aucune invite polkit sur un périphérique loop (à re-tester sur une vraie clé USB : l'agent KDE devrait s'afficher).
- Flux table de partitions : `Block.Format('gpt')` puis `PartitionTable.CreatePartitionAndFormat(offset, size=0 → tout l'espace, type, name, {}, fs, {label})` fonctionne en un appel.
- Signature utile : `CreatePartitionAndFormat` = `ttssa{sv}sa{sv}`.
- Conséquence : NTFS doit être détecté à l'exécution (présence de `mkfs.ntfs`) et grisé avec une explication si absent.

## Spike 2 — cxx-qt 0.10 + Kirigami

- Build 100 % Cargo, sans CMake (`qmake6` + `ninja` + `clang`/`g++` suffisent). Aucun problème de compilation.
- Module QML `org.kformat.spike` chargé, `Kirigami.ApplicationWindow` s'ouvre sous Wayland, sans avertissement.
- Un QObject Rust (`DiskModel`) expose une propriété et une méthode appelable depuis QML ; appel D-Bus udisks réussi.
- `cxx-qt-lib` 0.10 n'expose que `QGuiApplication` (pas `QApplication`). Les styles `org.kde.desktop` et `org.kde.breeze` se lancent sans erreur ; le rendu visuel reste à vérifier à l'œil.

## Spike 3 — Progression

- Pendant `Format` avec `erase=zero`, udisks crée un objet `Job` (`/org/freedesktop/UDisks2/jobs/N`) avec `Operation="format-erase"`, `Bytes`, `Progress`, `ProgressValid=true`.
- Progression observable pour l'effacement. À confirmer sur un vrai support (l'image sparse a fini avant tout échantillonnage utile).
- Sans effacement, l'opération est quasi instantanée : un indicateur indéterminé suffit.

## Décision induite : zbus 5, pas zbus 6

zbus 6.0.0 est une refonte : plus de module `blocking`, types renommés (`OwnedVariant`). On reste sur **zbus 5.x** (mature, API bloquante disponible). À réévaluer quand la 6.x se stabilise.

## Inconnues restantes
- ~~Rendu visuel Kirigami avec `QGuiApplication` seul~~ : validé à l'œil par l'utilisateur le 2026-10-18 (rendu satisfaisant sous Plasma/Wayland).
- Polkit sur périphérique amovible réel.
- Progression réelle sur gros support.
- Appel D-Bus bloquant dans le thread UI : à déplacer dans un thread (`CxxQtThread`) en phase 2.

Code des spikes : `spikes/hello/` (jetable, hors workspace).
