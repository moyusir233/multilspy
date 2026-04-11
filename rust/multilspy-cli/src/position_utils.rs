//! Position conversion utilities between raw file positions (1-based) and LSP positions (0-based).
//!
//! According to LSP specification:
//! - LSP uses 0-based line numbers and 0-based UTF-16 character offsets
//! - Raw file positions use 1-based line numbers and 1-based column numbers

use multilspy_protocol::protocol::common::{Position, Range, Location, DocumentSymbol, CallHierarchyItem, CallHierarchyIncomingCall, CallHierarchyOutgoingCall};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Convert a raw file position (1-based) to an LSP position (0-based).
pub fn raw_to_lsp_position(raw_line: u32, raw_column: u32) -> Position {
    Position {
        line: raw_line - 1,
        character: raw_column - 1,
    }
}

/// Convert an LSP position (0-based) to a raw file position (1-based).
pub fn lsp_to_raw_position(lsp_line: u32, lsp_character: u32) -> (u32, u32) {
    (lsp_line + 1, lsp_character + 1)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPosition {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawRange {
    pub start: RawPosition,
    pub end: RawPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLocation {
    pub uri: String,
    pub range: RawRange,
}

/// Convert an LSP Range's positions from 0-based to 1-based.
pub fn convert_lsp_range_to_raw(range: &Range) -> RawRange {
    let (start_line, start_column) = lsp_to_raw_position(range.start.line, range.start.character);
    let (end_line, end_column) = lsp_to_raw_position(range.end.line, range.end.character);

    RawRange {
        start: RawPosition { line: start_line, column: start_column },
        end: RawPosition { line: end_line, column: end_column },
    }
}

/// Convert an LSP Location's positions from 0-based to 1-based.
pub fn convert_lsp_location_to_raw(location: &Location) -> RawLocation {
    RawLocation {
        uri: location.uri.clone(),
        range: convert_lsp_range_to_raw(&location.range),
    }
}

/// Convert a list of LSP Locations to use 1-based positions.
pub fn convert_all_locations_to_raw(locations: &[Location]) -> Vec<RawLocation> {
    locations.iter().map(convert_lsp_location_to_raw).collect()
}

/// Convert a CallHierarchyItem's positions from 0-based to 1-based.
pub fn convert_call_hierarchy_item_to_raw(item: &CallHierarchyItem) -> CallHierarchyItemRaw {
    CallHierarchyItemRaw {
        name: item.name.clone(),
        kind: item.kind,
        tags: item.tags.clone(),
        detail: item.detail.clone(),
        uri: item.uri.clone(),
        range: convert_lsp_range_to_raw(&item.range),
        selection_range: convert_lsp_range_to_raw(&item.selection_range),
        data: item.data.clone(),
    }
}

/// Convert incoming calls (CallHierarchyIncomingCall) to use 1-based positions.
pub fn convert_incoming_calls_to_raw(calls: &[CallHierarchyIncomingCall]) -> Vec<CallHierarchyIncomingCallRaw> {
    calls.iter().map(|call| CallHierarchyIncomingCallRaw {
        from: convert_call_hierarchy_item_to_raw(&call.from),
        from_ranges: call.from_ranges.iter().map(convert_lsp_range_to_raw).collect(),
    }).collect()
}

/// Convert outgoing calls (CallHierarchyOutgoingCall) to use 1-based positions.
pub fn convert_outgoing_calls_to_raw(calls: &[CallHierarchyOutgoingCall]) -> Vec<CallHierarchyOutgoingCallRaw> {
    calls.iter().map(|call| CallHierarchyOutgoingCallRaw {
        to: convert_call_hierarchy_item_to_raw(&call.to),
        from_ranges: call.from_ranges.iter().map(convert_lsp_range_to_raw).collect(),
    }).collect()
}

/// Convert document symbols to use 1-based positions.
pub fn convert_document_symbols_to_raw(symbols: &[DocumentSymbol]) -> Vec<DocumentSymbolRaw> {
    symbols.iter().map(|symbol| DocumentSymbolRaw {
        name: symbol.name.clone(),
        detail: symbol.detail.clone(),
        kind: symbol.kind,
        tags: symbol.tags.clone(),
        deprecated: symbol.deprecated,
        range: convert_lsp_range_to_raw(&symbol.range),
        selection_range: convert_lsp_range_to_raw(&symbol.selection_range),
        children: symbol.children.as_ref().map(|children| convert_document_symbols_to_raw(children)),
    }).collect()
}

/// Get the unique key for a CallHierarchyItem, composed of name, uri, and range.
pub fn get_call_hierarchy_key(item: &CallHierarchyItem) -> String {
    let range_str = format!(
        "{},{} - {},{}",
        item.range.start.line,
        item.range.start.character,
        item.range.end.line,
        item.range.end.character
    );
    format!("{}|{}|{}", item.name, item.uri, range_str)
}

/// Extract information from a CallHierarchyItem (excluding name and uri which are used as key).
pub fn extract_call_hierarchy_item_info(item: &CallHierarchyItem) -> CallHierarchyItemInfo {
    CallHierarchyItemInfo {
        kind: item.kind,
        tags: item.tags.clone(),
        detail: item.detail.clone(),
        range: convert_lsp_range_to_raw(&item.range),
        selection_range: convert_lsp_range_to_raw(&item.selection_range),
        data: item.data.clone(),
    }
}

/// Raw (1-based) version of CallHierarchyItem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyItemRaw {
    pub name: String,
    pub kind: multilspy_protocol::protocol::common::SymbolKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<multilspy_protocol::protocol::common::SymbolTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub uri: String,
    pub range: RawRange,
    pub selection_range: RawRange,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Raw (1-based) version of CallHierarchyIncomingCall
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyIncomingCallRaw {
    pub from: CallHierarchyItemRaw,
    pub from_ranges: Vec<RawRange>,
}

/// Raw (1-based) version of CallHierarchyOutgoingCall
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyOutgoingCallRaw {
    pub to: CallHierarchyItemRaw,
    pub from_ranges: Vec<RawRange>,
}

/// Raw (1-based) version of DocumentSymbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSymbolRaw {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub kind: multilspy_protocol::protocol::common::SymbolKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<multilspy_protocol::protocol::common::SymbolTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    pub range: RawRange,
    pub selection_range: RawRange,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<DocumentSymbolRaw>>,
}

/// Extracted info from CallHierarchyItem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyItemInfo {
    pub kind: multilspy_protocol::protocol::common::SymbolKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<multilspy_protocol::protocol::common::SymbolTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub range: RawRange,
    pub selection_range: RawRange,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Recursive incoming calls result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursiveIncomingCallEntry {
    pub info: CallHierarchyItemInfo,
    pub incoming_calls: Vec<RecursiveIncomingCallRef>,
}

/// Reference to a recursive incoming call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursiveIncomingCallRef {
    pub key: String,
    pub from_ranges: Vec<RawRange>,
}

/// Recursive outgoing calls result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursiveOutgoingCallEntry {
    pub info: CallHierarchyItemInfo,
    pub outgoing_calls: Vec<RecursiveOutgoingCallRef>,
}

/// Reference to a recursive outgoing call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursiveOutgoingCallRef {
    pub key: String,
    pub from_ranges: Vec<RawRange>,
}

/// Recursive incoming calls result map
pub type RecursiveIncomingCallsResult = HashMap<String, RecursiveIncomingCallEntry>;

/// Recursive outgoing calls result map
pub type RecursiveOutgoingCallsResult = HashMap<String, RecursiveOutgoingCallEntry>;