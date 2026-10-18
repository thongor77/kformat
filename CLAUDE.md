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

Phase 0 (spikes) terminée le 2026-10-18 : udisks Format OK pour 6 FS, cxx-qt + Kirigami OK, progression via Job. Phase 1 (crate `core`) livrée et relue le 2026-10-18 (blocages de la revue corrigés). Phase 2 en cours : étapes 1 à 3 faites (liste, choix du format, confirmation + progression) ; revue de code de la crate `app` faite et corrigée. Essai réel sur clé USB réussi le 2026-10-18. Phase 4 en cours : option `--device`, lanceur et action Solid faits (installation utilisateur : `scripts/install-user.sh`) ; paquet AUR (`packaging/aur/`) construit et testé localement. Reste : vérifier l'action dans le widget des périphériques, puis publier (GitHub + AUR, par le mainteneur). Résultats phase 0 : `docs/Phase0-Resultats.md`.

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

- Aucune opération destructive sans confirmation explicite (retaper le nom du disque).
- Le disque contenant `/` ou `/boot` n'est jamais proposé au formatage.
- Tests de `core` avec un udisks simulé (trait) ; tests réels uniquement sur image loop.
