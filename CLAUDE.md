# CLAUDE.md — kformat

> Projet important. Doc : README.md + ce fichier + `docs/`.

## Description

Application KDE (Rust) pour formater disques, SSD et clés USB de façon simple et sûre.

## Stack

- Rust (workspace, edition 2021)
- `cxx-qt` + Qt6 + Kirigami (QML) pour l'interface
- `zbus` + udisks2 pour le backend (pas de daemon privilégié maison)
- polkit (autorisation, géré par udisks2)

## État actuel

**v0.1.0 publiée le 2026-10-24 ; v0.1.1 (icône de l'application) préparée le 2026-10-25, pas encore taguée ni publiée** : dépôt GitHub public (`thongor77/kformat`, MIT), tag, GitHub Release et paquet AUR `kformat`. Fonctionnel de bout en bout, validé sur une vraie clé USB : assistant en trois étapes, garde-fous (disque système, saisie du nom, identité du disque vérifiée avant chaque étape), `--device`, action « Format with kformat… » dans le widget Périphériques, verrou logind pendant l'écriture, interface en anglais/français/espagnol.

Reste : relecture de l'espagnol par un locuteur natif ; dettes mineures de la revue (voir `docs/Roadmap.md`) ; idée v2 : partitionnement. Nouvelle version : procédure dans `packaging/aur/README.md`. Historique et décisions : `docs/Decisions-Techniques.md`.

## Lancer le projet

`cargo test` (tests unitaires) ; `cargo test -p kformat-core --test loop_device -- --ignored` (test réel sur image loop, nécessite udisks2). Appli : `cargo build -p kformat-app` puis `./target/debug/kformat-app` ; test de bout en bout sans écran sur image loop : `CARGO_TARGET_DIR=target/smoke cargo build -p kformat-app --features smoke` puis `QT_QPA_PLATFORM=offscreen QT_FORCE_STDERR_LOGGING=1 target/smoke/debug/kformat-app --smoke /dev/loopN` (feature absente des builds normaux).

## Architecture

Workspace : `core` (logique + udisks, testable sans UI) et `app` (cxx-qt + QML).
Détail : `docs/Architecture.md`.

## Décisions techniques

`docs/Decisions-Techniques.md`

## Roadmap

`docs/Roadmap.md`

## Conventions spécifiques

- Langue des documents : `README.md` et `docs/` sont en **anglais** (dépôt public, visibilité) ; ce `CLAUDE.md` reste en français. Les noms de fichiers de `docs/` gardent leur nom du standard du workspace (`Decisions-Techniques.md`, `Phase0-Resultats.md`).

- Langues : anglais, français, espagnol (`LANG`/`LC_*`/`LANGUAGE`, forçable avec `KFORMAT_LANG=en|fr|es`, repli anglais). **Tout texte visible passe par `crates/core/src/messages.rs`** (une entrée = les 3 langues, sinon ça ne compile pas) ; le QML demande ses textes au contrôleur (`controller.tr("Clé")`, `tr1`, `tr2`, `tr3`). Jamais de texte en dur dans le Rust ou le QML.

- Aucune opération destructive sans confirmation explicite (retaper le nom du disque).
- Le disque contenant `/` ou `/boot` n'est jamais proposé au formatage.
- Tests de `core` avec un udisks simulé (trait) ; tests réels uniquement sur image loop.
