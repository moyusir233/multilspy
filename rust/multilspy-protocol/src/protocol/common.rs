//! Common LSP structures shared across requests and responses.
//!
//! This module defines the foundational data types used throughout the Language Server Protocol,
//! as specified in the [LSP 3.17 Basic JSON Structures](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#basicJsonStructures).
//!
//! # Structures
//!
//! | Structure | LSP Spec Section |
//! |-----------|-----------------|
//! | [`Position`] | [Position](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#position) |
//! | [`Range`] | [Range](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#range) |
//! | [`Location`] | [Location](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#location) |
//! | [`TextDocumentIdentifier`] | [TextDocumentIdentifier](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocumentIdentifier) |
//! | [`TextDocumentPositionParams`] | [TextDocumentPositionParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocumentPositionParams) |
//! | [`WorkspaceFolder`] | [WorkspaceFolder](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspaceFolder) |
//! | [`SymbolKind`] | [SymbolKind](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#symbolKind) |
//! | [`SymbolTag`] | [SymbolTag](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#symbolTag) |
//! | [`DocumentSymbol`] | [DocumentSymbol](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#documentSymbol) |
//! | [`CallHierarchyItem`] | [CallHierarchyItem](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchyItem) |
//! | [`CallHierarchyIncomingCall`] | [CallHierarchyIncomingCall](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_incomingCalls) |
//! | [`CallHierarchyOutgoingCall`] | [CallHierarchyOutgoingCall](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_outgoingCalls) |

use fluent_uri::Uri;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::PathBuf;

/// Position in a text document expressed as zero-based line and zero-based character offset.
///
/// A position is between two characters like an 'insert' cursor in an editor. Special values
/// like for example `-1` to denote the end of a line are not supported.
///
/// # Wire Format
///
/// ```json
/// { "line": 5, "character": 23 }
/// ```
///
/// # Fields
///
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | `line` | `u32` | Line position in a document (zero-based). LSP type: `uinteger`. |
/// | `character` | `u32` | Character offset on a line in a document (zero-based). LSP type: `uinteger`. The meaning of this offset is determined by the negotiated `PositionEncodingKind` (defaults to UTF-16). |
///
/// # LSP Specification
///
/// See [Position](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#position).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    /// Line position in a document (zero-based).
    ///
    /// LSP type: `uinteger`
    pub line: u32,
    /// Character offset on a line in a document (zero-based).
    ///
    /// The meaning of this offset is determined by the negotiated
    /// `PositionEncodingKind` (defaults to UTF-16). If the character value
    /// is greater than the line length it defaults back to the line length.
    ///
    /// LSP type: `uinteger`
    pub character: u32,
}

/// A range in a text document expressed as (zero-based) start and end positions.
///
/// A range is comparable to a selection in an editor. Therefore, the end position is exclusive.
/// If you want to specify a range that contains a line including the line ending character(s)
/// then use an end position denoting the start of the next line.
///
/// # Wire Format
///
/// ```json
/// {
///   "start": { "line": 5, "character": 23 },
///   "end": { "line": 6, "character": 0 }
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | `start` | [`Position`] | The range's start position (inclusive). |
/// | `end` | [`Position`] | The range's end position (exclusive). |
///
/// # LSP Specification
///
/// See [Range](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#range).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Range {
    /// The range's start position.
    pub start: Position,
    /// The range's end position.
    pub end: Position,
}

/// Represents a location inside a resource, such as a line inside a text file.
///
/// # Wire Format
///
/// ```json
/// {
///   "uri": "file:///path/to/file.rs",
///   "range": {
///     "start": { "line": 0, "character": 0 },
///     "end": { "line": 0, "character": 10 }
///   }
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | `uri` | `String` | The URI of the document. LSP type: `DocumentUri`. |
/// | `range` | [`Range`] | The range within the document. |
///
/// # LSP Specification
///
/// See [Location](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#location).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    /// The URI of the document. LSP type: `DocumentUri`.
    pub uri: String,
    /// The range within the document.
    pub range: Range,
}

impl Location {
    pub fn to_file_path(&self) -> Option<PathBuf> {
        // TODO match python's `PathUtils.uri_to_path`
        let uri = Uri::parse(self.uri.clone()).ok()?;
        Some(PathBuf::from(uri.path().as_str()))
    }
}

/// Text documents are identified using a URI. On the protocol level, URIs are passed as strings.
///
/// # Wire Format
///
/// ```json
/// { "uri": "file:///path/to/file.rs" }
/// ```
///
/// # Fields
///
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | `uri` | `String` | The text document's URI. LSP type: `DocumentUri`. |
///
/// # LSP Specification
///
/// See [TextDocumentIdentifier](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocumentIdentifier).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextDocumentIdentifier {
    /// The text document's URI.
    pub uri: String,
}

/// A parameter literal used in requests to pass a text document and a position inside that document.
///
/// It is up to the client to decide how a selection is converted into a position when issuing
/// a request for a text document. The client can for example honor or ignore the selection
/// direction to make LSP requests consistent with features implemented internally.
///
/// # Wire Format
///
/// ```json
/// {
///   "textDocument": { "uri": "file:///path/to/file.rs" },
///   "position": { "line": 5, "character": 23 }
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | `text_document` | [`TextDocumentIdentifier`] | The text document. Wire name: `textDocument`. |
/// | `position` | [`Position`] | The position inside the text document. |
///
/// # LSP Specification
///
/// See [TextDocumentPositionParams](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocumentPositionParams).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentPositionParams {
    /// The text document.
    pub text_document: TextDocumentIdentifier,
    /// The position inside the text document.
    pub position: Position,
}

/// A workspace folder as configured in the client.
///
/// # Wire Format
///
/// ```json
/// {
///   "uri": "file:///path/to/workspace",
///   "name": "my-project"
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | `uri` | `String` | The associated URI for this workspace folder. LSP type: `URI`. |
/// | `name` | `String` | The name of the workspace folder. Used to refer to this workspace folder in the user interface. |
///
/// # LSP Specification
///
/// See [WorkspaceFolder](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspaceFolder).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFolder {
    /// The associated URI for this workspace folder.
    pub uri: String,
    /// The name of the workspace folder. Used to refer to this
    /// workspace folder in the user interface.
    pub name: String,
}

/// A symbol kind as defined in the LSP specification.
///
/// Symbol kinds are serialized as integers on the wire. The protocol defines 26 standard
/// symbol kinds numbered 1 through 26. Clients and servers should use capability negotiation
/// to determine the supported set of symbol kinds.
///
/// # Wire Format
///
/// Serialized as an integer value (e.g., `1` for `File`, `12` for `Function`).
///
/// # Variants and Values
///
/// | Variant | Value | Description |
/// |---------|-------|-------------|
/// | `File` | 1 | |
/// | `Module` | 2 | |
/// | `Namespace` | 3 | |
/// | `Package` | 4 | |
/// | `Class` | 5 | |
/// | `Method` | 6 | |
/// | `Property` | 7 | |
/// | `Field` | 8 | |
/// | `Constructor` | 9 | |
/// | `Enum` | 10 | |
/// | `Interface` | 11 | |
/// | `Function` | 12 | |
/// | `Variable` | 13 | |
/// | `Constant` | 14 | |
/// | `String` | 15 | |
/// | `Number` | 16 | |
/// | `Boolean` | 17 | |
/// | `Array` | 18 | |
/// | `Object` | 19 | |
/// | `Key` | 20 | |
/// | `Null` | 21 | |
/// | `EnumMember` | 22 | |
/// | `Struct` | 23 | |
/// | `Event` | 24 | |
/// | `Operator` | 25 | |
/// | `TypeParameter` | 26 | |
///
/// # LSP Specification
///
/// See [SymbolKind](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#symbolKind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

impl SymbolKind {
    pub fn value(&self) -> i32 {
        match self {
            SymbolKind::File => 1,
            SymbolKind::Module => 2,
            SymbolKind::Namespace => 3,
            SymbolKind::Package => 4,
            SymbolKind::Class => 5,
            SymbolKind::Method => 6,
            SymbolKind::Property => 7,
            SymbolKind::Field => 8,
            SymbolKind::Constructor => 9,
            SymbolKind::Enum => 10,
            SymbolKind::Interface => 11,
            SymbolKind::Function => 12,
            SymbolKind::Variable => 13,
            SymbolKind::Constant => 14,
            SymbolKind::String => 15,
            SymbolKind::Number => 16,
            SymbolKind::Boolean => 17,
            SymbolKind::Array => 18,
            SymbolKind::Object => 19,
            SymbolKind::Key => 20,
            SymbolKind::Null => 21,
            SymbolKind::EnumMember => 22,
            SymbolKind::Struct => 23,
            SymbolKind::Event => 24,
            SymbolKind::Operator => 25,
            SymbolKind::TypeParameter => 26,
        }
    }

    pub fn from_value(value: i32) -> Option<SymbolKind> {
        match value {
            1 => Some(SymbolKind::File),
            2 => Some(SymbolKind::Module),
            3 => Some(SymbolKind::Namespace),
            4 => Some(SymbolKind::Package),
            5 => Some(SymbolKind::Class),
            6 => Some(SymbolKind::Method),
            7 => Some(SymbolKind::Property),
            8 => Some(SymbolKind::Field),
            9 => Some(SymbolKind::Constructor),
            10 => Some(SymbolKind::Enum),
            11 => Some(SymbolKind::Interface),
            12 => Some(SymbolKind::Function),
            13 => Some(SymbolKind::Variable),
            14 => Some(SymbolKind::Constant),
            15 => Some(SymbolKind::String),
            16 => Some(SymbolKind::Number),
            17 => Some(SymbolKind::Boolean),
            18 => Some(SymbolKind::Array),
            19 => Some(SymbolKind::Object),
            20 => Some(SymbolKind::Key),
            21 => Some(SymbolKind::Null),
            22 => Some(SymbolKind::EnumMember),
            23 => Some(SymbolKind::Struct),
            24 => Some(SymbolKind::Event),
            25 => Some(SymbolKind::Operator),
            26 => Some(SymbolKind::TypeParameter),
            _ => None,
        }
    }
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

impl Serialize for SymbolKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(self.value())
    }
}

impl<'de> Deserialize<'de> for SymbolKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SymbolKindVisitor;

        impl<'de> Visitor<'de> for SymbolKindVisitor {
            type Value = SymbolKind;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an integer between 1 and 26")
            }

            fn visit_i64<E>(self, value: i64) -> Result<SymbolKind, E>
            where
                E: de::Error,
            {
                SymbolKind::from_value(value as i32)
                    .ok_or_else(|| de::Error::custom(format!("unknown SymbolKind: {}", value)))
            }

            fn visit_u64<E>(self, value: u64) -> Result<SymbolKind, E>
            where
                E: de::Error,
            {
                SymbolKind::from_value(value as i32)
                    .ok_or_else(|| de::Error::custom(format!("unknown SymbolKind: {}", value)))
            }
        }

        deserializer.deserialize_i32(SymbolKindVisitor)
    }
}

/// Represents programming constructs like variables, classes, interfaces etc. that appear in
/// a document.
///
/// Document symbols can be hierarchical and they have two ranges: one that encloses its
/// definition and one that points to its most interesting range, e.g. the range of an identifier.
///
/// # Wire Format
///
/// ```json
/// {
///   "name": "MyClass",
///   "detail": "class MyClass",
///   "kind": 5,
///   "tags": [1],
///   "deprecated": false,
///   "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 10, "character": 1 } },
///   "selectionRange": { "start": { "line": 0, "character": 6 }, "end": { "line": 0, "character": 13 } },
///   "children": []
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `name` | `String` | Yes | The name of this symbol. Must not be empty or whitespace-only. |
/// | `detail` | `Option<String>` | No | More detail for this symbol, e.g. the signature of a function. |
/// | `kind` | [`SymbolKind`] | Yes | The kind of this symbol. |
/// | `tags` | `Option<Vec<SymbolTag>>` | No | Tags for this symbol. |
/// | `deprecated` | `Option<bool>` | No | Indicates if this symbol is deprecated. **Note:** Use `tags` with `SymbolTag::Deprecated` instead. |
/// | `range` | [`Range`] | Yes | The range enclosing this symbol not including leading/trailing whitespace but everything else like comments. |
/// | `selection_range` | [`Range`] | Yes | The range that should be selected and revealed when this symbol is picked. Must be contained by `range`. Wire name: `selectionRange`. |
/// | `children` | `Option<Vec<DocumentSymbol>>` | No | Children of this symbol, e.g. properties of a class. |
///
/// # LSP Specification
///
/// See [DocumentSymbol](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#documentSymbol).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbol {
    /// The name of this symbol. Will be displayed in the user interface and
    /// therefore must not be an empty string or a string only consisting of
    /// white spaces.
    pub name: String,
    /// More detail for this symbol, e.g the signature of a function.
    #[serde(default)]
    pub detail: Option<String>,
    /// The kind of this symbol.
    pub kind: SymbolKind,
    /// Tags for this symbol.
    #[serde(default)]
    pub tags: Option<Vec<SymbolTag>>,
    /// Indicates if this symbol is deprecated.
    ///
    /// **Deprecated:** Use `tags` with [`SymbolTag::Deprecated`] instead.
    #[serde(default)]
    pub deprecated: Option<bool>,
    /// The range enclosing this symbol not including leading/trailing whitespace
    /// but everything else like comments. This information is typically used to
    /// determine if the clients cursor is inside the symbol to reveal in the
    /// symbol in the UI.
    pub range: Range,
    /// The range that should be selected and revealed when this symbol is being
    /// picked, e.g. the name of a function. Must be contained by the `range`.
    pub selection_range: Range,
    /// Children of this symbol, e.g. properties of a class.
    #[serde(default)]
    pub children: Option<Vec<DocumentSymbol>>,
}

/// Symbol tags are extra annotations that tweak the rendering of a symbol.
///
/// Tags are serialized as integers on the wire. Currently the specification defines
/// only one tag value.
///
/// # Wire Format
///
/// Serialized as an integer value (e.g., `1` for `Deprecated`).
///
/// # Variants and Values
///
/// | Variant | Value | Description |
/// |---------|-------|-------------|
/// | `Deprecated` | 1 | Render a symbol as obsolete, usually using a strike-out. |
///
/// # LSP Specification
///
/// See [SymbolTag](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#symbolTag).
///
/// @since 3.16.0
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolTag {
    /// Render a symbol as obsolete, usually using a strike-out.
    Deprecated,
}

impl SymbolTag {
    pub fn value(&self) -> i32 {
        match self {
            SymbolTag::Deprecated => 1,
        }
    }

    pub fn from_value(value: i32) -> Option<SymbolTag> {
        match value {
            1 => Some(SymbolTag::Deprecated),
            _ => None,
        }
    }
}

impl fmt::Display for SymbolTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

impl Serialize for SymbolTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(self.value())
    }
}

impl<'de> Deserialize<'de> for SymbolTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SymbolTagVisitor;

        impl<'de> Visitor<'de> for SymbolTagVisitor {
            type Value = SymbolTag;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an integer 1 (for Deprecated)")
            }

            fn visit_i64<E>(self, value: i64) -> Result<SymbolTag, E>
            where
                E: de::Error,
            {
                SymbolTag::from_value(value as i32)
                    .ok_or_else(|| de::Error::custom(format!("unknown SymbolTag: {}", value)))
            }

            fn visit_u64<E>(self, value: u64) -> Result<SymbolTag, E>
            where
                E: de::Error,
            {
                SymbolTag::from_value(value as i32)
                    .ok_or_else(|| de::Error::custom(format!("unknown SymbolTag: {}", value)))
            }
        }

        deserializer.deserialize_i32(SymbolTagVisitor)
    }
}

/// Represents an item in a call hierarchy.
///
/// The result of a `textDocument/prepareCallHierarchy` request. A `CallHierarchyItem` is then
/// used as input to resolve incoming and outgoing calls.
///
/// # Wire Format
///
/// ```json
/// {
///   "name": "foo",
///   "kind": 12,
///   "tags": [1],
///   "detail": "fn foo()",
///   "uri": "file:///path/to/file.rs",
///   "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 5, "character": 1 } },
///   "selectionRange": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 6 } },
///   "data": null
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `name` | `String` | Yes | The name of this item. |
/// | `kind` | [`SymbolKind`] | Yes | The kind of this item. |
/// | `tags` | `Option<Vec<SymbolTag>>` | No | Tags for this item. |
/// | `detail` | `Option<String>` | No | More detail for this item, e.g. the signature of a function. |
/// | `uri` | `String` | Yes | The resource identifier of this item. LSP type: `DocumentUri`. |
/// | `range` | [`Range`] | Yes | The range enclosing this symbol not including leading/trailing whitespace. |
/// | `selection_range` | [`Range`] | Yes | The range that should be selected and revealed. Must be contained by `range`. Wire name: `selectionRange`. |
/// | `data` | `Option<Value>` | No | A data entry field preserved between a prepare and incoming/outgoing calls request. LSP type: `LSPAny`. |
///
/// # LSP Specification
///
/// See [CallHierarchyItem](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchyItem).
///
/// @since 3.16.0
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyItem {
    /// The name of this item.
    pub name: String,
    /// The kind of this item.
    pub kind: SymbolKind,
    /// Tags for this item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<SymbolTag>>,
    /// More detail for this item, e.g. the signature of a function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The resource identifier of this item.
    pub uri: String,
    /// The range enclosing this symbol not including leading/trailing whitespace
    /// but everything else like comments. This information is typically used to
    /// determine if the clients cursor is inside the symbol to reveal in the
    /// symbol in the UI.
    pub range: Range,
    /// The range that should be selected and revealed when this symbol is being
    /// picked, e.g. the name of a function. Must be contained by the `range`.
    pub selection_range: Range,
    /// A data entry field that is preserved on a call hierarchy item between
    /// a prepare and an incoming or outgoing calls request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Represents an incoming call to a [`CallHierarchyItem`].
///
/// Returned as part of the `callHierarchy/incomingCalls` response.
///
/// # Wire Format
///
/// ```json
/// {
///   "from": { "name": "bar", "kind": 12, "uri": "file:///...", "range": {...}, "selectionRange": {...} },
///   "fromRanges": [{ "start": { "line": 3, "character": 4 }, "end": { "line": 3, "character": 7 } }]
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `from` | [`CallHierarchyItem`] | Yes | The item that makes the call. |
/// | `from_ranges` | `Vec<Range>` | Yes | The ranges at which the calls appear, relative to the caller denoted by `from`. Wire name: `fromRanges`. |
///
/// # LSP Specification
///
/// See [CallHierarchyIncomingCall](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_incomingCalls).
///
/// @since 3.16.0
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyIncomingCall {
    /// The item that makes the call.
    pub from: CallHierarchyItem,
    /// The ranges at which the calls appear. This is relative to the caller
    /// denoted by `from`.
    pub from_ranges: Vec<Range>,
}

/// Represents an outgoing call from a [`CallHierarchyItem`].
///
/// Returned as part of the `callHierarchy/outgoingCalls` response.
///
/// # Wire Format
///
/// ```json
/// {
///   "to": { "name": "baz", "kind": 12, "uri": "file:///...", "range": {...}, "selectionRange": {...} },
///   "fromRanges": [{ "start": { "line": 7, "character": 4 }, "end": { "line": 7, "character": 7 } }]
/// }
/// ```
///
/// # Fields
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `to` | [`CallHierarchyItem`] | Yes | The item that is called. |
/// | `from_ranges` | `Vec<Range>` | Yes | The ranges at which this item is called, relative to the caller from which the outgoing call was requested. Wire name: `fromRanges`. |
///
/// # LSP Specification
///
/// See [CallHierarchyOutgoingCall](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#callHierarchy_outgoingCalls).
///
/// @since 3.16.0
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyOutgoingCall {
    /// The item that is called.
    pub to: CallHierarchyItem,
    /// The ranges at which this item is called. This is relative to the
    /// caller from which the outgoing call was requested.
    pub from_ranges: Vec<Range>,
}
