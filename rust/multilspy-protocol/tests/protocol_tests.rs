use multilspy_protocol::protocol::common::*;
use multilspy_protocol::protocol::requests::*;
use multilspy_protocol::protocol::responses::*;
use serde_json::json;

#[test]
fn test_position_serialization() {
    let pos = Position {
        line: 42,
        character: 10,
    };
    let serialized = serde_json::to_string(&pos).unwrap();
    let deserialized: Position = serde_json::from_str(&serialized).unwrap();
    assert_eq!(pos, deserialized);
}

#[test]
fn test_range_serialization() {
    let range = Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 10,
            character: 5,
        },
    };
    let serialized = serde_json::to_string(&range).unwrap();
    let deserialized: Range = serde_json::from_str(&serialized).unwrap();
    assert_eq!(range, deserialized);
}

#[test]
fn test_location_serialization() {
    let location = Location {
        uri: "file:///test.rs".to_string(),
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 10,
            },
        },
    };
    let serialized = serde_json::to_string(&location).unwrap();
    let deserialized: Location = serde_json::from_str(&serialized).unwrap();
    assert_eq!(location, deserialized);
}

#[test]
fn test_definition_params_serialization() {
    let params = DefinitionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.rs".to_string(),
            },
            position: Position {
                line: 5,
                character: 3,
            },
        },
    };
    let serialized = serde_json::to_string(&params).unwrap();
    let deserialized: DefinitionParams = serde_json::from_str(&serialized).unwrap();
    assert_eq!(params, deserialized);
}

#[test]
fn test_definition_response_serialization() {
    let response: DefinitionResponse = vec![Location {
        uri: "file:///test.rs".to_string(),
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 10,
            },
        },
    }];
    let serialized = serde_json::to_string(&response).unwrap();
    let deserialized: DefinitionResponse = serde_json::from_str(&serialized).unwrap();
    assert_eq!(response, deserialized);
}

#[test]
fn test_call_hierarchy_item_serialization() {
    let item = CallHierarchyItem {
        name: "test_fn".to_string(),
        kind: SymbolKind::Function,
        tags: None,
        detail: Some("fn test_fn()".to_string()),
        uri: "file:///test.rs".to_string(),
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 5,
                character: 1,
            },
        },
        selection_range: Range {
            start: Position {
                line: 0,
                character: 3,
            },
            end: Position {
                line: 0,
                character: 10,
            },
        },
        data: None,
    };
    let serialized = serde_json::to_string(&item).unwrap();
    let deserialized: CallHierarchyItem = serde_json::from_str(&serialized).unwrap();
    assert_eq!(item, deserialized);
}

#[test]
fn test_workspace_symbol_params_serialization() {
    let params = WorkspaceSymbolParams {
        query: "helper".to_string(),
    };
    let serialized = serde_json::to_string(&params).unwrap();
    let deserialized: WorkspaceSymbolParams = serde_json::from_str(&serialized).unwrap();
    assert_eq!(params, deserialized);
}

#[test]
fn test_workspace_symbol_item_deserializes_workspace_symbol_with_uri_only_location() {
    let raw = json!({
        "name": "helper",
        "kind": 12,
        "location": { "uri": "file:///workspace/src/main.rs" },
        "containerName": "main",
        "data": { "id": 1 }
    });
    let item: WorkspaceSymbolItem = serde_json::from_value(raw).unwrap();

    match item {
        WorkspaceSymbolItem::WorkspaceSymbol(symbol) => {
            assert_eq!(symbol.name, "helper");
            assert!(matches!(
                symbol.location,
                WorkspaceSymbolLocation::UriOnly(WorkspaceSymbolUriLocation { .. })
            ));
        }
        other => panic!("expected WorkspaceSymbol variant, got {:?}", other),
    }
}

#[test]
fn test_workspace_symbol_item_deserializes_symbol_information() {
    let raw = json!({
        "name": "helper",
        "kind": 12,
        "location": {
            "uri": "file:///workspace/src/main.rs",
            "range": {
                "start": { "line": 34, "character": 0 },
                "end": { "line": 37, "character": 1 }
            }
        },
        "containerName": "main"
    });
    let item: WorkspaceSymbolItem = serde_json::from_value(raw).unwrap();

    match item {
        WorkspaceSymbolItem::SymbolInformation(symbol) => {
            assert_eq!(symbol.location.range.start.line, 34);
        }
        other => panic!("expected SymbolInformation variant, got {:?}", other),
    }
}

#[test]
fn test_workspace_symbol_provider_capability_resolve_support() {
    let capabilities: ServerCapabilities = serde_json::from_value(json!({
        "workspaceSymbolProvider": {
            "workDoneProgress": true,
            "resolveProvider": true
        }
    }))
    .unwrap();

    assert!(capabilities.supports_workspace_symbol());
    assert!(capabilities.supports_workspace_symbol_resolve());
}
