use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    /// Line position in a document (zero-based).
    pub line: u32,
    /// Character offset on a line in a document (zero-based).
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Range {
    /// The range's start position.
    pub start: Position,
    /// The range's end position.
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

impl Location {
    pub fn to_file_path(&self) -> Option<PathBuf> {
        self.uri.strip_prefix("file://").map(PathBuf::from)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextDocumentIdentifier {
    /// The text document's URI.
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextDocumentPositionParams {
    /// The text document.
    pub text_document: TextDocumentIdentifier,
    /// The position inside the text document.
    pub position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFolder {
    /// The associated URI for this workspace folder.
    pub uri: String,
    /// The name of the workspace folder. Used to refer to this
    /// workspace folder in the user interface.
    pub name: String,
}

/// Symbol kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct SymbolKind(pub i32);

#[allow(non_upper_case_globals)]
impl SymbolKind {
    pub const File: SymbolKind = SymbolKind(1);
    pub const Module: SymbolKind = SymbolKind(2);
    pub const Namespace: SymbolKind = SymbolKind(3);
    pub const Package: SymbolKind = SymbolKind(4);
    pub const Class: SymbolKind = SymbolKind(5);
    pub const Method: SymbolKind = SymbolKind(6);
    pub const Property: SymbolKind = SymbolKind(7);
    pub const Field: SymbolKind = SymbolKind(8);
    pub const Constructor: SymbolKind = SymbolKind(9);
    pub const Enum: SymbolKind = SymbolKind(10);
    pub const Interface: SymbolKind = SymbolKind(11);
    pub const Function: SymbolKind = SymbolKind(12);
    pub const Variable: SymbolKind = SymbolKind(13);
    pub const Constant: SymbolKind = SymbolKind(14);
    pub const String: SymbolKind = SymbolKind(15);
    pub const Number: SymbolKind = SymbolKind(16);
    pub const Boolean: SymbolKind = SymbolKind(17);
    pub const Array: SymbolKind = SymbolKind(18);
    pub const Object: SymbolKind = SymbolKind(19);
    pub const Key: SymbolKind = SymbolKind(20);
    pub const Null: SymbolKind = SymbolKind(21);
    pub const EnumMember: SymbolKind = SymbolKind(22);
    pub const Struct: SymbolKind = SymbolKind(23);
    pub const Event: SymbolKind = SymbolKind(24);
    pub const Operator: SymbolKind = SymbolKind(25);
    pub const TypeParameter: SymbolKind = SymbolKind(26);
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbol {
    /// The name of this symbol. Will be displayed in the user interface and
    /// therefore must not be an empty string or a string only consisting of
    /// white spaces.
    pub name: String,
    /// More detail for this symbol, e.g the signature of a function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The kind of this symbol.
    pub kind: SymbolKind,
    /// Tags for this symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<SymbolTag>>,
    /// Indicates if this symbol is deprecated.
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<DocumentSymbol>>,
}

/// Symbol tags are extra annotations that tweak the rendering of a symbol.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct SymbolTag(pub i32);

#[allow(non_upper_case_globals)]
impl SymbolTag {
    /// Render a symbol as obsolete, usually using a strike-out.
    pub const Deprecated: SymbolTag = SymbolTag(1);
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyIncomingCall {
    /// The item that makes the call.
    pub from: CallHierarchyItem,
    /// The ranges at which the calls appear. This is relative to the caller
    /// denoted by `from`.
    pub from_ranges: Vec<Range>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallHierarchyOutgoingCall {
    /// The item that is called.
    pub to: CallHierarchyItem,
    /// The ranges at which this item is called. This is relative to the
    /// caller from which the outgoing call was requested.
    pub from_ranges: Vec<Range>,
}
