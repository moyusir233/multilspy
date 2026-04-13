//! LSP request parameter structures.
//!
//! This module defines the parameter types for various LSP requests, including
//! the `initialize` handshake, text document operations, and call hierarchy queries.
//!
//! # Structures
//!
//! | Structure | LSP Method / Spec Section |
//! |-----------|--------------------------|
//! | [`InitializeParams`] | `initialize` — [InitializeParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initializeParams) |
//! | [`ClientInfo`] | Nested in `InitializeParams` — [clientInfo](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initializeParams) |
//! | [`ClientCapabilities`] | [ClientCapabilities](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#clientCapabilities) |
//! | [`DefinitionParams`] | `textDocument/definition` — [DefinitionParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#definitionParams) |
//! | [`TypeDefinitionParams`] | `textDocument/typeDefinition` — [TypeDefinitionParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#typeDefinitionParams) |
//! | [`ReferencesParams`] | `textDocument/references` — [ReferenceParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#referenceParams) |
//! | [`DocumentSymbolParams`] | `textDocument/documentSymbol` — [DocumentSymbolParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#documentSymbolParams) |
//! | [`ImplementationParams`] | `textDocument/implementation` — [ImplementationParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#implementationParams) |
//! | [`CallHierarchyPrepareParams`] | `textDocument/prepareCallHierarchy` — [CallHierarchyPrepareParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_prepareCallHierarchy) |
//! | [`CallHierarchyIncomingCallsParams`] | `callHierarchy/incomingCalls` — [CallHierarchyIncomingCallsParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_incomingCalls) |
//! | [`CallHierarchyOutgoingCallsParams`] | `callHierarchy/outgoingCalls` — [CallHierarchyOutgoingCallsParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_outgoingCalls) |

use super::common::*;
use serde::{Deserialize, Serialize};

/// Parameters for the `initialize` request.
///
/// The `initialize` request is sent as the first request from the client to the server.
/// Until the server has responded with an [`InitializeResult`](super::super::protocol::responses::InitializeResult),
/// the client must not send any additional requests or notifications.
///
/// # Wire Format
///
/// ```json
/// {
///   "processId": 1234,
///   "clientInfo": { "name": "my-editor", "version": "1.0.0" },
///   "rootUri": "file:///path/to/workspace",
///   "capabilities": {},
///   "trace": "off",
///   "workspaceFolders": [{ "uri": "file:///path/to/workspace", "name": "my-project" }]
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `process_id` | `Option<u32>` | Yes (nullable) | The process ID of the parent process that started the server. `null` if not started by another process. LSP type: `integer \| null`. Wire name: `processId`. |
/// | `client_info` | `Option<ClientInfo>` | No | Information about the client. @since 3.15.0. Wire name: `clientInfo`. |
/// | `locale` | `Option<String>` | No | The locale the client is currently showing the user interface in (IETF language tag). @since 3.16.0. |
/// | `root_uri` | `Option<String>` | Yes (nullable) | The rootUri of the workspace. `null` if no folder is open. LSP type: `DocumentUri \| null`. **Deprecated** in favour of `workspaceFolders`. Wire name: `rootUri`. |
/// | `initialization_options` | `Option<Value>` | No | User provided initialization options. LSP type: `LSPAny`. Wire name: `initializationOptions`. |
/// | `capabilities` | [`ClientCapabilities`] | Yes | The capabilities provided by the client (editor or tool). |
/// | `trace` | `Option<String>` | No | The initial trace setting. If omitted, trace is disabled (`"off"`). LSP type: `TraceValue` (`"off"` \| `"messages"` \| `"verbose"`). |
/// | `workspace_folders` | `Option<Vec<WorkspaceFolder>>` | No | The workspace folders configured in the client when the server starts. `null` if the client supports workspace folders but none are configured. Wire name: `workspaceFolders`. |
///
/// # LSP Specification
///
/// See [InitializeParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initializeParams).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// The process ID of the parent process that started the server.
    /// Is `null` if the process has not been started by another process.
    /// If the parent process is not alive then the server should exit its process.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,

    /// Information about the client.
    ///
    /// @since 3.15.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_info: Option<ClientInfo>,

    /// The locale the client is currently showing the user interface in.
    /// This must not necessarily be the locale of the operating system.
    /// Uses IETF language tags as the value's syntax.
    ///
    /// @since 3.16.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,

    /// The rootUri of the workspace. Is `null` if no folder is open.
    /// If both `rootPath` and `rootUri` are set, `rootUri` wins.
    ///
    /// **Deprecated** in favour of `workspaceFolders`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_uri: Option<String>,

    /// User provided initialization options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialization_options: Option<serde_json::Value>,

    /// The capabilities provided by the client (editor or tool).
    pub capabilities: ClientCapabilities,

    /// The initial trace setting. If omitted trace is disabled (`"off"`).
    ///
    /// Valid values: `"off"`, `"messages"`, `"verbose"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,

    /// The workspace folders configured in the client when the server starts.
    /// This property is only available if the client supports workspace folders.
    /// It can be `null` if the client supports workspace folders but none are configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_folders: Option<Vec<WorkspaceFolder>>,
}

/// Information about the client application.
///
/// Nested within [`InitializeParams`] to identify the client to the server.
///
/// # Wire Format
///
/// ```json
/// { "name": "my-editor", "version": "1.0.0" }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `name` | `String` | Yes | The name of the client as defined by the client. |
/// | `version` | `Option<String>` | No | The client's version as defined by the client. |
///
/// # LSP Specification
///
/// See [InitializeParams.clientInfo](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initializeParams).
///
/// @since 3.15.0
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    /// The name of the client as defined by the client.
    pub name: String,

    /// The client's version as defined by the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Client capabilities communicated during the `initialize` handshake.
///
/// This is a partial implementation covering the capabilities needed by this crate.
/// Unknown capabilities received from the server are preserved in the `other` field
/// via `#[serde(flatten)]`.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `workspace` | `Option<WorkspaceClientCapabilities>` | No | Workspace-specific client capabilities. |
/// | `text_document` | `Option<TextDocumentClientCapabilities>` | No | Text document-specific client capabilities. Wire name: `textDocument`. |
/// | `other` | `Map<String, Value>` | — | Catch-all for unrecognized capability fields. |
///
/// # LSP Specification
///
/// See [ClientCapabilities](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#clientCapabilities).
///
/// **Note:** This is a partial representation. The full LSP `ClientCapabilities` interface
/// includes additional sections such as `window`, `general`, and `notebookDocument`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    /// Workspace-specific client capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceClientCapabilities>,

    /// Text document-specific client capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_document: Option<TextDocumentClientCapabilities>,

    /// Catch-all for additional capability fields not explicitly modeled.
    #[serde(flatten)]
    pub other: serde_json::Map<String, serde_json::Value>,
}

/// Workspace-specific client capabilities.
///
/// Partial implementation covering workspace folder support.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `workspace_folders` | `Option<bool>` | No | The client supports workspace folders. Wire name: `workspaceFolders`. |
///
/// # LSP Specification
///
/// See [workspace client capabilities](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#clientCapabilities).
///
/// **Note:** This is a partial representation. The full LSP workspace capabilities include
/// `applyEdit`, `workspaceEdit`, `didChangeConfiguration`, `symbol`, `executeCommand`, etc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceClientCapabilities {
    /// The client has support for workspace folders.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_folders: Option<bool>,
}

/// Text document-specific client capabilities.
///
/// Partial implementation covering the capabilities for features implemented in this crate.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `definition` | `Option<DefinitionClientCapabilities>` | No | Capabilities for `textDocument/definition`. |
/// | `type_definition` | `Option<TypeDefinitionClientCapabilities>` | No | Capabilities for `textDocument/typeDefinition`. Wire name: `typeDefinition`. |
/// | `references` | `Option<ReferencesClientCapabilities>` | No | Capabilities for `textDocument/references`. |
/// | `document_symbol` | `Option<DocumentSymbolClientCapabilities>` | No | Capabilities for `textDocument/documentSymbol`. Wire name: `documentSymbol`. |
/// | `implementation` | `Option<ImplementationClientCapabilities>` | No | Capabilities for `textDocument/implementation`. |
/// | `call_hierarchy` | `Option<CallHierarchyClientCapabilities>` | No | Capabilities for `textDocument/prepareCallHierarchy`. Wire name: `callHierarchy`. |
///
/// # LSP Specification
///
/// See [TextDocumentClientCapabilities](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocumentClientCapabilities).
///
/// **Note:** This is a partial representation. The full LSP interface includes many more
/// capabilities such as `completion`, `hover`, `signatureHelp`, `codeAction`, etc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentClientCapabilities {
    /// Capabilities specific to the `textDocument/definition` request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<DefinitionClientCapabilities>,

    /// Capabilities specific to the `textDocument/typeDefinition` request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_definition: Option<TypeDefinitionClientCapabilities>,

    /// Capabilities specific to the `textDocument/references` request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<ReferencesClientCapabilities>,

    /// Capabilities specific to the `textDocument/documentSymbol` request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_symbol: Option<DocumentSymbolClientCapabilities>,

    /// Capabilities specific to the `textDocument/implementation` request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation: Option<ImplementationClientCapabilities>,

    /// Capabilities specific to the `textDocument/prepareCallHierarchy` request.
    ///
    /// @since 3.16.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_hierarchy: Option<CallHierarchyClientCapabilities>,
}

/// Client capabilities for the `textDocument/definition` request.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `dynamic_registration` | `Option<bool>` | No | Whether definition supports dynamic registration. Wire name: `dynamicRegistration`. |
/// | `link_support` | `Option<bool>` | No | The client supports additional metadata in the form of definition links. Wire name: `linkSupport`. @since 3.14.0. |
///
/// # LSP Specification
///
/// See [DefinitionClientCapabilities](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#definitionClientCapabilities).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionClientCapabilities {
    /// Whether definition supports dynamic registration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,

    /// The client supports additional metadata in the form of definition links.
    ///
    /// @since 3.14.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_support: Option<bool>,
}

/// Client capabilities for the `textDocument/typeDefinition` request.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `dynamic_registration` | `Option<bool>` | No | Whether type definition supports dynamic registration. Wire name: `dynamicRegistration`. |
/// | `link_support` | `Option<bool>` | No | The client supports additional metadata in the form of definition links. Wire name: `linkSupport`. @since 3.14.0. |
///
/// # LSP Specification
///
/// See [TypeDefinitionClientCapabilities](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#typeDefinitionClientCapabilities).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TypeDefinitionClientCapabilities {
    /// Whether type definition supports dynamic registration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,

    /// The client supports additional metadata in the form of definition links.
    ///
    /// @since 3.14.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_support: Option<bool>,
}

/// Client capabilities for the `textDocument/references` request.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `dynamic_registration` | `Option<bool>` | No | Whether references supports dynamic registration. Wire name: `dynamicRegistration`. |
///
/// # LSP Specification
///
/// See [ReferenceClientCapabilities](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#referenceClientCapabilities).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReferencesClientCapabilities {
    /// Whether references supports dynamic registration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,
}

/// Client capabilities for the `textDocument/documentSymbol` request.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `dynamic_registration` | `Option<bool>` | No | Whether document symbol supports dynamic registration. Wire name: `dynamicRegistration`. |
/// | `hierarchical_document_symbol_support` | `Option<bool>` | No | The client supports hierarchical document symbols. Wire name: `hierarchicalDocumentSymbolSupport`. |
///
/// # LSP Specification
///
/// See [DocumentSymbolClientCapabilities](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#documentSymbolClientCapabilities).
///
/// **Note:** Partial representation. The full interface also includes `symbolKind`,
/// `tagSupport`, and `labelSupport`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbolClientCapabilities {
    /// Whether document symbol supports dynamic registration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,

    /// The client supports hierarchical document symbols.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hierarchical_document_symbol_support: Option<bool>,
}

/// Client capabilities for the `textDocument/implementation` request.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `dynamic_registration` | `Option<bool>` | No | Whether implementation supports dynamic registration. Wire name: `dynamicRegistration`. |
/// | `link_support` | `Option<bool>` | No | The client supports additional metadata in the form of definition links. Wire name: `linkSupport`. @since 3.14.0. |
///
/// # LSP Specification
///
/// See [ImplementationClientCapabilities](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#implementationClientCapabilities).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationClientCapabilities {
    /// Whether implementation supports dynamic registration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,

    /// The client supports additional metadata in the form of definition links.
    ///
    /// @since 3.14.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_support: Option<bool>,
}

/// Client capabilities for the `textDocument/prepareCallHierarchy` request.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `dynamic_registration` | `Option<bool>` | No | Whether call hierarchy supports dynamic registration. Wire name: `dynamicRegistration`. |
///
/// # LSP Specification
///
/// See [CallHierarchyClientCapabilities](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchyClientCapabilities).
///
/// @since 3.16.0
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyClientCapabilities {
    /// Whether call hierarchy supports dynamic registration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_registration: Option<bool>,
}

/// Parameters for the `textDocument/definition` request.
///
/// The definition request is sent from the client to the server to resolve the definition
/// location of a symbol at a given text document position.
///
/// # Wire Format
///
/// ```json
/// {
///   "textDocument": { "uri": "file:///path/to/file.rs" },
///   "position": { "line": 10, "character": 5 }
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `text_document_position` | [`TextDocumentPositionParams`] | Yes | The text document and position. Flattened on the wire. |
///
/// # LSP Specification
///
/// See [DefinitionParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#definitionParams).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionParams {
    #[serde(flatten)]
    pub text_document_position: TextDocumentPositionParams,
}

/// Parameters for the `textDocument/typeDefinition` request.
///
/// The type definition request is sent from the client to the server to resolve the type
/// definition location of a symbol at a given text document position.
///
/// # Wire Format
///
/// Same as [`DefinitionParams`].
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `text_document_position` | [`TextDocumentPositionParams`] | Yes | The text document and position. Flattened on the wire. |
///
/// # LSP Specification
///
/// See [TypeDefinitionParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#typeDefinitionParams).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TypeDefinitionParams {
    #[serde(flatten)]
    pub text_document_position: TextDocumentPositionParams,
}

/// Parameters for the `textDocument/references` request.
///
/// The references request is sent from the client to the server to resolve project-wide
/// references for the symbol denoted by the given text document position.
///
/// # Wire Format
///
/// ```json
/// {
///   "textDocument": { "uri": "file:///path/to/file.rs" },
///   "position": { "line": 10, "character": 5 },
///   "context": { "includeDeclaration": true }
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `text_document_position` | [`TextDocumentPositionParams`] | Yes | The text document and position. Flattened on the wire. |
/// | `context` | [`ReferenceContext`] | Yes | Additional context for the request. |
///
/// # LSP Specification
///
/// See [ReferenceParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#referenceParams).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReferencesParams {
    #[serde(flatten)]
    pub text_document_position: TextDocumentPositionParams,
    pub context: ReferenceContext,
}

/// Context for a `textDocument/references` request.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `include_declaration` | `bool` | Yes | Include the declaration of the current symbol. Wire name: `includeDeclaration`. |
///
/// # LSP Specification
///
/// See [ReferenceContext](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#referenceContext).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceContext {
    /// Include the declaration of the current symbol.
    pub include_declaration: bool,
}

/// Parameters for the `textDocument/documentSymbol` request.
///
/// The document symbol request is sent from the client to the server. The returned result
/// is either a flat list of `SymbolInformation` or a hierarchy of `DocumentSymbol`.
///
/// # Wire Format
///
/// ```json
/// { "textDocument": { "uri": "file:///path/to/file.rs" } }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `text_document` | [`TextDocumentIdentifier`] | Yes | The text document. Wire name: `textDocument`. |
///
/// # LSP Specification
///
/// See [DocumentSymbolParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#documentSymbolParams).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbolParams {
    /// The text document.
    pub text_document: TextDocumentIdentifier,
}

/// Parameters for the `textDocument/implementation` request.
///
/// The implementation request is sent from the client to the server to resolve the
/// implementation location of a symbol at a given text document position.
///
/// # Wire Format
///
/// Same as [`DefinitionParams`].
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `text_document_position` | [`TextDocumentPositionParams`] | Yes | The text document and position. Flattened on the wire. |
///
/// # LSP Specification
///
/// See [ImplementationParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#implementationParams).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationParams {
    #[serde(flatten)]
    pub text_document_position: TextDocumentPositionParams,
}

/// Parameters for the `textDocument/prepareCallHierarchy` request.
///
/// The call hierarchy request is sent from the client to the server to return a call hierarchy
/// for the language element of the given text document position. The call hierarchy requests
/// are executed in two steps:
///
/// 1. First a `textDocument/prepareCallHierarchy` is sent with this params type to obtain
///    a list of [`CallHierarchyItem`] items.
/// 2. For each item, `callHierarchy/incomingCalls` or `callHierarchy/outgoingCalls` is sent
///    to resolve the actual call graph.
///
/// # Wire Format
///
/// Same as [`DefinitionParams`].
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `text_document_position` | [`TextDocumentPositionParams`] | Yes | The text document and position. Flattened on the wire. |
///
/// # LSP Specification
///
/// See [CallHierarchyPrepareParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_prepareCallHierarchy).
///
/// @since 3.16.0
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyPrepareParams {
    #[serde(flatten)]
    pub text_document_position: TextDocumentPositionParams,
}

/// Parameters for the `callHierarchy/incomingCalls` request.
///
/// The request is sent from the client to the server to resolve incoming calls for
/// a given call hierarchy item.
///
/// # Wire Format
///
/// ```json
/// {
///   "item": {
///     "name": "foo",
///     "kind": 12,
///     "uri": "file:///path/to/file.rs",
///     "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 5, "character": 1 } },
///     "selectionRange": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 6 } }
///   }
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `item` | [`CallHierarchyItem`] | Yes | The item for which incoming calls are requested. |
///
/// # LSP Specification
///
/// See [CallHierarchyIncomingCallsParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_incomingCalls).
///
/// @since 3.16.0
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyIncomingCallsParams {
    /// The item for which incoming calls are requested.
    pub item: CallHierarchyItem,
}

/// Parameters for the `callHierarchy/outgoingCalls` request.
///
/// The request is sent from the client to the server to resolve outgoing calls for
/// a given call hierarchy item.
///
/// # Wire Format
///
/// ```json
/// {
///   "item": {
///     "name": "foo",
///     "kind": 12,
///     "uri": "file:///path/to/file.rs",
///     "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 5, "character": 1 } },
///     "selectionRange": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 6 } }
///   }
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `item` | [`CallHierarchyItem`] | Yes | The item for which outgoing calls are requested. |
///
/// # LSP Specification
///
/// See [CallHierarchyOutgoingCallsParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_outgoingCalls).
///
/// @since 3.16.0
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyOutgoingCallsParams {
    /// The item for which outgoing calls are requested.
    pub item: CallHierarchyItem,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_initialize_params_serialization() {
        let params = InitializeParams {
            process_id: Some(1234),
            client_info: None,
            locale: None,
            root_uri: Some("file:///test".to_string()),
            initialization_options: None,
            capabilities: ClientCapabilities::default(),
            trace: Some("off".to_string()),
            workspace_folders: None,
        };

        let serialized = serde_json::to_string(&params).unwrap();
        assert!(serialized.contains("\"processId\":1234"));
        assert!(serialized.contains("\"rootUri\":\"file:///test\""));
    }

    #[test]
    fn test_initialize_params_with_client_info() {
        let params = InitializeParams {
            process_id: Some(1234),
            client_info: Some(ClientInfo {
                name: "test-client".to_string(),
                version: Some("1.0.0".to_string()),
            }),
            locale: Some("en-US".to_string()),
            root_uri: Some("file:///test".to_string()),
            initialization_options: Some(json!({"setting": true})),
            capabilities: ClientCapabilities::default(),
            trace: Some("off".to_string()),
            workspace_folders: None,
        };

        let serialized = serde_json::to_string(&params).unwrap();
        let deserialized: InitializeParams = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized.client_info.as_ref().unwrap().name,
            "test-client"
        );
        assert_eq!(deserialized.locale.as_ref().unwrap(), "en-US");
        assert!(deserialized.initialization_options.is_some());
    }

    #[test]
    fn test_initialize_params_deserialization_with_unknown_fields() {
        let raw = r#"{
            "processId": 1234,
            "rootUri": "file:///test",
            "capabilities": {
                "workspace": { "workspaceFolders": true },
                "textDocument": {},
                "window": { "workDoneProgress": true }
            },
            "trace": "off"
        }"#;

        let params: InitializeParams = serde_json::from_str(raw).unwrap();
        assert_eq!(params.process_id, Some(1234));
        assert!(params.capabilities.other.contains_key("window"));
    }
}
