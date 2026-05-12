//! R7 (B.3) regression — `LanguageId::as_str()` returns the historical DB
//! slug verbatim for every variant.
//!
//! `symbols.language` rows persist these slugs. Changing any arm here is a
//! schema break that requires a coordinated re-index of every existing
//! database. The test pins the contract.

use scope_core::languages::dispatch::REGISTERED;
use scope_core::languages::LanguageId;

#[test]
fn db_slugs_match_historical_values() {
    assert_eq!(LanguageId::TypeScript.as_str(), "typescript");
    assert_eq!(LanguageId::CSharp.as_str(), "csharp");
    assert_eq!(LanguageId::Python.as_str(), "python");
    assert_eq!(LanguageId::Go.as_str(), "go");
    assert_eq!(LanguageId::Java.as_str(), "java");
    assert_eq!(LanguageId::Rust.as_str(), "rust");
    assert_eq!(LanguageId::Ruby.as_str(), "ruby");
}

#[test]
fn from_slug_round_trips_every_variant() {
    for &lang in REGISTERED {
        let slug = lang.as_str();
        assert_eq!(
            LanguageId::from_slug(slug),
            Some(lang),
            "round-trip failed for {lang}"
        );
    }
}

#[test]
fn from_slug_rejects_unknown() {
    assert_eq!(LanguageId::from_slug(""), None);
    assert_eq!(LanguageId::from_slug("TypeScript"), None);
    assert_eq!(LanguageId::from_slug("kotlin"), None);
}
