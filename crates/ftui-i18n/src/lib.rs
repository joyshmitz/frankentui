#![forbid(unsafe_code)]

//! Internationalization (i18n) foundation for FrankenTUI.
//!
//! Provides externalized string storage with key-based lookup,
//! locale fallback chains, ICU-style plural forms, and variable
//! interpolation.

pub mod catalog;
pub mod plural;

pub use catalog::{I18nError, LocaleStrings, StringCatalog, StringEntry};
pub use plural::{PluralCategory, PluralForms, PluralRule};
