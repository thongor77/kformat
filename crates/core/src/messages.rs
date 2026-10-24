//! Every user-visible message, in every supported language.
//!
//! Each entry gives the English, French and Spanish text; the macro makes a
//! missing translation a compile error. Placeholders are positional (`{0}`,
//! `{1}`…) and must be the same in all three texts (checked by a test).
use crate::i18n::Lang;

macro_rules! messages {
    ($($name:ident { en: $en:literal, fr: $fr:literal, es: $es:literal $(,)? }),+ $(,)?) => {
        /// A message of the catalog; its name doubles as the key QML asks for.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Msg {
            $($name),+
        }

        impl Msg {
            pub const ALL: &'static [Msg] = &[$(Msg::$name),+];

            pub fn key(self) -> &'static str {
                match self {
                    $(Msg::$name => stringify!($name)),+
                }
            }

            pub fn from_key(key: &str) -> Option<Msg> {
                Msg::ALL.iter().copied().find(|msg| msg.key() == key)
            }

            /// How many arguments the message expects (`{0}`..`{n-1}`).
            pub fn arg_count(self) -> usize {
                let template = self.templates()[0];
                (0..)
                    .take_while(|n| template.contains(&format!("{{{n}}}")))
                    .count()
            }

            /// English, French and Spanish template, in that order.
            fn templates(self) -> [&'static str; 3] {
                match self {
                    $(Msg::$name => [$en, $fr, $es]),+
                }
            }
        }
    };
}

messages! {
    // ---- disk list ----
    SystemDiskReason {
        en: "This disk holds the running system.",
        fr: "Ce disque contient le système en cours d'exécution.",
        es: "Este disco contiene el sistema en ejecución.",
    },
    DiskTitle {
        en: "Choose a disk",
        fr: "Choisir un disque",
        es: "Elegir un disco",
    },
    DiskRefresh {
        en: "Refresh",
        fr: "Actualiser",
        es: "Actualizar",
    },
    DiskReadError {
        en: "Cannot read the disks: {0}",
        fr: "Impossible de lire les disques : {0}",
        es: "No se pueden leer los discos: {0}",
    },
    DiskNoneSelected {
        en: "No disk selected",
        fr: "Aucun disque sélectionné",
        es: "Ningún disco seleccionado",
    },
    DiskRemovable {
        en: "Removable",
        fr: "Amovible",
        es: "Extraíble",
    },
    DiskInternal {
        en: "Internal",
        fr: "Interne",
        es: "Interno",
    },
    DiskPartitionCount {
        en: "{0} partition(s)",
        fr: "{0} partition(s)",
        es: "{0} partición(es)",
    },
    DiskNoneDetected {
        en: "No disk detected",
        fr: "Aucun disque détecté",
        es: "Ningún disco detectado",
    },
    DiskPlugHint {
        en: "Plug in a disk or USB stick, then refresh.",
        fr: "Branchez un disque ou une clé USB, puis actualisez.",
        es: "Conecte un disco o una memoria USB y luego actualice.",
    },
    StartNotFound {
        en: "Disk not found: {0}",
        fr: "Disque introuvable : {0}",
        es: "Disco no encontrado: {0}",
    },
    Continue {
        en: "Continue",
        fr: "Continuer",
        es: "Continuar",
    },

    // ---- format choice ----
    OptionsTitle {
        en: "Choose the format",
        fr: "Choisir le format",
        es: "Elegir el formato",
    },
    OptionsQuestion {
        en: "What do you want to do with this disk?",
        fr: "Que voulez-vous faire de ce disque ?",
        es: "¿Qué quiere hacer con este disco?",
    },
    OptionsLabelField {
        en: "Volume name:",
        fr: "Nom du volume :",
        es: "Nombre del volumen:",
    },
    OptionsLabelPlaceholder {
        en: "Optional",
        fr: "Facultatif",
        es: "Opcional",
    },
    OptionsAdvanced {
        en: "Advanced options",
        fr: "Options avancées",
        es: "Opciones avanzadas",
    },
    OptionsFsField {
        en: "File system:",
        fr: "Système de fichiers :",
        es: "Sistema de archivos:",
    },
    OptionsUnavailableSuffix {
        en: " — unavailable",
        fr: " — indisponible",
        es: " — no disponible",
    },
    OptionsTableField {
        en: "Partition table:",
        fr: "Table de partitions :",
        es: "Tabla de particiones:",
    },
    OptionsEraseField {
        en: "Erase:",
        fr: "Effacement :",
        es: "Borrado:",
    },
    OptionsEraseQuick {
        en: "Quick",
        fr: "Rapide",
        es: "Rápido",
    },
    OptionsEraseZero {
        en: "Overwrite with zeros (slow)",
        fr: "Écraser avec des zéros (lent)",
        es: "Sobrescribir con ceros (lento)",
    },

    // ---- presets ----
    PresetUniversalUsbTitle {
        en: "Universal USB stick",
        fr: "Clé USB universelle",
        es: "Memoria USB universal",
    },
    PresetUniversalUsbDescription {
        en: "exFAT: readable on Windows, macOS and Linux, files larger than 4 GB.",
        fr: "exFAT : lisible sur Windows, macOS et Linux, fichiers de plus de 4 Go.",
        es: "exFAT: legible en Windows, macOS y Linux, archivos de más de 4 GB.",
    },
    PresetLinuxTitle {
        en: "Linux disk",
        fr: "Disque Linux",
        es: "Disco para Linux",
    },
    PresetLinuxDescription {
        en: "ext4: the standard Linux file system.",
        fr: "ext4 : le système de fichiers standard de Linux.",
        es: "ext4: el sistema de archivos estándar de Linux.",
    },
    PresetWindowsTitle {
        en: "Windows disk",
        fr: "Disque Windows",
        es: "Disco para Windows",
    },
    PresetWindowsDescription {
        en: "NTFS: for a disk used mostly on Windows.",
        fr: "NTFS : pour un disque utilisé surtout sous Windows.",
        es: "NTFS: para un disco que se usa sobre todo en Windows.",
    },
    PresetLegacyFat32Title {
        en: "Maximum compatibility (FAT32)",
        fr: "Compatibilité maximale (FAT32)",
        es: "Máxima compatibilidad (FAT32)",
    },
    PresetLegacyFat32Description {
        en: "FAT32: older devices, TVs, car stereos (files limited to 4 GB).",
        fr: "FAT32 : appareils anciens, TV, autoradios (fichiers limités à 4 Go).",
        es: "FAT32: dispositivos antiguos, televisores, autorradios (archivos limitados a 4 GB).",
    },
    FsUnavailable {
        en: "{0} was not found: install the {1} package to use {2}.",
        fr: "{0} est introuvable : installez le paquet {1} pour utiliser {2}.",
        es: "No se encontró {0}: instale el paquete {1} para usar {2}.",
    },
    TableGptName {
        en: "GPT (recommended)",
        fr: "GPT (recommandé)",
        es: "GPT (recomendado)",
    },
    TableMbrName {
        en: "MBR (older devices)",
        fr: "MBR (appareils anciens)",
        es: "MBR (dispositivos antiguos)",
    },

    // ---- label validation ----
    LabelTooLong {
        en: "The name cannot be longer than {0} characters{1}.",
        fr: "Le nom ne peut pas dépasser {0} caractères{1}.",
        es: "El nombre no puede superar los {0} caracteres{1}.",
    },
    LabelForbiddenChar {
        en: "The character “{0}” is not allowed in this name{1}.",
        fr: "Le caractère « {0} » n'est pas autorisé dans ce nom{1}.",
        es: "El carácter «{0}» no está permitido en este nombre{1}.",
    },
    LabelForFs {
        en: " for {0}",
        fr: " pour {0}",
        es: " para {0}",
    },
    UnknownFs {
        en: "Unknown file system.",
        fr: "Système de fichiers inconnu.",
        es: "Sistema de archivos desconocido.",
    },

    // ---- confirmation screen ----
    ConfirmTitle {
        en: "Confirm the format",
        fr: "Confirmer le formatage",
        es: "Confirmar el formateo",
    },
    ConfirmTypePlaceholder {
        en: "Type “{0}” to confirm",
        fr: "Tapez « {0} » pour confirmer",
        es: "Escriba «{0}» para confirmar",
    },
    ConfirmButton {
        en: "Format permanently",
        fr: "Formater définitivement",
        es: "Formatear definitivamente",
    },
    ConfirmWarning {
        en: "All data on this disk will be permanently erased. This cannot be undone.",
        fr: "Toutes les données de ce disque seront définitivement effacées. Cette action est irréversible.",
        es: "Todos los datos de este disco se borrarán de forma permanente. Esta acción es irreversible.",
    },
    ConfirmSleepNotBlocked {
        en: "Sleep cannot be blocked during the format. Do not close the lid or put the computer to sleep before it finishes.",
        fr: "La mise en veille ne peut pas être bloquée pendant le formatage. Ne fermez pas le capot et ne mettez pas l'ordinateur en veille avant la fin.",
        es: "No se puede bloquear la suspensión durante el formateo. No cierre la tapa ni suspenda el equipo antes de que termine.",
    },
    ConfirmWillErase {
        en: "What will be erased",
        fr: "Ce qui sera effacé",
        es: "Lo que se borrará",
    },
    ConfirmNoPartitions {
        en: "No partition found on this disk.",
        fr: "Aucune partition détectée sur ce disque.",
        es: "No se detectó ninguna partición en este disco.",
    },
    ConfirmWillBeUnmounted {
        en: " (will be unmounted)",
        fr: " (sera démontée)",
        es: " (se desmontará)",
    },
    ConfirmNewFormat {
        en: "New format",
        fr: "Nouveau format",
        es: "Nuevo formato",
    },
    ConfirmFormatSummary {
        en: "{0} · {1} table{2}",
        fr: "{0} · table {1}{2}",
        es: "{0} · tabla {1}{2}",
    },
    ConfirmNamed {
        en: " · name {0}",
        fr: " · nom {0}",
        es: " · nombre {0}",
    },
    ConfirmUnnamed {
        en: " · no name",
        fr: " · sans nom",
        es: " · sin nombre",
    },
    ConfirmSteps {
        en: "Steps",
        fr: "Étapes",
        es: "Pasos",
    },
    Quoted {
        en: "“{0}”",
        fr: "« {0} »",
        es: "«{0}»",
    },

    // ---- plan summary ----
    StepUnmount {
        en: "Unmounting {0}",
        fr: "Démontage de {0}",
        es: "Desmontando {0}",
    },
    StepTableQuick {
        en: "Creating the {0} partition table",
        fr: "Création de la table de partitions {0}",
        es: "Creando la tabla de particiones {0}",
    },
    StepTableZero {
        en: "Erasing everything with zeros, then creating the {0} table",
        fr: "Effacement complet par des zéros, puis création de la table {0}",
        es: "Borrando todo con ceros y creando después la tabla {0}",
    },
    StepPartition {
        en: "Creating the partition and formatting as {0}",
        fr: "Création de la partition et formatage en {0}",
        es: "Creando la partición y formateando como {0}",
    },
    EraseQuickText {
        en: "Quick erase: old data can still be recovered with specialised tools.",
        fr: "Effacement rapide : les anciennes données restent récupérables avec des outils spécialisés.",
        es: "Borrado rápido: los datos anteriores aún se pueden recuperar con herramientas especializadas.",
    },
    EraseZeroText {
        en: "Full erase with zeros: old data cannot be recovered (slow).",
        fr: "Effacement complet par des zéros : les anciennes données sont irrécupérables (long).",
        es: "Borrado completo con ceros: los datos anteriores no se pueden recuperar (lento).",
    },

    // ---- progress and result ----
    RunTitleRunning {
        en: "Formatting in progress",
        fr: "Formatage en cours",
        es: "Formateo en curso",
    },
    RunTitleDone {
        en: "Done",
        fr: "Terminé",
        es: "Terminado",
    },
    RunTitleFailed {
        en: "Failed",
        fr: "Échec",
        es: "Error",
    },
    RunDoNotUnplug {
        en: "Do not unplug the disk and do not close the application.",
        fr: "Ne débranchez pas le disque et ne fermez pas l'application.",
        es: "No desconecte el disco ni cierre la aplicación.",
    },
    RunFinish {
        en: "Finish",
        fr: "Terminer",
        es: "Terminar",
    },
    RunBackToList {
        en: "Back to the list",
        fr: "Retour à la liste",
        es: "Volver a la lista",
    },
    FormatSucceeded {
        en: "The format completed successfully.",
        fr: "Le formatage s'est terminé avec succès.",
        es: "El formateo se completó correctamente.",
    },
    NoPlanToRun {
        en: "No plan to run: start again.",
        fr: "Aucun plan à exécuter : recommencez.",
        es: "No hay ningún plan que ejecutar: empiece de nuevo.",
    },
    SmokeDeviceRefused {
        en: "This disk is not allowed in test mode.",
        fr: "Ce disque n'est pas autorisé dans ce mode de test.",
        es: "Este disco no está permitido en el modo de pruebas.",
    },
    InternalError {
        en: "Internal error: the operation failed unexpectedly. Restart the application.",
        fr: "Erreur interne : l'opération a échoué de façon inattendue. Relancez l'application.",
        es: "Error interno: la operación falló de forma inesperada. Reinicie la aplicación.",
    },

    // ---- errors ----
    PlanSystemDisk {
        en: "{0} holds the running system and cannot be formatted.",
        fr: "{0} contient le système en cours d'exécution : il ne peut pas être formaté.",
        es: "{0} contiene el sistema en ejecución y no se puede formatear.",
    },
    PlanDiskTooSmall {
        en: "This disk is too small to be formatted as {0}.",
        fr: "Ce disque est trop petit pour être formaté en {0}.",
        es: "Este disco es demasiado pequeño para formatearlo como {0}.",
    },
    PlanMbrTooLarge {
        en: "An MBR table cannot handle a disk this large: choose GPT.",
        fr: "Une table MBR ne gère pas un disque de cette taille : choisissez GPT.",
        es: "Una tabla MBR no admite un disco de este tamaño: elija GPT.",
    },
    PlanInvalidDisk {
        en: "This disk has no usable device name.",
        fr: "Ce disque n'a pas de nom de périphérique exploitable.",
        es: "Este disco no tiene un nombre de dispositivo utilizable.",
    },
    PlanConfirmationMismatch {
        en: "The name you typed does not match the disk.",
        fr: "Le nom saisi ne correspond pas au disque.",
        es: "El nombre escrito no coincide con el disco.",
    },
    BackendBus {
        en: "Cannot talk to udisks2: {0}",
        fr: "Impossible de communiquer avec udisks2 : {0}",
        es: "No se puede comunicar con udisks2: {0}",
    },
    BackendStepFailed {
        en: "Step {0} of {1} failed: {2}.{3}",
        fr: "L'étape {0} sur {1} a échoué : {2}.{3}",
        es: "El paso {0} de {1} falló: {2}.{3}",
    },
    BackendPartialWarning {
        en: " The disk may be in an incomplete state: run the format again.",
        fr: " Le disque peut être dans un état incomplet : relancez le formatage.",
        es: " El disco puede haber quedado en un estado incompleto: vuelva a ejecutar el formateo.",
    },
    BackendDiskGone {
        en: "{0} is no longer connected.",
        fr: "{0} n'est plus connecté.",
        es: "{0} ya no está conectado.",
    },
    BackendDiskBecameSystem {
        en: "{0} is now used by the system: operation cancelled.",
        fr: "{0} est désormais utilisé par le système : opération annulée.",
        es: "{0} ahora lo usa el sistema: operación cancelada.",
    },
    BackendDiskChanged {
        en: "{0} is no longer the disk you confirmed: operation cancelled.",
        fr: "{0} n'est plus le disque que vous avez confirmé : opération annulée.",
        es: "{0} ya no es el disco que confirmó: operación cancelada.",
    },
    BackendInterrupted {
        en: "{0} The format stopped after {1} of {2} step(s): the disk may be in an incomplete state, run the format again.",
        fr: "{0} Le formatage s'est arrêté après {1} étape(s) sur {2} : le disque peut être dans un état incomplet, relancez le formatage.",
        es: "{0} El formateo se detuvo tras {1} de {2} paso(s): el disco puede haber quedado en un estado incompleto, vuelva a ejecutar el formateo.",
    },
    BackendParse {
        en: "Unexpected udisks2 response ({0}): {1}",
        fr: "Réponse inattendue d'udisks2 ({0}) : {1}",
        es: "Respuesta inesperada de udisks2 ({0}): {1}",
    },
    SessionUnknownOption {
        en: "Unknown option: {0}.",
        fr: "Option inconnue : {0}.",
        es: "Opción desconocida: {0}.",
    },
    SessionDiskNotFound {
        en: "{0} was not found. Refresh the disk list.",
        fr: "{0} est introuvable. Actualisez la liste des disques.",
        es: "No se encontró {0}. Actualice la lista de discos.",
    },
    CliInvalidDevice {
        en: "Invalid device: “{0}” (expected: /dev/sdX).",
        fr: "Périphérique invalide : « {0} » (attendu : /dev/sdX).",
        es: "Dispositivo no válido: «{0}» (se esperaba: /dev/sdX).",
    },
}

/// Replaces `{0}`, `{1}`… in `template` with `args` in a single pass, so text
/// that happens to contain a placeholder is never substituted twice.
/// A placeholder without a matching argument is kept as written.
fn render(template: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let argument = tail
            .find('}')
            .and_then(|end| tail[1..end].parse::<usize>().ok().map(|n| (n, end)))
            .and_then(|(n, end)| args.get(n).map(|arg| (*arg, end)));
        match argument {
            Some((arg, end)) => {
                out.push_str(arg);
                rest = &tail[end + 1..];
            }
            None => {
                out.push('{');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The message in `lang`, with `args` substituted.
pub fn tr(lang: Lang, msg: Msg, args: &[&str]) -> String {
    let index = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    render(msg.templates()[index], args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// The `{n}` placeholders used by a template.
    fn placeholders(template: &str) -> BTreeSet<usize> {
        let mut found = BTreeSet::new();
        let mut rest = template;
        while let Some(start) = rest.find('{') {
            let tail = &rest[start + 1..];
            let Some(end) = tail.find('}') else { break };
            if let Ok(n) = tail[..end].parse::<usize>() {
                found.insert(n);
            }
            rest = &tail[end + 1..];
        }
        found
    }

    #[test]
    fn every_message_uses_the_same_placeholders_in_every_language() {
        for msg in Msg::ALL {
            let [en, fr, es] = msg.templates();
            assert_eq!(placeholders(en), placeholders(fr), "{} en/fr", msg.key());
            assert_eq!(placeholders(en), placeholders(es), "{} en/es", msg.key());
        }
    }

    #[test]
    fn no_translation_is_empty_unless_it_is_a_pure_suffix() {
        for msg in Msg::ALL {
            for text in msg.templates() {
                assert!(!text.trim().is_empty(), "{} has an empty text", msg.key());
            }
        }
    }

    #[test]
    fn placeholders_are_numbered_without_gaps() {
        for msg in Msg::ALL {
            let used = placeholders(msg.templates()[0]);
            let expected: BTreeSet<usize> = (0..used.len()).collect();
            assert_eq!(used, expected, "{}", msg.key());
        }
    }

    #[test]
    fn keys_are_unique_and_round_trip() {
        let keys: BTreeSet<_> = Msg::ALL.iter().map(|m| m.key()).collect();
        assert_eq!(keys.len(), Msg::ALL.len());
        for msg in Msg::ALL {
            assert_eq!(Msg::from_key(msg.key()), Some(*msg));
        }
        assert_eq!(Msg::from_key("NoSuchMessage"), None);
    }

    #[test]
    fn the_three_languages_really_differ() {
        let differing = Msg::ALL
            .iter()
            .filter(|m| {
                let [en, fr, es] = m.templates();
                en != fr && fr != es && en != es
            })
            .count();
        assert!(
            differing * 10 > Msg::ALL.len() * 8,
            "too many untranslated messages"
        );
    }

    #[test]
    fn arguments_are_substituted_in_place() {
        assert_eq!(
            tr(Lang::En, Msg::StepUnmount, &["/dev/sda1"]),
            "Unmounting /dev/sda1"
        );
        assert_eq!(
            tr(Lang::Fr, Msg::StepUnmount, &["/dev/sda1"]),
            "Démontage de /dev/sda1"
        );
        assert_eq!(
            tr(Lang::Es, Msg::BackendStepFailed, &["2", "3", "busy", ""]),
            "El paso 2 de 3 falló: busy."
        );
    }

    #[test]
    fn substituted_text_is_never_substituted_again() {
        let out = tr(Lang::En, Msg::BackendBus, &["{0} {1}"]);
        assert_eq!(out, "Cannot talk to udisks2: {0} {1}");
    }

    #[test]
    fn a_missing_argument_keeps_its_placeholder() {
        assert_eq!(tr(Lang::En, Msg::StepUnmount, &[]), "Unmounting {0}");
    }

    #[test]
    fn plain_messages_need_no_arguments() {
        assert_eq!(tr(Lang::Fr, Msg::Continue, &[]), "Continuer");
        assert_eq!(tr(Lang::Es, Msg::Continue, &[]), "Continuar");
        assert_eq!(tr(Lang::En, Msg::Continue, &[]), "Continue");
    }

    #[test]
    fn arg_count_matches_the_highest_placeholder() {
        assert_eq!(Msg::Continue.arg_count(), 0);
        assert_eq!(Msg::StepUnmount.arg_count(), 1);
        assert_eq!(Msg::BackendInterrupted.arg_count(), 3);
        assert_eq!(Msg::BackendStepFailed.arg_count(), 4);
    }
}
