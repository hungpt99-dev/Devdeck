//! Fluent message bundles + ICU4X locale handling. Typed keys are generated
//! from the English source at build time; runtime switching needs no restart.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use fluent::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use icu::locale::Locale;
use unic_langid::LanguageIdentifier;

include!(concat!(env!("OUT_DIR"), "/keys.rs"));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocaleMode {
    System,
    English,
    Vietnamese,
}

pub struct Translator {
    bundles: BTreeMap<String, FluentBundle<FluentResource>>,
    locale: String,
}

impl Translator {
    /// Load en and vi from `locales/`. Missing locale files are a hard error:
    /// a release build must never show raw message keys.
    pub fn new(locales_dir: &Path) -> Result<Self, I18nError> {
        let mut bundles = BTreeMap::new();
        for locale in ["en", "vi"] {
            let dir = locales_dir.join(locale);
            if !dir.is_dir() {
                return Err(I18nError::MissingLocale(locale.into()));
            }
            let mut bundle = FluentBundle::new(vec![langid(locale)?]);
            // Plain text UI labels; bidi isolates would add invisible marks
            // to ordinary strings such as "1 tool".
            bundle.set_use_isolating(false);
            let mut entries: Vec<PathBuf> = fs::read_dir(&dir)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("ftl"))
                .collect();
            entries.sort();
            for path in entries {
                let source = fs::read_to_string(path)?;
                let resource = FluentResource::try_new(source)
                    .map_err(|(_, errors)| I18nError::Fluent(format!("{errors:?}")))?;
                bundle
                    .add_resource(resource)
                    .map_err(|e| I18nError::Fluent(format!("{e:?}")))?;
            }
            bundles.insert(locale.to_string(), bundle);
        }
        let mut translator = Self {
            bundles,
            locale: "en".into(),
        };
        let system = sys_locale::get_locale().unwrap_or_else(|| "en".into());
        translator.set_locale(&system);
        Ok(translator)
    }

    pub fn with_dir() -> Result<Self, I18nError> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../locales");
        Self::new(&path)
    }

    /// Resolve the configured locale through the documented fallback chain
    /// (e.g. vi-VN -> vi -> en). Returns false when no bundle matched.
    pub fn set_locale(&mut self, requested: &str) -> bool {
        for candidate in fallback_chain(requested) {
            if self.bundles.contains_key(&candidate) {
                self.locale = candidate;
                return true;
            }
        }
        false
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn available_locales(&self) -> Vec<&str> {
        self.bundles.keys().map(String::as_str).collect()
    }

    /// Typed entry point: an unknown key cannot compile.
    pub fn t(&self, key: MessageKey) -> String {
        self.t_args(key.as_str(), &[])
    }

    pub fn t_args<'a>(&'a self, key: &str, args: &[(&str, &'a str)]) -> String {
        if let Some(value) = self.format_in(&self.locale, key, args) {
            return value;
        }
        // Never surface a raw key in a release build: fall back to English.
        if self.locale != "en" {
            if let Some(value) = self.format_in("en", key, args) {
                return value;
            }
        }
        format!("[{key}]")
    }

    /// Format a message with a numeric `count` variable so Fluent plural
    /// rules work (string variables cannot be pluralized).
    pub fn t_count(&self, key: MessageKey, count: usize) -> String {
        if let Some(value) = self.format_count_in(&self.locale, key.as_str(), count) {
            return value;
        }
        if self.locale != "en" {
            if let Some(value) = self.format_count_in("en", key.as_str(), count) {
                return value;
            }
        }
        format!("[{}]", key.as_str())
    }

    fn format_in(&self, locale: &str, key: &str, args: &[(&str, &str)]) -> Option<String> {
        let bundle = self.bundles.get(locale)?;
        let message = bundle.get_message(key)?;
        let pattern = message.value()?;
        let mut fluent_args = FluentArgs::new();
        for (name, value) in args {
            fluent_args.set(*name, FluentValue::from(*value));
        }
        let mut errors = Vec::new();
        let formatted = bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);
        if errors.is_empty() {
            Some(formatted.into_owned())
        } else {
            None
        }
    }

    fn format_count_in(&self, locale: &str, key: &str, count: usize) -> Option<String> {
        let bundle = self.bundles.get(locale)?;
        let pattern = bundle.get_message(key)?.value()?;
        let mut args = FluentArgs::new();
        args.set("count", FluentValue::from(count as f64));
        let mut errors = Vec::new();
        let formatted = bundle.format_pattern(pattern, Some(&args), &mut errors);
        if errors.is_empty() {
            Some(formatted.into_owned())
        } else {
            None
        }
    }
}

fn langid(locale: &str) -> Result<LanguageIdentifier, I18nError> {
    locale
        .parse::<LanguageIdentifier>()
        .map_err(|e| I18nError::Locale(format!("{e:?}")))
}

/// Use ICU4X for canonicalization, then apply the product fallback chain.
pub fn fallback_chain(requested: &str) -> Vec<String> {
    let canonical = requested
        .parse::<Locale>()
        .map(|l| l.id.language.to_string())
        .unwrap_or_else(|_| {
            requested
                .split(['-', '_'])
                .next()
                .unwrap_or("en")
                .to_string()
        });
    let mut chain = vec![requested.to_string(), canonical.clone()];
    if canonical != "en" {
        chain.push("en".into());
    }
    chain
}

#[derive(Debug, thiserror::Error)]
pub enum I18nError {
    #[error("missing locale directory: {0}")]
    MissingLocale(String),
    #[error("locale parse error: {0}")]
    Locale(String),
    #[error("Fluent resource error: {0}")]
    Fluent(String),
    #[error("locale I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translator() -> Translator {
        Translator::new(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../locales")).unwrap()
    }

    #[test]
    fn typed_keys_cover_both_locales() {
        let t = translator();
        assert!(ALL_MESSAGE_KEYS.len() > 10);
        assert!(ALL_MESSAGE_KEYS.contains(&"app-title"));
        assert_eq!(t.t_args(MessageKey::AppTitle.as_str(), &[]), "Devdeck");
    }

    #[test]
    fn runtime_switch_translates_message() {
        let mut t = translator();
        t.set_locale("en");
        assert_eq!(t.t(MessageKey::NavEditor), "Editor");
        assert!(t.set_locale("vi"));
        assert_eq!(t.locale(), "vi");
        assert_eq!(t.t(MessageKey::NavEditor), "Trình soạn thảo");
        assert!(t.set_locale("en"));
        assert_eq!(t.t(MessageKey::NavEditor), "Editor");
    }

    #[test]
    fn fallback_chain_resolves_region() {
        assert_eq!(fallback_chain("vi-VN"), vec!["vi-VN", "vi", "en"]);
        assert_eq!(fallback_chain("en-US"), vec!["en-US", "en"]);
        assert_eq!(fallback_chain("fr-FR"), vec!["fr-FR", "fr", "en"]);
    }

    #[test]
    fn plural_and_variables_work() {
        let mut t = translator();
        t.set_locale("vi");
        let one = t.t_args(MessageKey::StatusTools.as_str(), &[("count", "1")]);
        assert_eq!(one, "Đã tải 1 công cụ");
        let many = t.t_args(MessageKey::StatusTools.as_str(), &[("count", "12")]);
        assert_eq!(many, "Đã tải 12 công cụ");
    }

    #[test]
    fn numeric_count_selects_correct_plural() {
        let mut t = translator();
        t.set_locale("en");
        assert_eq!(t.t_count(MessageKey::StatusTools, 1), "1 tool loaded");
        assert_eq!(t.t_count(MessageKey::StatusTools, 12), "12 tools loaded");
    }

    #[test]
    fn missing_key_never_leaks_raw_key_in_release() {
        let t = translator();
        assert_eq!(t.t_args("missing.key", &[]), "[missing.key]");
    }
}
