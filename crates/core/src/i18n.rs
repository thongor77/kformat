//! Languages kformat is translated into, and how the user's language is picked.

/// A supported interface language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    En,
    Fr,
    Es,
}

impl Lang {
    pub const ALL: [Lang; 3] = [Lang::En, Lang::Fr, Lang::Es];

    /// Used when the environment names no supported language.
    pub const FALLBACK: Lang = Lang::En;

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
            Lang::Es => "es",
        }
    }

    /// Language of a locale tag such as `fr_FR.UTF-8`, `es-MX` or `en`.
    pub fn from_locale(tag: &str) -> Option<Lang> {
        let language: String = tag
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect::<String>()
            .to_ascii_lowercase();
        Lang::ALL.into_iter().find(|lang| lang.code() == language)
    }

    /// Picks the language from environment variables, read through `getenv`.
    ///
    /// Order: `KFORMAT_LANG` (explicit override), then the locale from `LC_ALL`,
    /// `LC_MESSAGES` or `LANG` (first one set), then the `LANGUAGE` priority list
    /// unless the locale is `C`/`POSIX`. Anything unsupported falls back to English.
    pub fn detect(getenv: &dyn Fn(&str) -> Option<String>) -> Lang {
        let set = |name: &str| getenv(name).filter(|value| !value.is_empty());

        if let Some(lang) = set("KFORMAT_LANG").and_then(|tag| Lang::from_locale(&tag)) {
            return lang;
        }
        let locale = ["LC_ALL", "LC_MESSAGES", "LANG"].into_iter().find_map(set);
        let neutral = |tag: &str| matches!(tag, "C" | "POSIX");
        if locale.as_deref().is_some_and(neutral) {
            return Lang::FALLBACK;
        }
        if let Some(list) = set("LANGUAGE") {
            if let Some(lang) = list.split(':').find_map(Lang::from_locale) {
                return lang;
            }
        }
        locale
            .and_then(|tag| Lang::from_locale(&tag))
            .unwrap_or(Lang::FALLBACK)
    }

    /// Language of the running process's environment.
    pub fn from_env() -> Lang {
        Lang::detect(&|name| std::env::var(name).ok())
    }

    pub fn decimal_separator(self) -> char {
        match self {
            Lang::En => '.',
            Lang::Fr | Lang::Es => ',',
        }
    }

    /// Unit names for bytes, MB, GB and TB (decimal, as disk makers count).
    pub fn size_units(self) -> [&'static str; 4] {
        match self {
            Lang::Fr => ["o", "Mo", "Go", "To"],
            Lang::En | Lang::Es => ["B", "MB", "GB", "TB"],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detect(vars: &[(&str, &str)]) -> Lang {
        Lang::detect(&|name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        })
    }

    #[test]
    fn locale_tags_map_to_languages() {
        assert_eq!(Lang::from_locale("fr_FR.UTF-8"), Some(Lang::Fr));
        assert_eq!(Lang::from_locale("es-MX"), Some(Lang::Es));
        assert_eq!(Lang::from_locale("en"), Some(Lang::En));
        assert_eq!(Lang::from_locale("EN_us@euro"), Some(Lang::En));
    }

    #[test]
    fn unsupported_or_neutral_tags_map_to_nothing() {
        for tag in ["de_DE.UTF-8", "C", "POSIX", "", "fra", "frog", "_FR"] {
            assert_eq!(Lang::from_locale(tag), None, "{tag:?}");
        }
    }

    #[test]
    fn lang_is_read_from_lang() {
        assert_eq!(detect(&[("LANG", "fr_FR.UTF-8")]), Lang::Fr);
        assert_eq!(detect(&[("LANG", "es_ES.UTF-8")]), Lang::Es);
    }

    #[test]
    fn lc_all_beats_lc_messages_beats_lang() {
        let all = [
            ("LC_ALL", "es_ES.UTF-8"),
            ("LC_MESSAGES", "fr_FR"),
            ("LANG", "en_US"),
        ];
        assert_eq!(detect(&all), Lang::Es);
        assert_eq!(detect(&all[1..]), Lang::Fr);
        assert_eq!(detect(&all[2..]), Lang::En);
    }

    #[test]
    fn explicit_override_beats_everything() {
        let vars = [
            ("KFORMAT_LANG", "es"),
            ("LC_ALL", "fr_FR.UTF-8"),
            ("LANGUAGE", "fr"),
        ];
        assert_eq!(detect(&vars), Lang::Es);
    }

    #[test]
    fn language_list_picks_the_first_supported_entry() {
        let vars = [("LANG", "en_US.UTF-8"), ("LANGUAGE", "de:es:fr")];
        assert_eq!(detect(&vars), Lang::Es);
    }

    #[test]
    fn language_list_is_ignored_for_the_c_locale() {
        assert_eq!(detect(&[("LANG", "C"), ("LANGUAGE", "fr")]), Lang::En);
        assert_eq!(
            detect(&[("LC_ALL", "POSIX"), ("LANG", "fr_FR"), ("LANGUAGE", "fr")]),
            Lang::En
        );
    }

    #[test]
    fn empty_variables_are_skipped() {
        assert_eq!(detect(&[("LC_ALL", ""), ("LANG", "fr_FR.UTF-8")]), Lang::Fr);
    }

    #[test]
    fn unsupported_or_missing_environment_falls_back_to_english() {
        assert_eq!(detect(&[("LANG", "de_DE.UTF-8")]), Lang::FALLBACK);
        assert_eq!(detect(&[]), Lang::FALLBACK);
        assert_eq!(
            detect(&[("KFORMAT_LANG", "klingon"), ("LANG", "de_DE")]),
            Lang::FALLBACK
        );
    }

    #[test]
    fn size_formatting_conventions_differ_per_language() {
        assert_eq!(Lang::En.decimal_separator(), '.');
        assert_eq!(Lang::Fr.decimal_separator(), ',');
        assert_eq!(Lang::Fr.size_units()[2], "Go");
        assert_eq!(Lang::Es.size_units()[2], "GB");
    }

    #[test]
    fn codes_round_trip() {
        for lang in Lang::ALL {
            assert_eq!(Lang::from_locale(lang.code()), Some(lang));
        }
    }
}
