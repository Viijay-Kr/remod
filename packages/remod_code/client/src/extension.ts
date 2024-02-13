/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

import {
  languages,
  workspace,
  EventEmitter,
  ExtensionContext,
  window,
  InlayHintsProvider,
  TextDocument,
  CancellationToken,
  Range,
  InlayHint,
  TextDocumentChangeEvent,
  ProviderResult,
  commands,
  WorkspaceEdit,
  TextEdit,
  Selection,
  Uri,
  CodeLensProvider,
  CodeLens,
  Event,
} from "vscode";

import {
  CodeLensParams,
  Disposable,
  DocumentUri,
  Executable,
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";
import {
  ClientCapabilities,
  DocumentSelector,
  FeatureState,
  InitializeParams,
  ProtocolNotificationType0,
  ResourceOperationKind,
  ServerCapabilities,
  StaticFeature,
} from "vscode-languageclient";

let client: LanguageClient;
// type a = Parameters<>;

const documentSelector = [{ scheme: "file", language: "typescriptreact" }];

export class WorkaroundFeature implements StaticFeature {
  getState(): FeatureState {
    return {} as FeatureState;
  }
  dispose(): void {
    client.dispose();
  }
  fillInitializeParams?: (params: InitializeParams) => void;
  preInitialize?: (
    capabilities: ServerCapabilities<any>,
    documentSelector: DocumentSelector
  ) => void;
  initialize(
    capabilities: ServerCapabilities<any>,
    documentSelector: DocumentSelector
  ): void {
    client.sendNotification(new ProtocolNotificationType0("initialize"));
  }

  fillClientCapabilities(capabilities: ClientCapabilities): void {
    capabilities.workspace.workspaceEdit = { documentChanges: true };
    capabilities.workspace.applyEdit = true;
    capabilities.workspace.workspaceEdit.documentChanges = true;
    capabilities.workspace.workspaceEdit.resourceOperations = [
      ResourceOperationKind.Create,
    ];
    capabilities.workspace.executeCommand = { dynamicRegistration: false };
    capabilities.workspace.fileOperations.willCreate = true;
  }
}
export async function activate(context: ExtensionContext) {
  let disposable = commands.registerCommand(
    "remod.create-story",
    async (uri) => {
      window.activeTextEditor.document;
      let editor = window.activeTextEditor;
      let range = new Range(1, 1, 1, 1);
      editor.selection = new Selection(range.start, range.end);
    }
  );

  context.subscriptions.push(disposable);

  const traceOutputChannel = window.createOutputChannel(
    "Remod Langauge Client"
  );
  const command = process.env.SERVER_PATH || "remod_code";
  const run: Executable = {
    command,
    options: {
      env: {
        ...process.env,
        // eslint-disable-next-line @typescript-eslint/naming-convention
        RUST_LOG: "debug",
        RUST_BACKTRACE: "full",
      },
    },
  };

  const serverOptions: ServerOptions = {
    run,
    debug: run,
  };
  // If the extension is launched in debug mode then the debug server options are used
  // Otherwise the run options are used
  // Options to control the language client
  let clientOptions: LanguageClientOptions = {
    // Register the server for plain text documents
    documentSelector,
    synchronize: {
      // Notify the server about file changes to '.clientrc files contained in the workspace
      fileEvents: workspace.createFileSystemWatcher("**/.clientrc"),
    },
    traceOutputChannel,
  };
  // Create the language client and start the client.
  client = new LanguageClient(
    "remod-language-server",
    "remod-language-server",
    serverOptions,
    clientOptions
  );

  registerFeatures(context);
  client.registerProposedFeatures();
  client.registerFeature(new WorkaroundFeature());
  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

export function registerFeatures(ctx: ExtensionContext) {
  const maybeUpdater = {
    hintsProvider: null as Disposable | null,
    updateHintsEventEmitter: new EventEmitter<void>(),
    async onConfigChange() {
      this.dispose();
      // languages.registerCodeLensProvider(documentSelector, new LensProvider());
    },

    onDidChangeTextDocument({
      contentChanges,
      document,
    }: TextDocumentChangeEvent) {
      // debugger
      // this.updateHintsEventEmitter.fire();
    },

    dispose() {
      this.hintsProvider?.dispose();
      this.hintsProvider = null;
      this.updateHintsEventEmitter.dispose();
    },
  };

  workspace.onDidChangeConfiguration(
    maybeUpdater.onConfigChange,
    maybeUpdater,
    ctx.subscriptions
  );
  workspace.onDidChangeTextDocument(
    maybeUpdater.onDidChangeTextDocument,
    maybeUpdater,
    ctx.subscriptions
  );
  maybeUpdater.onConfigChange().catch(console.error);
}

class LensProvider implements CodeLensProvider {
  onDidChangeCodeLenses?: Event<void>;
  async provideCodeLenses(
    document: TextDocument,
    token: CancellationToken
  ): Promise<CodeLens[]> {
    if (DocumentUri.is(document.uri)) {
      const result = await client.sendRequest<CodeLens[]>(
        "textDocument/codeLens",
        {
          textDocument: {
            uri: document.uri,
          },
        } as CodeLensParams
      );
      console.log(">>> result", result);
      return result;
    }
  }
  resolveCodeLens?(
    codeLens: CodeLens,
    token: CancellationToken
  ): ProviderResult<CodeLens> {
    throw new Error("Method not implemented.");
  }
}
