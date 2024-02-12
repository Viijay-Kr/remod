use std::path::PathBuf;

use remod_config::Config;
use remod_core::arrow_components::ArrowExpressionComponents;
use remod_core::storybook::Storybook;
use serde_json::{Map, Value};
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
            match arg {
                Some(value) => match value {
                    Value::Object(v) => {
                        let may_be_file = v.get("document_uri");
                        let may_be_symbol = v.get("symbol");
                        let mut symbol: Option<String> = None;
                        let mut path = PathBuf::default();
                        match may_be_symbol {
                            Some(v) => match v {
                                Value::String(s) => {
                                    symbol = Some(s.to_string());
                                }
                                _ => {
                                    self.client
                                        .log_message(
                                            MessageType::ERROR,
                                            "Something wrong with the command arguments",
                                        )
                                        .await
                                }
                            },
                            None => {}
                        }
                        match may_be_file {
                            Some(f) => match f {
                                Value::String(file) => path = PathBuf::from(file),
                                _ => {
                                    self.client
                                        .log_message(
                                            MessageType::ERROR,
                                            "Something wrong with the command arguments",
                                        )
                                        .await
                                }
                            },
                            None => {
                                self.client
                                    .log_message(
                                        MessageType::ERROR,
                                        "Something wrong with the command arguments",
                                    )
                                    .await
                            }
                        }
                        let mut story_book = Storybook::default();
                        story_book.create_story(symbol, &path, &self.config)
                    }
                    _ => {
                        self.client
                            .log_message(
                                MessageType::ERROR,
                                "Something wrong with the command arguments",
                            )
                            .await
                    }
                },
                None => {
                    self.client
                        .log_message(
                            MessageType::ERROR,
                            "Something wrong with the command arguments",
                        )
                        .await
                }
            }
        }
        // self.client.apply_edit(WorkspaceEdit {
        //     changes: (),
        //     document_changes: Some(DocumentChanges::Operations(vec![
        //         DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
        //             uri: todo!(),
        //             options: todo!(),
        //             annotation_id: todo!(),
        //         })),
        //     ])),
        //     change_annotations: (),
        // })
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
