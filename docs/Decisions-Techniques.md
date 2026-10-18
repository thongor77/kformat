# Décisions techniques

## 2026-10-18 — Interface : cxx-qt + Kirigami
Pont Rust/Qt officiel (KDAB), rendu natif Plasma. Écarté : qmetaobject-rs (moins maintenu), iced/egui (non natifs).

## 2026-10-18 — Backend : udisks2 via zbus
Fait déjà lister, démonter, formater et autoriser via polkit. Évite un daemon root (contrairement à netmnt).
Repli `mkfs.*` + pkexec seulement pour ce qu'udisks ne couvre pas.

## 2026-10-18 — Périmètre MVP : formatage seul
Le partitionnement (redimensionnement, multi-partitions) double la complexité ; reporté en v2.

## 2026-10-18 — zbus 5.x (pas 6.0)
zbus 6.0.0 supprime l'API `blocking` et renomme des types. On reste sur 5.x tant que la 6 n'est pas stabilisée.

## 2026-10-18 — NTFS conditionnel
`mkfs.ntfs` absent par défaut : l'option NTFS est détectée à l'exécution et grisée avec explication.

## 2026-10-18 — Garde-fous par le typage (phase 1)
`build_plan` valide (disque système, FS disponible, libellé, tailles, MBR > 2 Tio) et renvoie un `FormatPlan`.
Seul `FormatPlan::confirm(nom_du_disque)` produit un `ConfirmedPlan`, et seul un `ConfirmedPlan` est exécutable par un backend.
`DiskBackend::execute` relit la liste des disques juste avant d'écrire : disque disparu ou devenu disque système → abandon.

## 2026-10-18 — Type de partition MBR explicite
Sans type, udisks met 0x83 (Linux) même pour FAT32 en MBR. `FsType::mbr_type()` impose 0x0c (FAT32), 0x07 (exFAT/NTFS), 0x83 sinon. Vérifié sur image loop.

## 2026-10-18 — Layout réel d'udisks2
La propriété `Table` d'une partition est sur l'interface `Partition`, pas sur `Block`. Un test synthétique erroné l'avait masqué ; le test d'intégration sur image loop l'a révélé. Les tests synthétiques doivent refléter le layout observé sur le vrai bus.

## 2026-10-18 — Revue de code phase 1 : détection « disque système » inversée
Revue (agent rust-reviewer) : verdict BLOCK sur la première version. Corrigé :
- Un montage n'est « utilisateur » que sous `/run/media/`, `/media/` ou `/mnt/` ; tout le reste (`/var/lib/…`, `/home/…`, `[SWAP]`) compte comme système. Liste blanche d'usage, plus liste noire de chemins système.
- Racine sur LUKS/LVM/md : les montages des périphériques empilés (`/sys/class/block/<n>/holders`, transitif) sont rattachés au disque (`Disk.holder_mounts`).
- Swap : lu via l'interface udisks `Swapspace.Active` (la chaîne `[SWAP]` est une convention lsblk, jamais fournie par udisks).
- Identité : `Disk.id` (Id udisks) ; avant CHAQUE étape on vérifie même disque + non système, et avant la première étape aussi la disposition des partitions (`same_drive` / `same_media`). Protège du disque débranché puis remplacé sous le même nom.
- `DiskBackend` scellé (`sealed::StepRunner`) : `run_step` n'est pas appelable de l'extérieur, aucun contournement de `build_plan` → `confirm` → `execute`.
- Analyse udisks stricte : propriété illisible = erreur (`BackendError::Parse`), jamais une partition ou un montage silencieusement perdu ; nom de périphérique vide refusé.
- `Unmount` vérifie que la partition appartient bien au disque du plan.

## Reporté (revue, sévérité moyenne/basse) — voir Roadmap
Confirmation renforcée pour disque non amovible, `#[source]` sur les erreurs, retry après `Format` avant `CreatePartitionAndFormat`, timeout/annulation de l'effacement par zéros, tri naturel (`sdb2` avant `sdb10`), `#[non_exhaustive]`/docs publiques, `Unmount` d'une partition montée plusieurs fois.

## 2026-10-18 — Rendu Kirigami validé
Le spike (QGuiApplication + Kirigami, style par défaut) a un rendu jugé satisfaisant par l'utilisateur sous Plasma/Wayland : pas besoin de contourner l'absence de QApplication. Note : lancer le binaire directement (`./target/debug/kformat-spike-hello`) plutôt que `cargo run`.

## 2026-10-18 — UI : JSON plutôt qu'un modèle de liste Qt
`core::view` sérialise les disques en JSON (camelCase), exposé en propriété `disksJson` du `DiskController` et lu par `JSON.parse` en QML. Plus simple qu'un QAbstractListModel pour une liste de quelques éléments, et la logique de présentation (tailles, raison du blocage, tri) est testée sans Qt. À revoir si la liste devient dynamique/volumineuse.

## 2026-10-18 — Appels D-Bus hors du thread UI
`DiskController::refresh` lance un `std::thread` et renvoie le résultat via `CxxQtThread::queue`. `busy` empêche les rafraîchissements concurrents.

## 2026-10-18 — Machine à états du contrôleur
`phase` : idle → planning → ready → running → done | failed. Le plan validé est gardé côté Rust (`Option<FormatPlan>`), jamais reconstruit depuis le QML. `confirmAndRun(typed)` appelle `FormatPlan::confirm` : même si l'interface laissait passer une saisie fausse, c'est `core` qui refuse. `reset()` et `preparePlan()` sont ignorés pendant `running`. La fenêtre refuse de se fermer pendant `running`.

## 2026-10-18 — Test bout en bout sans écran (feature `smoke`, désactivée par défaut)
Revue de code : `--smoke` accessible en production permettait, avec `QT_QPA_PLATFORM=offscreen`, de formater un disque sans saisie. Désormais le hook n'existe que dans un build `--features smoke` (le QML de test n'est même pas empaqueté sinon), et il refuse tout périphérique hors `/dev/loop*`. Build dédié : `CARGO_TARGET_DIR=target/smoke cargo build -p kformat-app --features smoke`.
`kformat-app --smoke <loop>` (plateforme offscreen) exécute prepare → mauvaise confirmation (refusée) → bonne confirmation → écriture, via les mêmes invokables que l'interface. Le drapeau `--smoke` ajoute seulement les périphériques loop à la liste de `UdisksBackend` ; les garde-fous de `core` restent actifs (disque système refusé).

## 2026-10-18 — Revue de code phase 2 (verdict WARNING, corrigé)
- Panique dans un thread : `core::guard::run_guarded` convertit toute panique en message d'erreur (refresh, prepare, exécution) — sinon l'application restait bloquée en `running`, fenêtre infermable.
- Plan périmé : `request_id` incrémenté par `prepare_plan` et `reset`, résultat ignoré s'il ne correspond plus ; `OptionsPage` appelle `reset()` à sa création et vérifie que le plan reçu concerne bien son disque.
- Retour arrière pendant l'écriture : `ConfirmPage` est remplacée (`replace`) par `RunPage`, qui refuse `backRequested` tant que `running`.
- Échec après la 1re étape : le message précise que le disque peut être dans un état incomplet.

## 2026-10-18 — Identité du disque : l'objet Drive, jamais `Block.Id` (bug trouvé sur vraie clé)
Premier essai réel (clé USB 31 Go) : le garde-fou « disque changé » a annulé le formatage à la dernière étape, après la création de la table GPT, laissant la clé à moitié formatée. Cause : `Disk.id` venait de `Block.Id`, que udisks dérive du *contenu* du disque (vide après réécriture de la table) ; les images loop, sans Drive, ne pouvaient pas révéler le bug.
Correctif : `Disk.id` = chemin de l'objet Drive (fabricant/modèle/numéro de série, stable au reformatage), vide pour un loop. Tests de non-régression : l'identité survit à un changement de `Block.Id`, et deux Drives distincts donnent deux identités.
Complément : une vérification qui échoue *après* la première étape renvoie `BackendError::Interrupted` (étapes faites / total + cause), message FR « le disque peut être dans un état incomplet ». **Leçon : tout garde-fou d'identité doit être testé sur du vrai matériel, pas seulement sur loop.**

## 2026-10-18 — Intégration bureau : action Solid, pas service menu Dolphin
Les service menus Dolphin ne concernent que fichiers et dossiers, pas les entrées de « Périphériques ». Le mécanisme KDE pour un périphérique est une action Solid (`solid/actions/*.desktop`), proposée dans le widget des périphériques. Format calqué sur les actions officielles de KDE (`/usr/share/solid/actions/`). Piège évité : dans `Exec`, `%f` est le point de montage et `%d` le nœud de périphérique (`/dev/sda1`) ; on utilise `%d`. `desktop-file-validate` signale des erreurs sur ce type de fichier, y compris sur les fichiers officiels : bruit du validateur générique.

## 2026-10-18 — `--device` : validation stricte
Seuls les nœuds simples `/dev/<nom>` sont acceptés (pas de `..`, de sous-dossier, d'espace ni de `;`). L'option ne contourne rien : le disque est relu, les garde-fous de `core` s'appliquent, et la confirmation par saisie du nom reste obligatoire.

## 2026-10-18 — Licence MIT (à confirmer par le mainteneur)
`Cargo.toml` annonçait GPL-3.0-or-later sans qu'aucun choix n'ait été fait et sans fichier LICENSE. Remplacé par MIT (fichier `LICENSE`), par cohérence avec `netmnt`. Aucune publication n'a eu lieu : changeable sans conséquence tant que le dépôt n'est pas public.

## 2026-10-18 — Paquet AUR
`packaging/aur/PKGBUILD` suit les conventions de `netmnt` (archive du tag GitHub, `prepare` avec `cargo fetch --locked`, `build` frozen, `check` = tests unitaires seulement car les tests udisks sont `#[ignore]` et ne peuvent pas tourner en chroot, `package` à la main). Les chemins du répertoire de build sont réécrits (`-ffile-prefix-map`, `--remap-path-prefix`) pour qu'aucun `$srcdir` ne reste dans le binaire. Dépendances d'exécution : qt6-base, qt6-declarative, kirigami, udisks2, polkit ; les outils `mkfs` sont optionnels (kformat grise le format et nomme le paquet). Testé avec `makepkg` depuis une archive locale ; `sha256sums=('SKIP')` jusqu'à l'existence du tag.
