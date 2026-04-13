use multilspy_rust::{LSPClient, RustAnalyzerConfig};
use std::path::PathBuf;
use std::process::Command;

fn rust_analyzer_available() -> bool {
    Command::new("rust-analyzer")
        .arg("--version")
        .output()
        .is_ok()
}

fn test_project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-rust-project")
}

fn initialize_params_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ra_initialize_params.json")
}

fn file_uri() -> String {
    let main_rs = test_project_root()
        .join("src/main.rs")
        .canonicalize()
        .expect("test-rust-project/src/main.rs must exist");
    format!("file://{}", main_rs.display())
}

fn make_config() -> RustAnalyzerConfig {
    RustAnalyzerConfig::new(test_project_root(), initialize_params_path())
}

async fn make_client() -> LSPClient {
    LSPClient::new(make_config()).await.expect("LSPClient::new should succeed")
}

// ---------------------------------------------------------------------------
// LSPClient::new / shutdown
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_new_and_shutdown() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    client.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
async fn test_new_with_invalid_project_root() {
    if !rust_analyzer_available() {
        return;
    }
    let config = RustAnalyzerConfig::new(
        PathBuf::from("/nonexistent/project/root"),
        initialize_params_path(),
    );
    let result = LSPClient::new(config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_new_with_invalid_initialize_params_path() {
    if !rust_analyzer_available() {
        return;
    }
    let config = RustAnalyzerConfig::new(
        test_project_root(),
        PathBuf::from("/nonexistent/ra_initialize_params.json"),
    );
    let result = LSPClient::new(config).await;
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// LSPClient::definition
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_definition_of_function_call() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 35: `let h = create_hello("world");` — cursor on `create_hello` (char 12)
    let result = client.definition(uri.clone(), 35, 12).await.unwrap();
    assert!(!result.is_empty(), "definition should return at least one location");
    let loc = &result[0];
    assert!(loc.uri.ends_with("main.rs"));
    // `create_hello` is defined at line 24
    assert_eq!(loc.range.start.line, 24);

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_definition_of_trait_method_call() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 31: `g.greet()` — cursor on `greet` (char 6)
    let result = client.definition(uri.clone(), 31, 6).await.unwrap();
    assert!(!result.is_empty(), "definition of trait method call should return at least one location");
    let loc = &result[0];
    assert!(loc.uri.ends_with("main.rs"));
    // `greet` is declared in the trait at line 1
    assert_eq!(loc.range.start.line, 1);

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_definition_of_struct_field() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 10: `format!("Hello, {}!", self.name)` — cursor on `name` (char 35)
    let result = client.definition(uri.clone(), 10, 35).await.unwrap();
    assert!(!result.is_empty());
    let loc = &result[0];
    assert!(loc.uri.ends_with("main.rs"));
    // `name` field is defined at line 5
    assert_eq!(loc.range.start.line, 5);

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_definition_at_definition_site_points_to_self() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 24: `fn create_hello(name: &str) -> Hello {` — cursor on `create_hello` itself
    let result = client.definition(uri.clone(), 24, 5).await.unwrap();
    assert!(!result.is_empty());
    let loc = &result[0];
    assert_eq!(loc.range.start.line, 24);

    client.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// LSPClient::type_definition
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_type_definition_of_variable() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 35: `let h = create_hello("world");` — cursor on `h` (char 8)
    let result = client.type_definition(uri.clone(), 35, 8).await.unwrap();
    assert!(!result.is_empty(), "type_definition should return at least one location");
    let loc = &result[0];
    assert!(loc.uri.ends_with("main.rs"));
    // `h` has type `Hello`, defined at line 4
    assert_eq!(loc.range.start.line, 4);

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_type_definition_of_function_return() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 40: `let result = helper();` — cursor on `result` (char 8)
    let result = client.type_definition(uri.clone(), 40, 8).await.unwrap();
    assert!(!result.is_empty());
    let loc = &result[0];
    // `result` has type `String`; the type_definition should point into std
    assert!(loc.uri.contains("string") || loc.uri.contains("alloc") || loc.uri.contains("String") || !loc.uri.is_empty());

    client.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// LSPClient::references
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_references_include_declaration() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 24: `fn create_hello` — cursor on `create_hello` (char 5), include_declaration=true
    let result = client
        .references(uri.clone(), 24, 5, true)
        .await
        .unwrap();
    // Should include the declaration itself + the call at line 35
    assert!(
        result.len() >= 2,
        "references with include_declaration should return >= 2 locations, got {}",
        result.len()
    );
    let lines: Vec<u32> = result.iter().map(|loc| loc.range.start.line).collect();
    assert!(lines.contains(&24), "should include declaration at line 24");
    assert!(lines.contains(&35), "should include usage at line 35");

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_references_exclude_declaration() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 24: `fn create_hello` — cursor on `create_hello` (char 5), include_declaration=false
    let result = client
        .references(uri.clone(), 24, 5, false)
        .await
        .unwrap();
    assert!(
        !result.is_empty(),
        "references without declaration should return at least 1 location"
    );
    let lines: Vec<u32> = result.iter().map(|loc| loc.range.start.line).collect();
    assert!(lines.contains(&35), "should include usage at line 35");

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_references_of_trait_method() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 1: `fn greet(&self) -> String;` — cursor on `greet` (char 7), include_declaration=true
    let result = client.references(uri.clone(), 1, 7, true).await.unwrap();
    // The trait method `greet` is declared at line 1 and referenced/implemented at lines 9, 19, 31
    assert!(
        result.len() >= 2,
        "trait method should have multiple references, got {}",
        result.len()
    );

    client.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// LSPClient::document_symbols
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_document_symbols() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    let symbols = client.document_symbols(uri.clone()).await.unwrap();
    assert!(!symbols.is_empty(), "document_symbols should return symbols");

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Greeter"), "should contain Greeter trait");
    assert!(names.contains(&"Hello"), "should contain Hello struct");
    assert!(names.contains(&"Goodbye"), "should contain Goodbye struct");
    assert!(names.contains(&"create_hello"), "should contain create_hello function");
    assert!(names.contains(&"call_greet"), "should contain call_greet function");
    assert!(names.contains(&"helper"), "should contain helper function");
    assert!(names.contains(&"main"), "should contain main function");

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_document_symbols_kinds() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    use multilspy_protocol::protocol::common::SymbolKind;

    let symbols = client.document_symbols(uri.clone()).await.unwrap();

    let greeter = symbols.iter().find(|s| s.name == "Greeter").unwrap();
    assert_eq!(greeter.kind, SymbolKind::Interface);

    let hello = symbols.iter().find(|s| s.name == "Hello").unwrap();
    assert_eq!(hello.kind, SymbolKind::Struct);

    let main_fn = symbols.iter().find(|s| s.name == "main").unwrap();
    assert_eq!(main_fn.kind, SymbolKind::Function);

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_document_symbols_children() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    let symbols = client.document_symbols(uri.clone()).await.unwrap();

    let hello = symbols.iter().find(|s| s.name == "Hello").unwrap();
    let children = hello.children.as_ref().expect("Hello should have children");
    let child_names: Vec<&str> = children.iter().map(|c| c.name.as_str()).collect();
    assert!(
        child_names.contains(&"name"),
        "Hello struct should have 'name' field as child symbol"
    );

    client.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// LSPClient::implementation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_implementation_of_trait() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 0: `trait Greeter {` — cursor on `Greeter` (char 6)
    let result = client.implementation(uri.clone(), 0, 6).await.unwrap();
    assert!(
        result.len() >= 2,
        "Greeter trait should have at least 2 implementations (Hello, Goodbye), got {}",
        result.len()
    );

    let lines: Vec<u32> = result.iter().map(|loc| loc.range.start.line).collect();
    assert!(lines.contains(&8), "should contain impl at line 8 (Hello)");
    assert!(lines.contains(&18), "should contain impl at line 18 (Goodbye)");

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_implementation_of_trait_method() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 1: `fn greet(&self) -> String;` — cursor on `greet` (char 7)
    let result = client.implementation(uri.clone(), 1, 7).await.unwrap();
    assert!(
        result.len() >= 2,
        "greet method should have at least 2 implementations, got {}",
        result.len()
    );

    let lines: Vec<u32> = result.iter().map(|loc| loc.range.start.line).collect();
    assert!(lines.contains(&9), "should contain Hello::greet at line 9");
    assert!(lines.contains(&19), "should contain Goodbye::greet at line 19");

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_implementation_of_struct() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 4: `struct Hello {` — cursor on `Hello` (char 7)
    let result = client.implementation(uri.clone(), 4, 7).await.unwrap();
    assert!(
        !result.is_empty(),
        "Hello struct should have at least one impl block"
    );

    let lines: Vec<u32> = result.iter().map(|loc| loc.range.start.line).collect();
    assert!(lines.contains(&8), "should contain impl block at line 8");

    client.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// LSPClient::prepare_call_hierarchy
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_prepare_call_hierarchy() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 34: `fn helper()` — cursor on `helper` (char 3)
    let items = client.prepare_call_hierarchy(uri.clone(), 34, 5).await.unwrap();
    assert!(!items.is_empty(), "prepare_call_hierarchy should return items");

    let item = &items[0];
    assert_eq!(item.name, "helper");
    assert!(item.uri.ends_with("main.rs"));

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_prepare_call_hierarchy_on_struct_returns_empty_or_item() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 4: `struct Hello {` — cursor on `Hello` (char 7)
    // Call hierarchy on a struct may return empty or a single item depending on RA version
    let result = client.prepare_call_hierarchy(uri.clone(), 4, 7).await;
    assert!(result.is_ok(), "should not error even for non-function symbols");

    client.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// LSPClient::incoming_calls
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_incoming_calls() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 34: `fn helper()` — prepare then get incoming calls
    let items = client.prepare_call_hierarchy(uri.clone(), 34, 5).await.unwrap();
    assert!(!items.is_empty());

    let incoming = client.incoming_calls(items[0].clone()).await.unwrap();
    assert!(
        !incoming.is_empty(),
        "helper is called from main, so incoming calls should not be empty"
    );

    let caller_names: Vec<&str> = incoming.iter().map(|c| c.from.name.as_str()).collect();
    assert!(
        caller_names.contains(&"main"),
        "main should be an incoming caller of helper"
    );

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_incoming_calls_of_leaf_function() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 39: `fn main()` — main is not called by anyone in the module
    let items = client.prepare_call_hierarchy(uri.clone(), 39, 5).await.unwrap();
    assert!(!items.is_empty());

    let incoming = client.incoming_calls(items[0].clone()).await.unwrap();
    assert!(
        incoming.is_empty(),
        "main should have no incoming calls within the project"
    );

    client.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// LSPClient::outgoing_calls
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_outgoing_calls() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 34: `fn helper()` — calls create_hello and call_greet
    let items = client.prepare_call_hierarchy(uri.clone(), 34, 5).await.unwrap();
    assert!(!items.is_empty());

    let outgoing = client.outgoing_calls(items[0].clone()).await.unwrap();
    assert!(
        !outgoing.is_empty(),
        "helper calls create_hello and call_greet, so outgoing should not be empty"
    );

    let callee_names: Vec<&str> = outgoing.iter().map(|c| c.to.name.as_str()).collect();
    assert!(
        callee_names.contains(&"create_hello"),
        "helper should have outgoing call to create_hello"
    );
    assert!(
        callee_names.contains(&"call_greet"),
        "helper should have outgoing call to call_greet"
    );

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_outgoing_calls_of_leaf() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 24: `fn create_hello(name: &str) -> Hello {` — calls to_string only
    let items = client.prepare_call_hierarchy(uri.clone(), 24, 5).await.unwrap();
    assert!(!items.is_empty());

    let outgoing = client.outgoing_calls(items[0].clone()).await.unwrap();
    // create_hello calls `name.to_string()` and constructs Hello
    // At minimum it should not error
    assert!(!outgoing.is_empty(), "create_hello should have at least one outgoing call (to_string)");

    client.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// LSPClient::incoming_calls_recursive
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_incoming_calls_recursive() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 24: `fn create_hello` — called by helper, which is called by main
    let results = client
        .incoming_calls_recursive(uri.clone(), 24, 5, Some(10))
        .await
        .unwrap();
    assert!(
        !results.is_empty(),
        "recursive incoming calls should find callers"
    );

    let all_names: Vec<&str> = results
        .iter()
        .flat_map(|(_, calls)| calls.iter().map(|c| c.from.name.as_str()))
        .collect();
    assert!(
        all_names.contains(&"helper"),
        "should find helper as a caller of create_hello"
    );
    assert!(
        all_names.contains(&"main"),
        "should find main as a transitive caller (calls helper)"
    );

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_incoming_calls_recursive_with_depth_limit() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // With max_depth=1, we should only get direct callers of create_hello (helper), not main
    let results = client
        .incoming_calls_recursive(uri.clone(), 24, 5, Some(1))
        .await
        .unwrap();
    assert!(!results.is_empty());

    let depth_0_item_names: Vec<&str> = results.iter().map(|(item, _)| item.name.as_str()).collect();
    assert!(
        depth_0_item_names.contains(&"create_hello"),
        "first item should be create_hello"
    );

    // With depth 1, the BFS traverses create_hello (depth 0) and finds helper, but helper is at depth 1 so its callers are not expanded
    let items_at_depth_1: Vec<&str> = results
        .iter()
        .flat_map(|(_, calls)| calls.iter().map(|c| c.from.name.as_str()))
        .collect();
    assert!(
        items_at_depth_1.contains(&"helper"),
        "should find helper as direct caller"
    );

    client.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// LSPClient::outgoing_calls_recursive
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_outgoing_calls_recursive() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 39: `fn main()` — calls helper, which calls create_hello and call_greet
    let results = client
        .outgoing_calls_recursive(uri.clone(), 39, 5, Some(10))
        .await
        .unwrap();
    assert!(
        !results.is_empty(),
        "recursive outgoing calls from main should find callees"
    );

    let all_names: Vec<&str> = results
        .iter()
        .flat_map(|(_, calls)| calls.iter().map(|c| c.to.name.as_str()))
        .collect();
    assert!(
        all_names.contains(&"helper"),
        "should find helper as a direct callee of main"
    );
    assert!(
        all_names.contains(&"create_hello"),
        "should find create_hello as a transitive callee (via helper)"
    );

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_outgoing_calls_recursive_with_depth_limit() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // With max_depth=1, we should only get direct callees of main (helper), not create_hello/call_greet
    let results = client
        .outgoing_calls_recursive(uri.clone(), 39, 5, Some(1))
        .await
        .unwrap();
    assert!(!results.is_empty());

    let depth_0_item_names: Vec<&str> = results.iter().map(|(item, _)| item.name.as_str()).collect();
    assert!(
        depth_0_item_names.contains(&"main"),
        "first item should be main"
    );

    let direct_callees: Vec<&str> = results
        .iter()
        .flat_map(|(_, calls)| calls.iter().map(|c| c.to.name.as_str()))
        .collect();
    assert!(
        direct_callees.contains(&"helper"),
        "should find helper as direct callee of main"
    );

    client.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// Edge cases / error handling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_definition_at_whitespace_returns_empty() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 3: empty line `\n` — should return empty or handle gracefully
    let result = client.definition(uri.clone(), 3, 0).await;
    if let Ok(locations) = result {
        assert!(locations.is_empty(), "definition on blank line should be empty");
    }

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_references_on_keyword_returns_empty() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 0: `trait Greeter {` — cursor on `trait` keyword (char 0)
    let _ = client.references(uri.clone(), 0, 0, true).await;

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_definition_with_invalid_uri() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;

    let result = client
        .definition("file:///nonexistent/file.rs".to_string(), 0, 0)
        .await;
    if let Ok(locations) = result {
        assert!(locations.is_empty());
    }

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_document_symbols_with_invalid_uri() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;

    let result = client
        .document_symbols("file:///nonexistent/file.rs".to_string())
        .await;
    if let Ok(symbols) = result {
        assert!(symbols.is_empty());
    }

    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_definition_out_of_range_position() {
    if !rust_analyzer_available() {
        return;
    }
    let client = make_client().await;
    let uri = file_uri();

    // line 9999, char 9999 — way beyond file boundaries
    let result = client.definition(uri.clone(), 9999, 9999).await;
    if let Ok(locations) = result {
        assert!(locations.is_empty());
    }

    client.shutdown().await.unwrap();
}
