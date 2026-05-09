use crate::types::LspCapabilities;

#[derive(Debug, Clone)]
pub struct LspServerDef {
    pub id: &'static str,
    pub name: &'static str,
    pub command: &'static str,
    pub args: &'static [&'static str],
    pub languages: &'static [&'static str],
    pub extensions: &'static [&'static str],
    pub root_markers: &'static [&'static str],
    pub skip_if: &'static [&'static str],
    pub requires: &'static [&'static str],
    pub capabilities: LspCapabilities,
    pub install_hint: &'static str,
}

pub static KNOWN_SERVERS: &[LspServerDef] = &[
    // Dart / Flutter
    LspServerDef {
        id: "dart",
        name: "Dart / Flutter",
        command: "dart",
        args: &["language-server", "--protocol=lsp"],
        languages: &["Dart"],
        extensions: &[".dart"],
        root_markers: &["pubspec.yaml", "analysis_options.yaml"],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::FULL,
        install_hint: "https://dart.dev/get-dart",
    },
    // TypeScript / JavaScript
    LspServerDef {
        id: "typescript",
        name: "TypeScript / JavaScript",
        command: "typescript-language-server",
        args: &["--stdio"],
        languages: &["TypeScript", "JavaScript"],
        extensions: &[".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts"],
        root_markers: &["package.json", "tsconfig.json", "jsconfig.json"],
        skip_if: &["deno.json", "deno.jsonc"],
        requires: &["tsc"],
        capabilities: LspCapabilities::FULL,
        install_hint: "npm install -g typescript-language-server typescript",
    },
    // Vue
    LspServerDef {
        id: "vue",
        name: "Vue",
        command: "vue-language-server",
        args: &["--stdio"],
        languages: &["Vue"],
        extensions: &[".vue"],
        root_markers: &["package.json", "vite.config.ts", "vite.config.js"],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::NO_CALL_HIERARCHY,
        install_hint: "npm install -g @vue/language-server",
    },
    // Svelte
    LspServerDef {
        id: "svelte",
        name: "Svelte",
        command: "svelteserver",
        args: &["--stdio"],
        languages: &["Svelte"],
        extensions: &[".svelte"],
        root_markers: &["package.json", "svelte.config.js", "svelte.config.ts"],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::BASIC,
        install_hint: "npm install -g svelte-language-server",
    },
    // Python (pyright)
    LspServerDef {
        id: "pyright",
        name: "Python (pyright)",
        command: "pyright-langserver",
        args: &["--stdio"],
        languages: &["Python"],
        extensions: &[".py", ".pyi"],
        root_markers: &[
            "pyproject.toml",
            "setup.py",
            "requirements.txt",
            "pyrightconfig.json",
        ],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::FULL,
        install_hint: "npm install -g pyright",
    },
    // Python (pylsp)
    LspServerDef {
        id: "pylsp",
        name: "Python (pylsp)",
        command: "pylsp",
        args: &[],
        languages: &["Python"],
        extensions: &[".py", ".pyi"],
        root_markers: &["pyproject.toml", "setup.py", "requirements.txt"],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::NO_CALL_HIERARCHY,
        install_hint: "pip install python-lsp-server",
    },
    // Go
    LspServerDef {
        id: "gopls",
        name: "Go",
        command: "gopls",
        args: &[],
        languages: &["Go"],
        extensions: &[".go"],
        root_markers: &["go.work", "go.mod"],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::FULL,
        install_hint: "go install golang.org/x/tools/gopls@latest",
    },
    // Rust
    LspServerDef {
        id: "rust-analyzer",
        name: "Rust",
        command: "rust-analyzer",
        args: &[],
        languages: &["Rust"],
        extensions: &[".rs"],
        root_markers: &["Cargo.toml"],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::FULL,
        install_hint: "rustup component add rust-analyzer",
    },
    // Kotlin (kotlin-lsp)
    LspServerDef {
        id: "kotlin-lsp",
        name: "Kotlin (kotlin-lsp)",
        command: "kotlin-lsp",
        args: &["--stdio"],
        languages: &["Kotlin"],
        extensions: &[".kt", ".kts"],
        root_markers: &[
            "settings.gradle.kts",
            "settings.gradle",
            "build.gradle.kts",
            "build.gradle",
            "gradlew",
            "pom.xml",
        ],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::FULL,
        install_hint: "https://github.com/Kotlin/kotlin-lsp",
    },
    // Kotlin (kotlin-language-server)
    LspServerDef {
        id: "kotlin-language-server",
        name: "Kotlin (kotlin-language-server)",
        command: "kotlin-language-server",
        args: &[],
        languages: &["Kotlin"],
        extensions: &[".kt", ".kts"],
        root_markers: &[
            "settings.gradle.kts",
            "settings.gradle",
            "build.gradle.kts",
            "build.gradle",
        ],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::NO_CALL_HIERARCHY,
        install_hint: "https://github.com/fwcd/kotlin-language-server",
    },
    // Swift
    LspServerDef {
        id: "sourcekit-lsp",
        name: "Swift",
        command: "sourcekit-lsp",
        args: &[],
        languages: &["Swift"],
        extensions: &[".swift"],
        root_markers: &["Package.swift"],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::NO_CALL_HIERARCHY,
        install_hint: "Ships with Xcode / Swift toolchain",
    },
    // Ruby
    LspServerDef {
        id: "ruby-lsp",
        name: "Ruby",
        command: "ruby-lsp",
        args: &[],
        languages: &["Ruby"],
        extensions: &[".rb", ".rake", ".gemspec"],
        root_markers: &["Gemfile", ".ruby-version", "Rakefile"],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::FULL,
        install_hint: "gem install ruby-lsp",
    },
    // C / C++
    LspServerDef {
        id: "clangd",
        name: "C / C++",
        command: "clangd",
        args: &[],
        languages: &["C", "C++"],
        extensions: &[".c", ".cpp", ".cc", ".cxx", ".h", ".hpp"],
        root_markers: &["compile_commands.json", "CMakeLists.txt", ".clangd"],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::FULL,
        install_hint: "brew install llvm  # or: apt install clangd",
    },
    // Lua
    LspServerDef {
        id: "lua-language-server",
        name: "Lua",
        command: "lua-language-server",
        args: &[],
        languages: &["Lua"],
        extensions: &[".lua"],
        root_markers: &[".luarc.json", ".luarc.jsonc"],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::NO_CALL_HIERARCHY,
        install_hint: "brew install lua-language-server",
    },
    // Bash / Shell
    LspServerDef {
        id: "bash-language-server",
        name: "Bash / Shell",
        command: "bash-language-server",
        args: &["start"],
        languages: &["Bash", "Shell"],
        extensions: &[".sh", ".bash"],
        root_markers: &[],
        skip_if: &[],
        requires: &[],
        capabilities: LspCapabilities::BASIC,
        install_hint: "npm install -g bash-language-server",
    },
];

pub fn find_server_by_id(id: &str) -> Option<&'static LspServerDef> {
    KNOWN_SERVERS.iter().find(|s| s.id == id)
}

pub fn find_servers_for_extension(ext: &str) -> Vec<&'static LspServerDef> {
    KNOWN_SERVERS
        .iter()
        .filter(|s| s.extensions.contains(&ext))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_servers_count() {
        assert_eq!(KNOWN_SERVERS.len(), 15);
    }

    #[test]
    fn test_find_by_id() {
        let server = find_server_by_id("rust-analyzer").unwrap();
        assert_eq!(server.command, "rust-analyzer");
        assert_eq!(server.extensions, &[".rs"]);
    }

    #[test]
    fn test_find_by_extension() {
        let servers = find_servers_for_extension(".ts");
        assert!(!servers.is_empty());
        assert!(servers.iter().any(|s| s.id == "typescript"));
    }

    #[test]
    fn test_capabilities_preset() {
        let rust = find_server_by_id("rust-analyzer").unwrap();
        assert_eq!(rust.capabilities, LspCapabilities::FULL);

        let svelte = find_server_by_id("svelte").unwrap();
        assert_eq!(svelte.capabilities, LspCapabilities::BASIC);

        let vue = find_server_by_id("vue").unwrap();
        assert_eq!(vue.capabilities, LspCapabilities::NO_CALL_HIERARCHY);
    }
}
