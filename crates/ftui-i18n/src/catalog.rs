//! String catalog with locale fallback and interpolation.
//!
//! # Invariants
//!
//! 1. **Fallback chain terminates**: every lookup walks the chain exactly
//!    once, returning `None` if no locale provides the key.
//!
//! 2. **Interpolation is idempotent**: `format()` replaces `{name}` tokens
//!    using a single pass; nested or recursive substitution does not occur.
//!
//! 3. **Thread safety**: `StringCatalog` is `Send + Sync` (all data is
//!    immutable after construction).
//!
//! # Failure Modes
//!
//! | Failure | Cause | Behavior |
//! |---------|-------|----------|
//! | Missing key | Key not in any locale | Returns `None` |
//! | Missing locale | Locale not loaded | Falls through chain |
//! | Bad interpolation arg | `{name}` but no `name` arg | Token left as-is |
//! | Empty catalog | No locales loaded | All lookups return `None` |

use std::collections::HashMap;

use crate::plural::{PluralCategory, PluralForms, PluralRule};

/// Locale identifier (e.g., `"en"`, `"en-US"`, `"ru"`).
pub type Locale = String;

/// Errors from i18n operations.
#[derive(Debug, Clone)]
pub enum I18nError {
    /// A locale string was malformed.
    InvalidLocale(String),
    /// A catalog file could not be parsed.
    ParseError(String),
    /// Duplicate key in the same locale.
    DuplicateKey { locale: String, key: String },
}

impl std::fmt::Display for I18nError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLocale(l) => write!(f, "invalid locale: {l}"),
            Self::ParseError(msg) => write!(f, "parse error: {msg}"),
            Self::DuplicateKey { locale, key } => {
                write!(f, "duplicate key '{key}' in locale '{locale}'")
            }
        }
    }
}

impl std::error::Error for I18nError {}

/// A single string entry: either a simple string or plural forms.
#[derive(Debug, Clone)]
pub enum StringEntry {
    /// A simple, non-pluralized string.
    Simple(String),
    /// Plural forms keyed by CLDR category.
    Plural(PluralForms),
}

/// Strings for a single locale.
#[derive(Debug, Clone, Default)]
pub struct LocaleStrings {
    strings: HashMap<String, StringEntry>,
}

impl LocaleStrings {
    /// Create an empty locale string set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a simple string.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.strings
            .insert(key.into(), StringEntry::Simple(value.into()));
    }

    /// Insert plural forms.
    pub fn insert_plural(&mut self, key: impl Into<String>, forms: PluralForms) {
        self.strings.insert(key.into(), StringEntry::Plural(forms));
    }

    /// Look up a string entry by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&StringEntry> {
        self.strings.get(key)
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Whether the locale has no strings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

/// Central string catalog with locale fallback and pluralization.
///
/// # Example
///
/// ```
/// use ftui_i18n::catalog::{StringCatalog, LocaleStrings};
/// use ftui_i18n::plural::PluralForms;
///
/// let mut catalog = StringCatalog::new();
///
/// let mut en = LocaleStrings::new();
/// en.insert("greeting", "Hello");
/// en.insert("welcome", "Welcome, {name}!");
/// en.insert_plural("items", PluralForms {
///     one: "{count} item".into(),
///     other: "{count} items".into(),
///     ..Default::default()
/// });
/// catalog.add_locale("en", en);
/// catalog.set_fallback_chain(vec!["en".into()]);
///
/// assert_eq!(catalog.get("en", "greeting"), Some("Hello"));
/// assert_eq!(
///     catalog.format("en", "welcome", &[("name", "Alice")]),
///     Some("Welcome, Alice!".into())
/// );
/// assert_eq!(
///     catalog.get_plural("en", "items", 1),
///     Some("{count} item")
/// );
/// assert_eq!(
///     catalog.get_plural("en", "items", 5),
///     Some("{count} items")
/// );
/// ```
#[derive(Debug, Clone)]
pub struct StringCatalog {
    locales: HashMap<Locale, LocaleStrings>,
    fallback_chain: Vec<Locale>,
    plural_rules: HashMap<Locale, PluralRule>,
}

impl Default for StringCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl StringCatalog {
    /// Create an empty catalog.
    #[must_use]
    pub fn new() -> Self {
        Self {
            locales: HashMap::new(),
            fallback_chain: Vec::new(),
            plural_rules: HashMap::new(),
        }
    }

    /// Add strings for a locale.
    ///
    /// Automatically detects the plural rule based on the locale tag.
    pub fn add_locale(&mut self, locale: impl Into<String>, strings: LocaleStrings) {
        let locale = locale.into();
        let rule = PluralRule::for_locale(&locale);
        self.plural_rules.insert(locale.clone(), rule);
        self.locales.insert(locale, strings);
    }

    /// Set the fallback chain (tried in order when a key is missing).
    ///
    /// Example: `["es-MX", "es", "en"]` — try Mexican Spanish, then
    /// generic Spanish, then English.
    pub fn set_fallback_chain(&mut self, chain: Vec<Locale>) {
        self.fallback_chain = chain;
    }

    /// Override the plural rule for a locale.
    pub fn set_plural_rule(&mut self, locale: impl Into<String>, rule: PluralRule) {
        self.plural_rules.insert(locale.into(), rule);
    }

    /// Look up a simple string by key.
    ///
    /// Tries the specified locale first, then walks the fallback chain.
    /// Returns `None` if no locale provides the key.
    #[must_use]
    pub fn get(&self, locale: &str, key: &str) -> Option<&str> {
        // Try the specified locale
        if let Some(entry) = self.locales.get(locale).and_then(|ls| ls.get(key)) {
            return match entry {
                StringEntry::Simple(s) => Some(s.as_str()),
                StringEntry::Plural(p) => Some(&p.other),
            };
        }

        // Walk fallback chain
        for fallback in &self.fallback_chain {
            if fallback == locale {
                continue; // Already tried
            }
            if let Some(entry) = self
                .locales
                .get(fallback.as_str())
                .and_then(|ls| ls.get(key))
            {
                return match entry {
                    StringEntry::Simple(s) => Some(s.as_str()),
                    StringEntry::Plural(p) => Some(&p.other),
                };
            }
        }

        None
    }

    /// Look up a pluralized string by key and count.
    ///
    /// Uses the locale's plural rule to select the appropriate form.
    #[must_use]
    pub fn get_plural(&self, locale: &str, key: &str, count: i64) -> Option<&str> {
        let rule = self
            .plural_rules
            .get(locale)
            .cloned()
            .unwrap_or(PluralRule::English);
        let category = rule.categorize(count);

        // Try specified locale
        if let Some(result) = self.get_plural_from(locale, key, category) {
            return Some(result);
        }

        // Walk fallback chain
        for fallback in &self.fallback_chain {
            if fallback == locale {
                continue;
            }
            let fb_rule = self
                .plural_rules
                .get(fallback.as_str())
                .cloned()
                .unwrap_or(PluralRule::English);
            let fb_category = fb_rule.categorize(count);
            if let Some(result) = self.get_plural_from(fallback, key, fb_category) {
                return Some(result);
            }
        }

        None
    }

    fn get_plural_from(&self, locale: &str, key: &str, category: PluralCategory) -> Option<&str> {
        self.locales
            .get(locale)
            .and_then(|ls| ls.get(key))
            .map(|entry| match entry {
                StringEntry::Plural(forms) => forms.select(category),
                StringEntry::Simple(s) => s.as_str(),
            })
    }

    /// Look up a string and perform `{key}` interpolation.
    ///
    /// Each `(name, value)` pair in `args` replaces `{name}` in the
    /// template string. Tokens without matching args are left as-is.
    #[must_use]
    pub fn format(&self, locale: &str, key: &str, args: &[(&str, &str)]) -> Option<String> {
        self.get(locale, key)
            .map(|template| interpolate(template, args))
    }

    /// Look up a pluralized string and perform interpolation.
    ///
    /// Automatically adds a `{count}` argument.
    #[must_use]
    pub fn format_plural(
        &self,
        locale: &str,
        key: &str,
        count: i64,
        extra_args: &[(&str, &str)],
    ) -> Option<String> {
        self.get_plural(locale, key, count).map(|template| {
            let count_str = count.to_string();
            let mut all_args: Vec<(&str, &str)> = vec![("count", &count_str)];
            all_args.extend_from_slice(extra_args);
            interpolate(template, &all_args)
        })
    }

    /// All registered locale tags.
    #[must_use]
    pub fn locales(&self) -> Vec<&str> {
        self.locales.keys().map(String::as_str).collect()
    }
}

/// Single-pass `{name}` interpolation. Unmatched tokens left as-is.
fn interpolate(template: &str, args: &[(&str, &str)]) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            // Try to read a token name until '}'
            let mut token = String::new();
            let mut found_close = false;
            for c in chars.by_ref() {
                if c == '}' {
                    found_close = true;
                    break;
                }
                token.push(c);
            }

            if found_close {
                // Look up the token in args
                if let Some(&(_, value)) = args.iter().find(|&&(name, _)| name == token) {
                    result.push_str(value);
                } else {
                    // No match: leave token as-is
                    result.push('{');
                    result.push_str(&token);
                    result.push('}');
                }
            } else {
                // Unclosed brace: emit as-is
                result.push('{');
                result.push_str(&token);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plural::PluralForms;

    fn english_catalog() -> StringCatalog {
        let mut catalog = StringCatalog::new();
        let mut en = LocaleStrings::new();
        en.insert("greeting", "Hello");
        en.insert("welcome", "Welcome, {name}!");
        en.insert("farewell", "Goodbye, {name}. See you {when}.");
        en.insert_plural(
            "items",
            PluralForms {
                one: "{count} item".into(),
                other: "{count} items".into(),
                ..Default::default()
            },
        );
        catalog.add_locale("en", en);
        catalog.set_fallback_chain(vec!["en".into()]);
        catalog
    }

    #[test]
    fn simple_lookup() {
        let catalog = english_catalog();
        assert_eq!(catalog.get("en", "greeting"), Some("Hello"));
    }

    #[test]
    fn missing_key_returns_none() {
        let catalog = english_catalog();
        assert_eq!(catalog.get("en", "nonexistent"), None);
    }

    #[test]
    fn missing_locale_falls_back() {
        let catalog = english_catalog();
        // "fr" not in catalog, falls back to "en"
        assert_eq!(catalog.get("fr", "greeting"), Some("Hello"));
    }

    #[test]
    fn fallback_chain_order() {
        let mut catalog = StringCatalog::new();

        let mut en = LocaleStrings::new();
        en.insert("greeting", "Hello");
        en.insert("color", "Color");

        let mut es = LocaleStrings::new();
        es.insert("greeting", "Hola");
        // "color" not in es

        let mut es_mx = LocaleStrings::new();
        es_mx.insert("greeting", "Qué onda");
        // "color" not in es_mx

        catalog.add_locale("en", en);
        catalog.add_locale("es", es);
        catalog.add_locale("es-MX", es_mx);
        catalog.set_fallback_chain(vec!["es-MX".into(), "es".into(), "en".into()]);

        // Direct hit
        assert_eq!(catalog.get("es-MX", "greeting"), Some("Qué onda"));
        // Falls through es-MX (no color) -> es (no color) -> en
        assert_eq!(catalog.get("es-MX", "color"), Some("Color"));
    }

    #[test]
    fn plural_english_singular() {
        let catalog = english_catalog();
        assert_eq!(catalog.get_plural("en", "items", 1), Some("{count} item"));
    }

    #[test]
    fn plural_english_plural() {
        let catalog = english_catalog();
        assert_eq!(catalog.get_plural("en", "items", 0), Some("{count} items"));
        assert_eq!(catalog.get_plural("en", "items", 2), Some("{count} items"));
        assert_eq!(
            catalog.get_plural("en", "items", 100),
            Some("{count} items")
        );
    }

    #[test]
    fn plural_russian() {
        let mut catalog = StringCatalog::new();
        let mut ru = LocaleStrings::new();
        ru.insert_plural(
            "files",
            PluralForms {
                one: "{count} файл".into(),
                few: Some("{count} файла".into()),
                many: Some("{count} файлов".into()),
                other: "{count} файлов".into(),
                ..Default::default()
            },
        );
        catalog.add_locale("ru", ru);

        assert_eq!(catalog.get_plural("ru", "files", 1), Some("{count} файл"));
        assert_eq!(catalog.get_plural("ru", "files", 3), Some("{count} файла"));
        assert_eq!(catalog.get_plural("ru", "files", 5), Some("{count} файлов"));
        assert_eq!(catalog.get_plural("ru", "files", 21), Some("{count} файл"));
    }

    #[test]
    fn interpolation_single_arg() {
        let catalog = english_catalog();
        assert_eq!(
            catalog.format("en", "welcome", &[("name", "Alice")]),
            Some("Welcome, Alice!".into())
        );
    }

    #[test]
    fn interpolation_multiple_args() {
        let catalog = english_catalog();
        assert_eq!(
            catalog.format("en", "farewell", &[("name", "Bob"), ("when", "tomorrow")]),
            Some("Goodbye, Bob. See you tomorrow.".into())
        );
    }

    #[test]
    fn interpolation_missing_arg_left_as_is() {
        let catalog = english_catalog();
        assert_eq!(
            catalog.format("en", "welcome", &[]),
            Some("Welcome, {name}!".into())
        );
    }

    #[test]
    fn format_plural_auto_count() {
        let catalog = english_catalog();
        assert_eq!(
            catalog.format_plural("en", "items", 1, &[]),
            Some("1 item".into())
        );
        assert_eq!(
            catalog.format_plural("en", "items", 42, &[]),
            Some("42 items".into())
        );
    }

    #[test]
    fn interpolation_edge_cases() {
        // Unclosed brace
        assert_eq!(interpolate("Hello {world", &[]), "Hello {world");
        // Empty braces
        assert_eq!(interpolate("Hello {}", &[]), "Hello {}");
        // No braces
        assert_eq!(interpolate("Hello World", &[]), "Hello World");
        // Multiple occurrences
        assert_eq!(interpolate("{x} and {x}", &[("x", "A")]), "A and A");
    }

    #[test]
    fn empty_catalog() {
        let catalog = StringCatalog::new();
        assert_eq!(catalog.get("en", "anything"), None);
        assert_eq!(catalog.get_plural("en", "anything", 1), None);
        assert!(catalog.locales().is_empty());
    }

    #[test]
    fn locale_listing() {
        let catalog = english_catalog();
        let locales = catalog.locales();
        assert_eq!(locales.len(), 1);
        assert!(locales.contains(&"en"));
    }

    #[test]
    fn locale_strings_len() {
        let catalog = english_catalog();
        let en = catalog.locales.get("en").unwrap();
        assert_eq!(en.len(), 4); // greeting, welcome, farewell, items
        assert!(!en.is_empty());
    }

    #[test]
    fn simple_entry_from_plural_lookup() {
        // Looking up a Simple entry via get_plural should still work
        let catalog = english_catalog();
        assert_eq!(catalog.get_plural("en", "greeting", 1), Some("Hello"));
    }
}
