# kformat

Application KDE pour formater simplement disques durs, SSD et clés USB, en plusieurs formats.
Rust + cxx-qt + Kirigami, backend udisks2 (D-Bus via zbus).

## Problème

Les outils existants (GNOME Disks, KDE Partition Manager) sont complets mais peu ergonomiques
pour la tâche courante : « formater ce disque/cette clé dans tel format, sans me tromper ».

## Objectif MVP

Formatage seul (pas de partitionnement avancé) :
- assistant en 3 étapes : disque → usage → confirmation ;
- préréglages parlants (clé USB universelle, disque Linux, Windows…) ;
- garde-fous contre l'effacement du mauvais disque ;
- formats : ext4, btrfs, xfs, f2fs, exFAT, FAT32, NTFS (si ntfs-3g présent) ;
- table de partitions GPT/MBR, étiquette, effacement rapide ou sécurisé.

Voir `docs/` pour l'architecture, les décisions et la roadmap.
