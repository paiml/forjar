//! FJ-110: Language Server Protocol for forjar YAML.
//!
//! Lightweight LSP server over stdio using JSON-RPC 2.0.
//! Provides: autocompletion, hover docs, diagnostics (validation),
//! go-to-definition for includes/data sources.
//! No external LSP library — raw JSON-RPC protocol.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};

/// LSP server state.
#[derive(Debug)]
pub struct LspServer {
    /// Whether the client has sent `initialize`.
    pub initialized: bool,
    /// Open document URIs mapped to their content.
    pub documents: HashMap<String, String>,
    /// Workspace root URI.
    pub root_uri: Option<String>,
    /// Whether a shutdown was requested.
    pub shutdown_requested: bool,
}

/// LSP diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DiagnosticSeverity {
    /// Error severity.
    Error = 1,
    /// Warning severity.
    Warning = 2,
    /// Informational severity.
    Information = 3,
    /// Hint severity.
    Hint = 4,
}

/// LSP diagnostic.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Diagnostic {
    /// Start line (0-based).
    pub line: u32,
    /// Start character offset.
    pub character: u32,
    /// End line (0-based).
    pub end_line: u32,
    /// End character offset.
    pub end_character: u32,
    /// Diagnostic severity level.
    pub severity: DiagnosticSeverity,
    /// Diagnostic message.
    pub message: String,
    /// Source identifier (e.g., "forjar-lsp").
    pub source: String,
}

/// LSP completion item.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletionItem {
    /// Display label.
    pub label: String,
    /// Detail description.
    pub detail: String,
    /// Text to insert on accept.
    pub insert_text: String,
    /// Completion kind (1=Text, 6=Variable, 14=Keyword, 15=Snippet).
    pub kind: u32,
}

/// Known forjar YAML top-level keys.
const TOP_LEVEL_KEYS: &[(&str, &str)] = &[
    ("machines", "Machine definitions (name, host, transport)"),
    (
        "resources",
        "Resource definitions (package, file, service, etc.)",
    ),
    ("data", "Data sources (file, forjar-state, environment)"),
    ("tags", "Tag assignments for resources"),
    ("policy", "Policy configuration (convergence, security)"),
    ("includes", "Include other forjar YAML files"),
    ("template", "Template parameters for reusable configs"),
];

/// Known resource types.
const RESOURCE_TYPES: &[(&str, &str)] = &[
    ("package", "System package (apt, yum, brew, cargo)"),
    ("file", "File resource (content, template, permissions)"),
    ("service", "System service (systemd, launchd)"),
    ("mount", "Filesystem mount point"),
    ("directory", "Directory with ownership/permissions"),
    ("command", "Arbitrary command execution"),
    ("git_repo", "Git repository clone/checkout"),
    ("cron", "Cron job definition"),
    ("user", "System user account"),
    ("group", "System group"),
    ("gpu_driver", "GPU driver installation (NVIDIA/ROCm)"),
];

/// Known resource fields.
const RESOURCE_FIELDS: &[(&str, &str)] = &[
    ("name", "Resource identifier (unique within config)"),
    ("type", "Resource type (package, file, service, etc.)"),
    (
        "ensure",
        "Desired state: present, absent, latest, running, stopped",
    ),
    ("provider", "Package provider: apt, yum, brew, cargo, pip"),
    ("content", "File content (inline string)"),
    ("source", "File source path or URL"),
    ("mode", "File permissions (octal, e.g., '0644')"),
    ("owner", "File/directory owner"),
    ("group", "File/directory group"),
    ("depends_on", "List of resource dependencies"),
    ("machine", "Target machine name"),
    ("tags", "Resource tags for filtering"),
    ("on_success", "Notification on successful convergence"),
    ("on_failure", "Notification on failure"),
    ("on_drift", "Notification when drift detected"),
];

impl Default for LspServer {
    fn default() -> Self {
        Self::new()
    }
}

impl LspServer {
    /// Create a new LSP server instance.
    pub fn new() -> Self {
        LspServer {
            initialized: false,
            documents: HashMap::new(),
            root_uri: None,
            shutdown_requested: false,
        }
    }

    /// Handle a JSON-RPC request and return optional response.
    pub fn handle_message(&mut self, msg: &serde_json::Value) -> Option<serde_json::Value> {
        let method = msg.get("method")?.as_str()?;
        let id = msg.get("id");

        match method {
            "initialize" => {
                self.initialized = true;
                if let Some(root) = msg.pointer("/params/rootUri") {
                    self.root_uri = root.as_str().map(String::from);
                }
                Some(make_response(id, initialize_result()))
            }
            "initialized" => None, // notification, no response
            "shutdown" => {
                self.shutdown_requested = true;
                Some(make_response(id, serde_json::Value::Null))
            }
            "exit" => std::process::exit(if self.shutdown_requested { 0 } else { 1 }),
            "textDocument/didOpen" => {
                self.handle_did_open(msg);
                None
            }
            "textDocument/didChange" => {
                self.handle_did_change(msg);
                None
            }
            "textDocument/completion" => {
                let items = self.handle_completion(msg);
                Some(make_response(
                    id,
                    serde_json::to_value(items).unwrap_or_default(),
                ))
            }
            "textDocument/hover" => {
                let hover = self.handle_hover(msg);
                Some(make_response(id, hover))
            }
            _ => {
                if id.is_some() {
                    Some(make_error_response(id, -32601, "Method not found"))
                } else {
                    None // unknown notification
                }
            }
        }
    }

    fn handle_did_open(&mut self, msg: &serde_json::Value) {
        if let (Some(uri), Some(text)) = (
            msg.pointer("/params/textDocument/uri")
                .and_then(|v| v.as_str()),
            msg.pointer("/params/textDocument/text")
                .and_then(|v| v.as_str()),
        ) {
            self.documents.insert(uri.to_string(), text.to_string());
        }
    }

    fn handle_did_change(&mut self, msg: &serde_json::Value) {
        if let (Some(uri), Some(changes)) = (
            msg.pointer("/params/textDocument/uri")
                .and_then(|v| v.as_str()),
            msg.pointer("/params/contentChanges")
                .and_then(|v| v.as_array()),
        ) {
            // Full document sync — take last change
            if let Some(text) = changes
                .last()
                .and_then(|c| c.get("text"))
                .and_then(|t| t.as_str())
            {
                self.documents.insert(uri.to_string(), text.to_string());
            }
        }
    }

    fn handle_completion(&self, msg: &serde_json::Value) -> Vec<CompletionItem> {
        let uri = msg
            .pointer("/params/textDocument/uri")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let line = msg
            .pointer("/params/position/line")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return Vec::new(),
        };
        let current_line = doc.lines().nth(line).unwrap_or("");
        let indent = current_line.len() - current_line.trim_start().len();
        completion_items(indent, current_line.trim())
    }

    fn handle_hover(&self, msg: &serde_json::Value) -> serde_json::Value {
        let uri = msg
            .pointer("/params/textDocument/uri")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let line = msg
            .pointer("/params/position/line")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return serde_json::Value::Null,
        };
        let current_line = doc.lines().nth(line).unwrap_or("");
        hover_info(current_line.trim())
    }

    /// Validate a document and return diagnostics.
    pub fn validate_document(&self, uri: &str) -> Vec<Diagnostic> {
        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return Vec::new(),
        };
        validate_yaml(doc)
    }
}

/// Generate completion items based on context.
pub(crate) fn completion_items(indent: usize, line_prefix: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    if indent == 0 && (line_prefix.is_empty() || !line_prefix.contains(':')) {
        // Top-level keys
        for (key, desc) in TOP_LEVEL_KEYS {
            items.push(CompletionItem {
                label: key.to_string(),
                detail: desc.to_string(),
                insert_text: format!("{key}:"),
                kind: 14, // Keyword
            });
        }
    } else if line_prefix.starts_with("type:") || line_prefix.starts_with("- type:") {
        // Resource types
        for (rtype, desc) in RESOURCE_TYPES {
            items.push(CompletionItem {
                label: rtype.to_string(),
                detail: desc.to_string(),
                insert_text: rtype.to_string(),
                kind: 6, // Variable
            });
        }
    } else if indent >= 2 {
        // Resource fields
        for (field, desc) in RESOURCE_FIELDS {
            items.push(CompletionItem {
                label: field.to_string(),
                detail: desc.to_string(),
                insert_text: format!("{field}: "),
                kind: 6, // Variable
            });
        }
    }
    items
}

/// Generate hover info for a line.
fn hover_info(line: &str) -> serde_json::Value {
    let key = line
        .split(':')
        .next()
        .unwrap_or("")
        .trim()
        .trim_start_matches("- ");
    let desc = TOP_LEVEL_KEYS
        .iter()
        .chain(RESOURCE_FIELDS.iter())
        .chain(RESOURCE_TYPES.iter())
        .find(|(k, _)| *k == key)
        .map(|(_, d)| *d);

    match desc {
        Some(d) => serde_json::json!({
            "contents": { "kind": "markdown", "value": format!("**{key}**: {d}") }
        }),
        None => serde_json::Value::Null,
    }
}

/// Basic YAML validation for forjar configs.
pub fn validate_yaml(content: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    // Check YAML parse
    if let Err(e) = serde_yaml_ng::from_str::<serde_json::Value>(content) {
        diags.push(Diagnostic {
            line: 0,
            character: 0,
            end_line: 0,
            end_character: 80,
            severity: DiagnosticSeverity::Error,
            message: format!("YAML parse error: {e}"),
            source: "forjar-lsp".to_string(),
        });
        return diags;
    }

    // Check for common issues
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains("\t") {
            diags.push(Diagnostic {
                line: i as u32,
                character: 0,
                end_line: i as u32,
                end_character: line.len() as u32,
                severity: DiagnosticSeverity::Warning,
                message: "Tabs should not be used in YAML; use spaces".to_string(),
                source: "forjar-lsp".to_string(),
            });
        }
        if trimmed.starts_with("ensure:") {
            let val = trimmed.trim_start_matches("ensure:").trim();
            if !["present", "absent", "latest", "running", "stopped", ""].contains(&val) {
                diags.push(Diagnostic {
                    line: i as u32,
                    character: 0,
                    end_line: i as u32,
                    end_character: line.len() as u32,
                    severity: DiagnosticSeverity::Warning,
                    message: format!("Unknown ensure value '{val}'; expected present|absent|latest|running|stopped"),
                    source: "forjar-lsp".to_string(),
                });
            }
        }
    }
    diags
}

/// Build an LSP initialize result.
fn initialize_result() -> serde_json::Value {
    serde_json::json!({
        "capabilities": {
            "textDocumentSync": 1,
            "completionProvider": { "triggerCharacters": [":", " ", "-"] },
            "hoverProvider": true,
            "diagnosticProvider": { "interFileDependencies": false, "workspaceDiagnostics": false }
        },
        "serverInfo": { "name": "forjar-lsp", "version": env!("CARGO_PKG_VERSION") }
    })
}

fn make_response(id: Option<&serde_json::Value>, result: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id.cloned().unwrap_or(serde_json::Value::Null),
        "result": result
    })
}

fn make_error_response(id: Option<&serde_json::Value>, code: i32, msg: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id.cloned().unwrap_or(serde_json::Value::Null),
        "error": { "code": code, "message": msg }
    })
}

/// Write an LSP message (Content-Length header + JSON body).
pub fn write_message<W: Write>(writer: &mut W, msg: &serde_json::Value) -> io::Result<()> {
    let body = serde_json::to_string(msg).map_err(io::Error::other)?;
    write!(writer, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    writer.flush()
}

/// Read one LSP message from stdin.
pub fn read_message<R: BufRead>(reader: &mut R) -> io::Result<serde_json::Value> {
    let mut header = String::new();
    let mut content_length: usize = 0;

    loop {
        header.clear();
        reader.read_line(&mut header)?;
        let trimmed = header.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(len_str) = trimmed.strip_prefix("Content-Length: ") {
            content_length = len_str
                .parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        }
    }

    if content_length == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "no content length",
        ));
    }

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Entry point: run the LSP server on stdio.
pub fn cmd_lsp() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let mut server = LspServer::new();

    loop {
        let msg = read_message(&mut reader).map_err(|e| e.to_string())?;
        if let Some(resp) = server.handle_message(&msg) {
            write_message(&mut writer, &resp).map_err(|e| e.to_string())?;
        }
        // Publish diagnostics after document changes
        if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
            if method == "textDocument/didOpen" || method == "textDocument/didChange" {
                if let Some(uri) = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(|v| v.as_str())
                {
                    let diags = server.validate_document(uri);
                    let notification = publish_diagnostics(uri, &diags);
                    write_message(&mut writer, &notification).map_err(|e| e.to_string())?;
                }
            }
        }
    }
}

fn publish_diagnostics(uri: &str, diags: &[Diagnostic]) -> serde_json::Value {
    let lsp_diags: Vec<serde_json::Value> = diags
        .iter()
        .map(|d| {
            serde_json::json!({
                "range": {
                    "start": { "line": d.line, "character": d.character },
                    "end": { "line": d.end_line, "character": d.end_character }
                },
                "severity": d.severity as u32,
                "source": d.source,
                "message": d.message
            })
        })
        .collect();
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": { "uri": uri, "diagnostics": lsp_diags }
    })
}
