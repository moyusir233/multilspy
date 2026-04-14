//! LSP response result structures.
//!
//! This module defines the result types returned by the server in response to LSP requests,
//! including the `initialize` result, server capabilities, and various response type aliases.
//!
//! # Structures
//!
//! | Structure | LSP Spec Section |
//! |-----------|-----------------|
//! | [`InitializeResult`] | [InitializeResult](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initializeResult) |
//! | [`ServerCapabilities`] | [ServerCapabilities](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities) |
//! | [`ServerInfo`] | [InitializeResult.serverInfo](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initializeResult) |
//! | [`DefinitionProviderCapability`] | [definitionProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities) |
//! | [`TypeDefinitionProviderCapability`] | [typeDefinitionProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities) |
//! | [`ReferencesProviderCapability`] | [referencesProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities) |
//! | [`DocumentSymbolProviderCapability`] | [documentSymbolProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities) |
//! | [`ImplementationProviderCapability`] | [implementationProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities) |
//! | [`WorkspaceSymbolProviderCapability`] | [workspaceSymbolProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_symbol) |
//! | [`CallHierarchyProviderCapability`] | [callHierarchyProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities) |
//!
//! # Response Type Aliases
//!
//! | Type Alias | Return Type | LSP Method |
//! |------------|-------------|------------|
//! | [`DefinitionResponse`] | `Vec<Location>` | `textDocument/definition` |
//! | [`TypeDefinitionResponse`] | `Vec<Location>` | `textDocument/typeDefinition` |
//! | [`ReferencesResponse`] | `Vec<Location>` | `textDocument/references` |
//! | [`DocumentSymbolResponse`] | `Vec<DocumentSymbol>` | `textDocument/documentSymbol` |
//! | [`ImplementationResponse`] | `Vec<Location>` | `textDocument/implementation` |
//! | [`WorkspaceSymbolResponse`] | `Vec<WorkspaceSymbolItem>` | `workspace/symbol` |
//! | [`WorkspaceSymbolResolveResponse`] | `Option<WorkspaceSymbol>` | `workspaceSymbol/resolve` |
//! | [`CallHierarchyPrepareResponse`] | `Vec<CallHierarchyItem>` | `textDocument/prepareCallHierarchy` |
//! | [`CallHierarchyIncomingCallsResponse`] | `Vec<CallHierarchyIncomingCall>` | `callHierarchy/incomingCalls` |
//! | [`CallHierarchyOutgoingCallsResponse`] | `Vec<CallHierarchyOutgoingCall>` | `callHierarchy/outgoingCalls` |

use serde::{Deserialize, Serialize};
use super::common::*;

/// Result of the `initialize` request.
///
/// The server responds with this structure after receiving the `initialize` request.
/// It communicates the server's capabilities back to the client.
///
/// # Wire Format
///
/// ```json
/// {
///   "capabilities": {
///     "definitionProvider": true,
///     "referencesProvider": { "workDoneProgress": true }
///   },
///   "serverInfo": { "name": "rust-analyzer", "version": "1.0.0" }
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `capabilities` | [`ServerCapabilities`] | Yes | The capabilities the language server provides. |
/// | `server_info` | `Option<ServerInfo>` | No | Information about the server. @since 3.15.0. Wire name: `serverInfo`. |
///
/// # LSP Specification
///
/// See [InitializeResult](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initializeResult).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// The capabilities the language server provides.
    pub capabilities: ServerCapabilities,

    /// Information about the server.
    ///
    /// @since 3.15.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_info: Option<ServerInfo>,
}

/// Server capabilities communicated during the `initialize` handshake.
///
/// This is a partial implementation covering the capabilities for features implemented
/// in this crate. Unknown capabilities received from the server are preserved in the
/// `other` field via `#[serde(flatten)]`.
///
/// Each capability field can be either a simple boolean (`true`/`false`) or an options object
/// with additional configuration such as `workDoneProgress`.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `definition_provider` | `Option<DefinitionProviderCapability>` | No | The server provides go to definition support. Wire name: `definitionProvider`. |
/// | `type_definition_provider` | `Option<TypeDefinitionProviderCapability>` | No | The server provides go to type definition support. Wire name: `typeDefinitionProvider`. |
/// | `references_provider` | `Option<ReferencesProviderCapability>` | No | The server provides find references support. Wire name: `referencesProvider`. |
/// | `document_symbol_provider` | `Option<DocumentSymbolProviderCapability>` | No | The server provides document symbol support. Wire name: `documentSymbolProvider`. |
/// | `implementation_provider` | `Option<ImplementationProviderCapability>` | No | The server provides go to implementation support. Wire name: `implementationProvider`. |
/// | `workspace_symbol_provider` | `Option<WorkspaceSymbolProviderCapability>` | No | The server provides workspace symbol support. Wire name: `workspaceSymbolProvider`. |
/// | `call_hierarchy_provider` | `Option<CallHierarchyProviderCapability>` | No | The server provides call hierarchy support. Wire name: `callHierarchyProvider`. @since 3.16.0. |
/// | `other` | `Map<String, Value>` | — | Catch-all for additional capability fields not explicitly modeled. |
///
/// # LSP Specification
///
/// See [ServerCapabilities](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities).
///
/// **Note:** This is a partial representation. The full LSP `ServerCapabilities` interface
/// includes many more providers such as `completionProvider`, `hoverProvider`,
/// `signatureHelpProvider`, `codeActionProvider`, `renamingProvider`, etc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// The server provides go to definition support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition_provider: Option<DefinitionProviderCapability>,

    /// The server provides go to type definition support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_definition_provider: Option<TypeDefinitionProviderCapability>,

    /// The server provides find references support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references_provider: Option<ReferencesProviderCapability>,

    /// The server provides document symbol support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_symbol_provider: Option<DocumentSymbolProviderCapability>,

    /// The server provides go to implementation support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation_provider: Option<ImplementationProviderCapability>,

    /// The server provides workspace symbol support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_symbol_provider: Option<WorkspaceSymbolProviderCapability>,

    /// The server provides call hierarchy support.
    ///
    /// @since 3.16.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_hierarchy_provider: Option<CallHierarchyProviderCapability>,

    /// Catch-all for additional capability fields not explicitly modeled.
    #[serde(flatten)]
    pub other: serde_json::Map<String, serde_json::Value>,
}

impl ServerCapabilities {
    pub fn supports_workspace_symbol(&self) -> bool {
        self.workspace_symbol_provider
            .as_ref()
            .is_some_and(WorkspaceSymbolProviderCapability::is_supported)
    }

    pub fn supports_workspace_symbol_resolve(&self) -> bool {
        self.workspace_symbol_provider
            .as_ref()
            .is_some_and(WorkspaceSymbolProviderCapability::resolve_provider)
    }
}

/// Definition provider capability.
///
/// Per the LSP specification, the `definitionProvider` field in [`ServerCapabilities`] can be
/// either a simple boolean or a `DefinitionOptions` object.
///
/// # Wire Format
///
/// ```json
/// true
/// ```
/// or
/// ```json
/// { "workDoneProgress": true }
/// ```
///
/// # LSP Specification
///
/// See [ServerCapabilities.definitionProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum DefinitionProviderCapability {
    /// Simple boolean indicating support.
    Simple(bool),
    /// Options object with additional configuration.
    Options(DefinitionOptions),
}

/// Options for the definition provider.
///
/// Extends `WorkDoneProgressOptions`.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `work_done_progress` | `Option<bool>` | No | Whether work done progress is supported. Wire name: `workDoneProgress`. |
///
/// # LSP Specification
///
/// See [DefinitionOptions](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#definitionOptions).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

/// Type definition provider capability.
///
/// The `typeDefinitionProvider` field can be a boolean, `TypeDefinitionOptions`,
/// or `TypeDefinitionRegistrationOptions`. This implementation supports the boolean
/// and options variants.
///
/// # LSP Specification
///
/// See [ServerCapabilities.typeDefinitionProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum TypeDefinitionProviderCapability {
    Simple(bool),
    Options(TypeDefinitionOptions),
}

/// Options for the type definition provider.
///
/// Extends `WorkDoneProgressOptions`.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `work_done_progress` | `Option<bool>` | No | Whether work done progress is supported. Wire name: `workDoneProgress`. |
///
/// # LSP Specification
///
/// See [TypeDefinitionOptions](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#typeDefinitionOptions).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TypeDefinitionOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

/// References provider capability.
///
/// The `referencesProvider` field can be a boolean or a `ReferenceOptions` object.
///
/// # LSP Specification
///
/// See [ServerCapabilities.referencesProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ReferencesProviderCapability {
    Simple(bool),
    Options(ReferencesOptions),
}

/// Options for the references provider.
///
/// Extends `WorkDoneProgressOptions`.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `work_done_progress` | `Option<bool>` | No | Whether work done progress is supported. Wire name: `workDoneProgress`. |
///
/// # LSP Specification
///
/// See [ReferenceOptions](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#referenceOptions).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReferencesOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

/// Document symbol provider capability.
///
/// The `documentSymbolProvider` field can be a boolean or a `DocumentSymbolOptions` object.
///
/// # LSP Specification
///
/// See [ServerCapabilities.documentSymbolProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum DocumentSymbolProviderCapability {
    Simple(bool),
    Options(DocumentSymbolOptions),
}

/// Options for the document symbol provider.
///
/// Extends `WorkDoneProgressOptions`.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `work_done_progress` | `Option<bool>` | No | Whether work done progress is supported. Wire name: `workDoneProgress`. |
/// | `label` | `Option<String>` | No | A human-readable string that is shown when multiple outlines trees are shown for the same document. @since 3.16.0. |
///
/// # LSP Specification
///
/// See [DocumentSymbolOptions](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#documentSymbolOptions).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbolOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
    /// A human-readable string that is shown when multiple outlines trees
    /// are shown for the same document.
    ///
    /// @since 3.16.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Implementation provider capability.
///
/// The `implementationProvider` field can be a boolean, `ImplementationOptions`,
/// or `ImplementationRegistrationOptions`. This implementation supports the boolean
/// and options variants.
///
/// # LSP Specification
///
/// See [ServerCapabilities.implementationProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ImplementationProviderCapability {
    Simple(bool),
    Options(ImplementationOptions),
}

/// Options for the implementation provider.
///
/// Extends `WorkDoneProgressOptions`.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `work_done_progress` | `Option<bool>` | No | Whether work done progress is supported. Wire name: `workDoneProgress`. |
///
/// # LSP Specification
///
/// See [ImplementationOptions](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#implementationOptions).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

/// Workspace symbol provider capability.
///
/// The `workspaceSymbolProvider` field can be a boolean, `WorkspaceSymbolOptions`, or
/// `WorkspaceSymbolRegistrationOptions`. This implementation supports the boolean and options
/// variants used by the CLI and client.
///
/// # LSP Specification
///
/// See [ServerCapabilities.workspaceSymbolProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_symbol).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum WorkspaceSymbolProviderCapability {
    Simple(bool),
    Options(WorkspaceSymbolOptions),
}

impl WorkspaceSymbolProviderCapability {
    pub fn is_supported(&self) -> bool {
        match self {
            WorkspaceSymbolProviderCapability::Simple(supported) => *supported,
            WorkspaceSymbolProviderCapability::Options(_) => true,
        }
    }

    pub fn resolve_provider(&self) -> bool {
        match self {
            WorkspaceSymbolProviderCapability::Simple(_) => false,
            WorkspaceSymbolProviderCapability::Options(options) => {
                options.resolve_provider.unwrap_or(false)
            }
        }
    }
}

/// Options for the workspace symbol provider.
///
/// Extends `WorkDoneProgressOptions`.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `work_done_progress` | `Option<bool>` | No | Whether work done progress is supported. Wire name: `workDoneProgress`. |
/// | `resolve_provider` | `Option<bool>` | No | Whether the server supports `workspaceSymbol/resolve`. Wire name: `resolveProvider`. |
///
/// # LSP Specification
///
/// See [WorkspaceSymbolOptions](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspaceSymbolOptions).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSymbolOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_provider: Option<bool>,
}

/// Call hierarchy provider capability.
///
/// The `callHierarchyProvider` field can be a boolean, `CallHierarchyOptions`,
/// or `CallHierarchyRegistrationOptions`. This implementation supports the boolean
/// and options variants.
///
/// # LSP Specification
///
/// See [ServerCapabilities.callHierarchyProvider](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities).
///
/// @since 3.16.0
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum CallHierarchyProviderCapability {
    Simple(bool),
    Options(CallHierarchyOptions),
}

/// Options for the call hierarchy provider.
///
/// Extends `WorkDoneProgressOptions`.
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `work_done_progress` | `Option<bool>` | No | Whether work done progress is supported. Wire name: `workDoneProgress`. |
///
/// # LSP Specification
///
/// See [CallHierarchyOptions](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchyOptions).
///
/// @since 3.16.0
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

/// Information about the server.
///
/// Returned as part of the [`InitializeResult`].
///
/// # Wire Format
///
/// ```json
/// { "name": "rust-analyzer", "version": "2024-01-01" }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `name` | `String` | Yes | The name of the server as defined by the server. |
/// | `version` | `Option<String>` | No | The server's version as defined by the server. |
///
/// # LSP Specification
///
/// See [InitializeResult.serverInfo](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initializeResult).
///
/// @since 3.15.0
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    /// The name of the server as defined by the server.
    pub name: String,

    /// The server's version as defined by the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Response type for `textDocument/definition`.
///
/// The result is `Location[] | LocationLink[] | null` per the spec. This implementation
/// uses `Vec<Location>` (the non-link variant). An empty vector represents `null`.
///
/// See [textDocument/definition](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition).
pub type DefinitionResponse = Vec<Location>;

/// Response type for `textDocument/typeDefinition`.
///
/// The result is `Location[] | LocationLink[] | null` per the spec. This implementation
/// uses `Vec<Location>` (the non-link variant). An empty vector represents `null`.
///
/// See [textDocument/typeDefinition](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_typeDefinition).
pub type TypeDefinitionResponse = Vec<Location>;

/// Response type for `textDocument/references`.
///
/// The result is `Location[] | null` per the spec. An empty vector represents `null`.
///
/// See [textDocument/references](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_references).
pub type ReferencesResponse = Vec<Location>;

/// Response type for `textDocument/documentSymbol`.
///
/// The result is `DocumentSymbol[] | SymbolInformation[] | null` per the spec.
/// This implementation uses `Vec<DocumentSymbol>` (the hierarchical variant).
/// An empty vector represents `null`.
///
/// See [textDocument/documentSymbol](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentSymbol).
pub type DocumentSymbolResponse = Vec<DocumentSymbol>;

/// Response type for `textDocument/implementation`.
///
/// The result is `Location[] | LocationLink[] | null` per the spec. This implementation
/// uses `Vec<Location>` (the non-link variant). An empty vector represents `null`.
///
/// See [textDocument/implementation](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_implementation).
pub type ImplementationResponse = Vec<Location>;

/// Response type for `workspace/symbol`.
///
/// The result is `SymbolInformation[] | WorkspaceSymbol[] | null` per the spec. This
/// implementation preserves both item variants using [`WorkspaceSymbolItem`]. An empty vector
/// represents `null`.
///
/// See [workspace/symbol](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_symbol).
pub type WorkspaceSymbolResponse = Vec<WorkspaceSymbolItem>;

/// Response type for `workspaceSymbol/resolve`.
///
/// The result is `WorkspaceSymbol | null` per the spec.
///
/// See [workspaceSymbol/resolve](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_symbolResolve).
pub type WorkspaceSymbolResolveResponse = Option<WorkspaceSymbol>;

/// Response type for `textDocument/prepareCallHierarchy`.
///
/// The result is `CallHierarchyItem[] | null` per the spec. An empty vector represents `null`.
///
/// See [textDocument/prepareCallHierarchy](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_prepareCallHierarchy).
///
/// @since 3.16.0
pub type CallHierarchyPrepareResponse = Vec<CallHierarchyItem>;

/// Response type for `callHierarchy/incomingCalls`.
///
/// The result is `CallHierarchyIncomingCall[] | null` per the spec. An empty vector represents `null`.
///
/// See [callHierarchy/incomingCalls](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_incomingCalls).
///
/// @since 3.16.0
pub type CallHierarchyIncomingCallsResponse = Vec<CallHierarchyIncomingCall>;

/// Response type for `callHierarchy/outgoingCalls`.
///
/// The result is `CallHierarchyOutgoingCall[] | null` per the spec. An empty vector represents `null`.
///
/// See [callHierarchy/outgoingCalls](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_outgoingCalls).
///
/// @since 3.16.0
pub type CallHierarchyOutgoingCallsResponse = Vec<CallHierarchyOutgoingCall>;
