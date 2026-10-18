# Architecture

## Utilisateurs
Utilisateur Plasma, de débutant à avancé, qui veut formater un support sans connaître les `mkfs`.

## Cas d'usage
1. Formater une clé USB pour qu'elle marche partout (exFAT).
2. Préparer un disque Linux (ext4/btrfs/xfs).
3. Préparer un disque pour Windows (NTFS).
4. Effacer un support avant de le céder (zéros).
5. Lancer l'assistant depuis le clic droit d'un périphérique dans Dolphin (v1.1).

## Inconnues à lever
- `Format` d'udisks2 : couverture des options par système de fichiers (btrfs, f2fs, NTFS).
- Effacement sécurisé : option `erase` d'udisks vs écriture directe.
- Retour de progression : udisks2 n'expose pas de pourcentage fiable pour Format.
- Intégration cxx-qt 0.10 avec Kirigami (QML engine, async tokio).

## Structure cible

```
kformat/
├── crates/
│   ├── core/   # modèle (Disk, Preset, FormatPlan), garde-fous, trait DiskBackend, impl udisks (zbus)
│   └── app/    # cxx-qt : QObjects exposés au QML ; qml/ (Kirigami)
└── docs/
```

Flux : QML → QObject (app) → `core::plan` (validation + garde-fous) → `DiskBackend` → udisks2 (D-Bus) → polkit.

## Sécurité
- Pas de root côté appli ; l'autorisation passe par polkit via udisks2.
- Garde-fous dans `core`, pas dans l'UI : disque système exclu, démontage préalable,
  confirmation par saisie du nom, plan affiché avant exécution.
