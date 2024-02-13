use remod_config::Config;
use remod_core::arrow_components::ArrowExpressionComponents;
use remod_core::storybook::Storybook;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use tokio::try_join;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    config: Config,
    commands: Vec<String>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: self.commands.to_owned(),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                }),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        change: Some(TextDocumentSyncKind::FULL),
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let may_be_file = params.text_document.uri.to_file_path();
        let mut code_lens: Vec<CodeLens> = vec![];
        match may_be_file {
            Ok(path) => {
                self.client
                    .log_message(
                        MessageType::INFO,
                        format!("code lens working {}", path.display()),
                    )
                    .await;
                let mut arrow_components = ArrowExpressionComponents::new(&path, &self.config);
                arrow_components.extract_components();

                arrow_components.variables.iter().for_each(|s| {
                    let (start, end) = arrow_components.look_up_variable(s);
                    let full_file_path = path.as_os_str().to_str().unwrap().to_string();
                    let mut cmd_map = Map::new();
                    cmd_map.insert(
                        "document_uri".to_string(),
                        Value::String(String::from(&full_file_path)),
                    );
                    cmd_map.insert("symbol".to_string(), Value::String(s.sym.to_string()));
                    code_lens.push(CodeLens {
                        range: Range {
                            start: Position {
                                line: (start.line as u32) - 1,
                                character: start.col.0 as u32,
                            },
                            end: Position {
                                line: (end.line as u32) - 1,
                                character: end.col.0 as u32,
                            },
                        },
                        command: Some(Command {
                            title: "Create Story".to_string(),
                            command: "create_story".to_string(),
                            arguments: Some(vec![Value::Object(cmd_map)]),
                        }),
                        data: Some(Value::String(full_file_path)),
                    });
                });
                Ok(Some(code_lens))
            }
            Err(_) => Ok(Some(code_lens)),
        }
    }
    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        if &params.command == "create_story" {
            let arg = params.arguments.get(0);
            self.client
                .log_message(MessageType::LOG, arg.clone().unwrap())
                .await;
            let mut symbol: Option<String> = None;
            let mut path: Option<PathBuf> = None;
            match arg {
                Some(value) => match value {
                    Value::Object(v) => {
                        let may_be_file = v.get("document_uri");
                        let may_be_symbol = v.get("symbol");
                        match may_be_symbol {
                            Some(v) => match v {
                                Value::String(s) => {
                                    symbol = Some(s.to_string());
                                }
                                _ => {}
                            },
                            None => {}
                        }
                        match may_be_file {
                            Some(f) => match f {
                                Value::String(file) => path = Some(PathBuf::from(file)),
                                _ => {}
                            },
                            None => {}
                        }
                    }
                    _ => {}
                },
                None => {}
            };
            if symbol == None && path == None {
                self.client
                    .log_message(MessageType::ERROR, "Error parsing command arguments")
                    .await;
            } else {
                let mut story_book = Storybook::default();

                let story_file =
                    story_book.pre_process_story_module(symbol, &path.unwrap(), &self.config);
                match story_file {
                    Ok((stf, file)) => {
                        let path = Path::new(&file);
                        self.client
                            .log_message(MessageType::LOG, format!("Trying to parse {}", &file))
                            .await;
                        let may_be_uri = Url::from_file_path(path);
                        match may_be_uri {
                            Ok(uri) => {
                                let create_file = CreateFile {
                                    uri: uri.clone(),
                                    options: Some(CreateFileOptions {
                                        ignore_if_exists: Some(false),
                                        overwrite: Some(false),
                                    }),
                                    annotation_id: Some("STORY:CREATE".to_string()),
                                };
                                let text_edits: Vec<OneOf<TextEdit, AnnotatedTextEdit>> =
                                    vec![OneOf::Left(TextEdit {
                                        range: Range {
                                            start: Position {
                                                line: 0,
                                                character: 0,
                                            },
                                            end: Position {
                                                line: 0,
                                                character: 0,
                                            },
                                        },
                                        new_text: stf.emit_story_file(),
                                    })];
                                let text_document_edit = TextDocumentEdit {
                                    text_document: OptionalVersionedTextDocumentIdentifier {
                                        uri: uri.clone(),
                                        version: None,
                                    },
                                    edits: text_edits,
                                };
                                let create = self.client.apply_edit(WorkspaceEdit {
                                    changes: None,
                                    document_changes: Some(DocumentChanges::Operations(vec![
                                        DocumentChangeOperation::Op(ResourceOp::Create(
                                            create_file,
                                        )),
                                    ])),
                                    change_annotations: None,
                                });

                                let result = try_join!(create);
                                match result {
                                    Ok(ref c) => {
                                        if c.0.applied {
                                            self.client
                                                .log_message(
                                                    MessageType::INFO,
                                                    "Create Story successfull",
                                                )
                                                .await;
                                            let edit = self.client.apply_edit(WorkspaceEdit {
                                                changes: None,
                                                document_changes: Some(
                                                    DocumentChanges::Operations(vec![
                                                        DocumentChangeOperation::Edit(
                                                            text_document_edit,
                                                        ),
                                                    ]),
                                                ),
                                                change_annotations: None,
                                            });
                                            let edit_result = try_join!(edit);
                                            match edit_result {
                                                Ok(ref ed) => {
                                                    dbg!(ed);
                                                    if ed.0.applied {
                                                        self.client
                                                            .log_message(
                                                                MessageType::INFO,
                                                                "Story successfully populated",
                                                            )
                                                            .await;
                                                        let (start_loc, end_loc) =
                                                            stf.find_story_ident_loc(&self.config);
                                                        let selection_range = Some(Range {
                                                            start: Position {
                                                                line: (start_loc.line as u32) - 1,
                                                                character: start_loc.col_display
                                                                    as u32,
                                                            },
                                                            end: Position {
                                                                line: (end_loc.line as u32) - 1,
                                                                character: end_loc.col_display
                                                                    as u32,
                                                            },
                                                        });
                                                        let _ = self
                                                            .client
                                                            .show_document(ShowDocumentParams {
                                                                uri,
                                                                external: Some(false),
                                                                take_focus: Some(true),
                                                                selection: selection_range,
                                                            })
                                                            .await;
                                                    }
                                                }
                                                Err(_) => todo!(),
                                            }
                                        }
                                    }
                                    Err(e) => self.client.log_message(MessageType::ERROR, e).await,
                                }
                            }
                            Err(..) => {
                                self.client
                                    .log_message(MessageType::ERROR, "Error parsing URI")
                                    .await
                            }
                        }
                    }
                    Err(e) => self.client.log_message(MessageType::ERROR, e).await,
                }
            }
        }
        Ok(Some(Value::Null))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let commands: Vec<String> = vec!["create_story".to_string()];
    let (service, socket) = LspService::new(|client| Backend {
        client,
        commands,
        config: Config {
            typescript: Some(true),
            ..Default::default()
        },
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
