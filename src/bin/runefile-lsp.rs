//! Runefile Language Server Binary
//!
//! This binary provides a Language Server Protocol implementation for Runefile.

use rune::lsp::RunefileLanguageServer;
use std::io::{self, BufRead, BufReader, Write};
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .with_writer(io::stderr)
        .init();

    info!("Starting Runefile Language Server");

    let mut server = RunefileLanguageServer::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = BufReader::new(stdin.lock());

    // Process LSP messages from stdin
    let mut content_length: Option<usize> = None;
    let mut in_headers = true;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                error!("Error reading line: {}", e);
                continue;
            }
        };

        if in_headers {
            if line.is_empty() {
                in_headers = false;
                // Read content based on content-length
                if let Some(_len) = content_length {
                    // In a real implementation, we'd read the JSON-RPC message
                    // and dispatch to the appropriate handler
                }
            } else if line.to_lowercase().starts_with("content-length:") {
                if let Some(len_str) = line.split(':').nth(1) {
                    content_length = len_str.trim().parse().ok();
                }
            }
        } else {
            // Process JSON-RPC message
            debug!("Received message: {}", line);

            // Parse and handle the message
            // This is a simplified implementation
            if line.contains("\"method\":\"initialize\"") {
                let response = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{"textDocumentSync":1,"completionProvider":{"triggerCharacters":[" "]},"hoverProvider":true}}}"#;
                send_response(&mut stdout, response);
            } else if line.contains("\"method\":\"shutdown\"") {
                let response = r#"{"jsonrpc":"2.0","id":2,"result":null}"#;
                send_response(&mut stdout, response);
            }

            // Reset for next message
            in_headers = true;
            content_length = None;
        }
    }
}

fn send_response(stdout: &mut io::Stdout, content: &str) {
    let response = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);
    let _ = stdout.write_all(response.as_bytes());
    let _ = stdout.flush();
}
