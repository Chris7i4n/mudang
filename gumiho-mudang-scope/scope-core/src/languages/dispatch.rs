//! Compile-time language dispatch.
//!
//! Per R7 (α.4): `register_languages!` is the only registration point. The
//! dispatch table is built from `LanguageId::extensions()` /
//! `LanguageId::shebangs()` const-fn data, so adding a variant flows into
//! both lookup functions without any extra step. `assert_no_extension_overlap`
//! panics **at compile time** if two languages claim the same extension —
//! the indexer can never see an ambiguous file.

use crate::languages::id::LanguageId;

/// Byte-wise string equality, callable from `const` context.
const fn str_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Generates `dispatch_extension` and `dispatch_shebang` over the listed
/// `LanguageId` variants. Single source of truth for the variant set.
macro_rules! register_languages {
    ( $( $variant:ident ),* $(,)? ) => {
        /// Compile-time list of every registered language. Drives the
        /// const-panic overlap check below.
        pub const REGISTERED: &[LanguageId] = &[
            $( LanguageId::$variant, )*
        ];

        /// Resolve a file extension (no leading dot) to its language.
        ///
        /// `const fn` — usable in static initializers; the dispatch table
        /// is computed at compile time. `O(total extensions)` per call,
        /// but the count is fixed and tiny.
        pub const fn dispatch_extension(ext: &str) -> Option<LanguageId> {
            $(
                {
                    let exts = LanguageId::$variant.extensions();
                    let mut i = 0;
                    while i < exts.len() {
                        if str_eq(exts[i], ext) {
                            return Some(LanguageId::$variant);
                        }
                        i += 1;
                    }
                }
            )*
            None
        }

        /// Resolve a shebang interpreter token to its language.
        ///
        /// Currently unused (extension dispatch is sufficient for every
        /// indexed file). Reserved for the cheap-path extension queued in
        /// BACKLOG.md.
        pub const fn dispatch_shebang(token: &str) -> Option<LanguageId> {
            $(
                {
                    let tokens = LanguageId::$variant.shebangs();
                    let mut i = 0;
                    while i < tokens.len() {
                        if str_eq(tokens[i], token) {
                            return Some(LanguageId::$variant);
                        }
                        i += 1;
                    }
                }
            )*
            None
        }
    };
}

register_languages!(TypeScript, CSharp, Python, Go, Java, Rust, Ruby);

/// Compile-time invariant: no two registered languages claim the same
/// extension. Triggers a const-panic with a clear message if violated.
const _ASSERT_NO_EXTENSION_OVERLAP: () = {
    let mut i = 0;
    while i < REGISTERED.len() {
        let mut j = i + 1;
        while j < REGISTERED.len() {
            let a = REGISTERED[i].extensions();
            let b = REGISTERED[j].extensions();
            let mut ai = 0;
            while ai < a.len() {
                let mut bi = 0;
                while bi < b.len() {
                    if str_eq(a[ai], b[bi]) {
                        panic!("R7 invariant violated: two LanguageId variants claim the same file extension");
                    }
                    bi += 1;
                }
                ai += 1;
            }
            j += 1;
        }
        i += 1;
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_dispatch_every_known_extension() {
        assert_eq!(dispatch_extension("ts"), Some(LanguageId::TypeScript));
        assert_eq!(dispatch_extension("tsx"), Some(LanguageId::TypeScript));
        assert_eq!(dispatch_extension("cs"), Some(LanguageId::CSharp));
        assert_eq!(dispatch_extension("py"), Some(LanguageId::Python));
        assert_eq!(dispatch_extension("go"), Some(LanguageId::Go));
        assert_eq!(dispatch_extension("java"), Some(LanguageId::Java));
        assert_eq!(dispatch_extension("rs"), Some(LanguageId::Rust));
        assert_eq!(dispatch_extension("rb"), Some(LanguageId::Ruby));
    }

    #[test]
    fn unknown_extension_returns_none() {
        assert_eq!(dispatch_extension("md"), None);
        assert_eq!(dispatch_extension(""), None);
        assert_eq!(dispatch_extension("rs2"), None);
    }

    #[test]
    fn shebangs_dispatch_known_tokens() {
        assert_eq!(dispatch_shebang("python"), Some(LanguageId::Python));
        assert_eq!(dispatch_shebang("python3"), Some(LanguageId::Python));
        assert_eq!(dispatch_shebang("ruby"), Some(LanguageId::Ruby));
        assert_eq!(dispatch_shebang("bash"), None);
    }

    #[test]
    fn registered_count_matches_variants() {
        assert_eq!(REGISTERED.len(), 7);
    }
}
