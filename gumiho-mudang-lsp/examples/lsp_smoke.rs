//! Smoke test: connects to a real LSP server (default: rust-analyzer),
//! performs a handshake, opens a file, waits for diagnostics, and shuts down.
//!
//! Usage:
//!   cargo run --example lsp_smoke
//!
//! Requires rust-analyzer on PATH.

use std::sync::Arc;
use std::time::Duration;

use gumiho_lsp::client::LspClient;

#[tokio::main]
async fn main() {
    let cwd = std::env::current_dir().unwrap();
    println!("=== LSP Smoke Test ===");
    println!("cwd: {}", cwd.display());

    let client = Arc::new(LspClient::new("rust-analyzer", &[], &cwd));

    // Register notification handler for diagnostics
    client.on_notification(|notif| {
        if notif.method == "textDocument/publishDiagnostics" {
            if let Some(params) = &notif.params {
                let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("?");
                let count = params
                    .get("diagnostics")
                    .and_then(|v| v.as_array())
                    .map_or(0, |a| a.len());
                println!("[diag] {uri}: {count} diagnostic(s)");
            }
        }
    });

    println!("\n1. Starting rust-analyzer...");
    if let Err(e) = client.start().await {
        eprintln!("Failed to start: {e}");
        eprintln!("Is rust-analyzer installed? Run: rustup component add rust-analyzer");
        std::process::exit(1);
    }
    println!("   Started OK");

    println!("\n2. Sending initialize...");
    let root_uri = format!("file://{}", cwd.display());
    match client.initialize(&root_uri).await {
        Ok(result) => {
            let name = result
                .get("serverInfo")
                .and_then(|s| s.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");
            let version = result
                .get("serverInfo")
                .and_then(|s| s.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            println!("   Server: {name} v{version}");
        }
        Err(e) => {
            eprintln!("Initialize failed: {e}");
            client.kill().await;
            std::process::exit(1);
        }
    }

    // Open a file
    let test_file = cwd.join("src/lib.rs");
    if test_file.exists() {
        let content = std::fs::read_to_string(&test_file).unwrap();
        let file_uri = format!("file://{}", test_file.display());
        println!("\n3. Opening {}...", test_file.display());
        let _ = client
            .notify(
                "textDocument/didOpen",
                serde_json::json!({
                    "textDocument": {
                        "uri": file_uri,
                        "languageId": "rust",
                        "version": 1,
                        "text": content
                    }
                }),
            )
            .await;
        println!("   Waiting 3s for diagnostics...");
        tokio::time::sleep(Duration::from_secs(3)).await;
    } else {
        println!("\n3. Skipping file open (lib.rs not found at expected path)");
    }

    // Try a hover request
    if test_file.exists() {
        let file_uri = format!("file://{}", test_file.display());
        println!("\n4. Sending hover request (line 0, char 4)...");
        match client
            .request(
                "textDocument/hover",
                serde_json::json!({
                    "textDocument": { "uri": file_uri },
                    "position": { "line": 0, "character": 4 }
                }),
            )
            .await
        {
            Ok(result) => {
                if result.is_null() {
                    println!("   No hover info at this position");
                } else {
                    let preview = result.to_string();
                    let truncated = if preview.len() > 200 {
                        format!("{}...", &preview[..200])
                    } else {
                        preview
                    };
                    println!("   Hover: {truncated}");
                }
            }
            Err(e) => println!("   Hover failed: {e}"),
        }
    }

    println!("\n5. Shutting down...");
    let _ = client.shutdown_and_exit().await;
    println!("   Done!");
    println!("\n=== Smoke test passed ===");
}
