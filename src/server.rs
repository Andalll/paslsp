use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use url::Url;

use crate::parser::{Parser, SymbolTable};

pub struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, DocumentState>>>,
}

pub struct DocumentState {
    pub text: String,
    pub symbols: SymbolTable,
}

fn p3(l: usize, c: usize) -> Position {
    Position { line: l as u32, character: c as u32 }
}

fn rng(start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Range {
    Range { start: p3(start_line, start_col), end: p3(end_line, end_col) }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        self.client
            .log_message(MessageType::INFO, "PasLsp initialized")
            .await;

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "PasLsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "PasLsp ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        let symbols = parse_document(&text);
        self.documents
            .write()
            .await
            .insert(uri.clone(), DocumentState { text, symbols });
        self.publish_diagnostics(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let new_text = params
            .content_changes
            .into_iter()
            .find_map(|change| Some(change.text))
            .unwrap_or_default();

        {
            let mut docs = self.documents.write().await;
            match docs.get_mut(&uri) {
                Some(doc) => {
                    doc.text = new_text.clone();
                    doc.symbols = parse_document(&doc.text);
                }
                None => {
                    let symbols = parse_document(&new_text);
                    docs.insert(uri.clone(), DocumentState { text: new_text.clone(), symbols });
                }
            }
        }
        self.publish_diagnostics(&uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.write().await.remove(&uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let docs = self.documents.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let _position = params.text_document_position.position;
        let _line = doc.text.lines().nth(0).unwrap_or("");

        let mut items: Vec<CompletionItem> = PASCAL_KEYWORDS
            .iter()
            .map(|kw| CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Pascal keyword".to_string()),
                ..Default::default()
            })
            .collect();

        for (name, (type_name, _, _)) in &doc.symbols.vars {
            if !items.iter().any(|i| i.label == *name) {
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(format!("variable: {}", type_name)),
                    ..Default::default()
                });
            }
        }
        for (name, _) in &doc.symbols.procs {
            if !items.iter().any(|i| i.label == *name) {
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some("procedure/function".to_string()),
                    ..Default::default()
                });
            }
        }

        Ok(Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items,
        })))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let docs = self.documents.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let position = params.text_document_position_params.position;
        let line = doc.text.lines().nth(position.line as usize).unwrap_or("");
        let col = position.character as usize;
        let word = word_at(line, col);

        let contents = if PASCAL_KEYWORDS.contains(&word.as_str()) {
            format!("Pascal keyword: `{}`", word)
        } else if let Some((type_name, l, c)) = doc.symbols.vars.get(&word) {
            format!(
                "Variable `{}`: type `{}` (declared line {}, col {})",
                word, type_name, l, c
            )
        } else if let Some((l, c)) = doc.symbols.procs.get(&word) {
            format!("Procedure/Function `{}` (declared line {}, col {})", word, l, c)
        } else if !word.is_empty() {
            format!("Identifier: `{}`", word)
        } else {
            return Ok(None);
        };

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::PlainText,
                value: contents,
            }),
            range: None,
        }))
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let docs = self.documents.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let position = params.text_document_position_params.position;
        let line = doc.text.lines().nth(position.line as usize).unwrap_or("");
        let col = position.character as usize;
        let word = word_at(line, col);

        let mut locations: Vec<Location> = Vec::new();

        if let Some((_t, l, c)) = doc.symbols.vars.get(&word) {
            locations.push(Location {
                uri: uri.clone(),
                range: rng(*l - 1, *c - 1, *l - 1, (*c - 1) + word.len()),
            });
        } else if let Some((l, c)) = doc.symbols.procs.get(&word) {
            locations.push(Location {
                uri: uri.clone(),
                range: rng(*l - 1, *c - 1, *l - 1, (*c - 1) + word.len()),
            });
        }

        Ok(Some(GotoDefinitionResponse::Scalar(locations.into_iter().next().unwrap_or(Location {
            uri,
            range: Range { start: Position::default(), end: Position::default() },
        }))))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let _context = params.context;
        let docs = self.documents.read().await;
        let doc = match docs.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let position = params.text_document_position.position;
        let line = doc.text.lines().nth(position.line as usize).unwrap_or("");
        let col = position.character as usize;
        let word = word_at(line, col);

        let mut locations: Vec<Location> = Vec::new();

        if let Some((_t, l, c)) = doc.symbols.vars.get(&word) {
            locations.push(Location {
                uri: uri.clone(),
                range: rng(*l - 1, *c - 1, *l - 1, (*c - 1) + word.len()),
            });
        } else if let Some((l, c)) = doc.symbols.procs.get(&word) {
            locations.push(Location {
                uri: uri.clone(),
                range: rng(*l - 1, *c - 1, *l - 1, (*c - 1) + word.len()),
            });
        }

        for (i, text_line) in doc.text.lines().enumerate() {
            let mut start = 0;
            while let Some(found) = text_line[start..].find(&word) {
                let abs_start = start + found;
                let abs_end = abs_start + word.len();
                let left_ok = abs_start == 0
                    || !text_line[abs_start - 1..abs_start].chars().next().unwrap_or(' ').is_alphanumeric();
                let right_ok = abs_end == text_line.len()
                    || !text_line[abs_end..abs_end + 1].chars().next().unwrap_or(' ').is_alphanumeric();
                if left_ok && right_ok && !(i as u32 == position.line && abs_start as u32 == col as u32) {
                    locations.push(Location {
                        uri: uri.clone(),
                        range: rng(i, abs_start, i, abs_end),
                    });
                }
                start = abs_end;
            }
        }

        locations.sort_by_key(|loc| (loc.uri.clone(), loc.range.start.line, loc.range.start.character));
        locations.dedup_by(|a, b| {
            a.uri == b.uri
                && a.range.start.line == b.range.start.line
                && a.range.start.character == b.range.start.character
        });

        Ok(Some(locations))
    }

    async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        let _uri = params.text_document.uri;
        let docs = self.documents.read().await;
        let doc = match docs.get(&_uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let mut symbols: Vec<DocumentSymbol> = Vec::new();

        for (name, (type_name, line, col)) in &doc.symbols.vars {
            symbols.push(DocumentSymbol {
                name: name.clone(),
                kind: SymbolKind::VARIABLE,
                tags: None,
                deprecated: None,
                detail: Some(format!("variable: {}", type_name)),
                range: rng(*line - 1, *col - 1, *line - 1, (*col - 1) + name.len()),
                selection_range: rng(*line - 1, *col - 1, *line - 1, (*col - 1) + name.len()),
                children: None,
            });
        }

        for (name, (line, col)) in &doc.symbols.procs {
            symbols.push(DocumentSymbol {
                name: name.clone(),
                kind: SymbolKind::FUNCTION,
                tags: None,
                deprecated: None,
                detail: Some("procedure/function".to_string()),
                range: rng(*line - 1, *col - 1, *line - 1, (*col - 1) + name.len()),
                selection_range: rng(*line - 1, *col - 1, *line - 1, (*col - 1) + name.len()),
                children: None,
            });
        }

        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn publish_diagnostics(&self, uri: &Url) {
        let text = {
            let docs = self.documents.read().await;
            match docs.get(uri) {
                Some(d) => d.text.clone(),
                None => return,
            }
        };

        let diagnostics = check_pascal(&text);

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}

fn parse_document(text: &str) -> SymbolTable {
    let mut parser = Parser::new(text);
    let _ = parser.parse_program();
    parser.symbols
}

fn check_pascal(text: &str) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut begin_count: i32 = 0;
    let mut end_count: i32 = 0;

    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim();

        if trimmed == "begin" || trimmed.starts_with("begin ") {
            begin_count += 1;
        }
        if trimmed == "end"
            || trimmed.starts_with("end;")
            || trimmed.starts_with("end.")
            || trimmed.starts_with("end ")
        {
            end_count += 1;
        }

        if trimmed.starts_with("program ") && !trimmed.ends_with(";") {
            diagnostics.push(Diagnostic {
                range: rng(i, 0, i, trimmed.len()),
                severity: Some(DiagnosticSeverity::WARNING),
                message: "program statement should end with semicolon".to_string(),
                source: Some("paslsp".to_string()),
                ..Default::default()
            });
        }
    }

    if begin_count > end_count {
        diagnostics.push(Diagnostic {
            range: rng(0, 0, 0, 0),
            severity: Some(DiagnosticSeverity::ERROR),
            message: format!(
                "unmatched begin/end: {} begin(s), {} end(s)",
                begin_count, end_count
            ),
            source: Some("paslsp".to_string()),
            ..Default::default()
        });
    }

    diagnostics
}

fn word_at(line: &str, col: usize) -> String {
    let bytes = line.as_bytes();
    let pos = col.min(bytes.len());

    let mut start = pos;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }

    let mut end = pos;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }

    line[start..end].to_string()
}

const PASCAL_KEYWORDS: &[&str] = &[
    "absolute", "and", "array", "as", "asm", "case", "const", "constructor", "destructor",
    "div", "do", "downto", "else", "end", "except", "exports", "file", "finalization", "finally",
    "for", "function", "goto", "if", "implementation", "in", "inherited", "inline", "interface",
    "is", "label", "library", "mod", "nil", "not", "object", "of", "only", "operator", "or",
    "otherwise", "packed", "procedure", "program", "record", "reintroduce", "repeat", "resident",
    "self", "set", "shl", "shr", "string", "then", "to", "type", "unit", "until", "uses", "var",
    "virtual", "while", "with", "xor", "begin",
];
