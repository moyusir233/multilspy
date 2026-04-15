# Implement `analyze-fn-call-trait-deps-graph` CLI Command for Multilspy

## Role
You are a senior Rust LSP tooling engineer with deep expertise in the multilspy codebase, Language Server Protocol (LSP) specifications, and CLI application design. You are familiar with the existing `multilspy-cli` and `multilspy-rust` crate implementations, including the already implemented `workspace/symbol`, `textDocument/implementation`, `textDocument/documentSymbol`, and `outgoing-calls-recursive` capabilities.

## Core Task
Implement a new CLI command `analyze-fn-call-trait-deps-graph` for the `multilspy-cli` application.

This command analyzes trait-method dependencies for exactly one user-specified Rust entry function within one or more target directories. The entry function is provided by the user as a source location tuple: `uri + line + character`. The command must recursively traverse the entry function's outgoing call graph, allow traversal through non-trait functions, detect whether any reachable calls map to methods belonging to a user-specified trait set, and output the result in JSON format.

The output is a single-entry dependency result, not a multi-entry function-to-function graph. The graph model still uses a directed graph internally, but the only logical node exposed in the CLI result is the resolved entry function.

## Definitions
- **Entry function location**: exactly one source location provided by the user as `--uri + --line + --character`. This is the root of the analysis.
- **Target trait set**: the set of traits explicitly provided by the user by name.
- **Target trait method**: any method that belongs to a trait in the target trait set.
- **Dependency ID**: a string identifier for a matched target trait method, formatted similarly to `ChatService.call_fn_name`.
- **Target directories**: one or more directory scopes used to filter relevant symbols and implementations. They must support both relative paths from the current working directory and full file URIs.

## Success Criteria
1. The command accepts exactly one entry function location tuple (`--uri + --line + --character`), one or more trait names, and one or more target directory paths.
2. The command resolves the entry function location to exactly one function or method symbol. If the location does not resolve to a function-like symbol, it returns an error.
3. The command resolves each trait name to exactly one trait symbol. If multiple symbols match for any trait, it returns an error.
4. The command recursively analyzes outgoing calls reachable from the entry function.
5. Recursive traversal may pass through non-trait functions, but only matched target trait methods are recorded as dependencies.
6. Each matched dependency must include at least one call stack from the entry function to the matched target trait method.
7. The command outputs valid JSON matching the required schema.
8. The implementation reuses 100% of existing LSP request interfaces and does not add any new LSP endpoint implementations.
9. The new public interface in `multilspy-rust/src/logic.rs` follows the same pattern as the existing `outgoing_calls_recursive` method.
10. The implementation introduces no breaking changes to existing functionality.

## Prerequisite Context
- Reference existing implementations in `multilspy-cli` and `multilspy-rust`.
- Reuse existing capabilities for workspace symbol resolution, implementation lookup, document symbol lookup, and outgoing recursive call analysis.
- Use the `petgraph` crate for graph data structure management: `https://docs.rs/petgraph/latest/petgraph/`.

## Required Behavior

### 1. Input Validation
- Validate that the command receives exactly:
  - one entry function file URI
  - one entry function line
  - one entry function character
  - one or more trait names
  - one or more target directory paths
- The CLI should allow repeated `--target-dir <DIR>` flags for this command.
- The CLI must require `--uri` for the entry function file and must not use `--relative-path` for this command.
- Convert relative directory paths to absolute file URIs as required by the existing LSP flow.
- Return a clear error if any required argument is missing or malformed.

### 2. Entry Function Resolution
- The user input for the entry function is a concrete source location: `--uri + --line + --character`.
- Resolve the entry function directly from the provided location using existing LSP capabilities only.
- Preferred approach:
  - use `textDocument/documentSymbol` to load symbols for the file
  - identify the enclosing function or method whose range or selection range contains the provided position
- If needed, reuse existing call-hierarchy preparation logic at the same position as an additional validation step.
- Resolution rules:
  - if the file cannot be loaded, return an error
  - if the position is not inside any function or method symbol, return an error
  - if the position ambiguously maps to multiple function-like symbols, return an error
  - if exactly one matching function-like symbol is found, use it as the entry function

### 3. Trait Resolution
- Use `workspace/symbol` to search for each user-specified trait name.
- Filter matches strictly:
  - `name` must exactly match the input trait name
  - matching is case-sensitive
  - `kind` must equal `11` (`SymbolKind::Interface`), which is the expected mapping for Rust traits in this workflow
- Apply target-directory filtering strictly:
  - only trait declarations whose resolved URI is inside at least one `--target-dir` are eligible
  - any same-named trait declarations outside all provided `--target-dir` values must be ignored
- Resolution rules for each trait:
  - if zero matching symbols remain, return an error
  - if more than one matching symbol remains, return an error
  - if exactly one matching symbol remains, use it as the resolved trait

### 4. Target Trait Method Set Construction
- Build the set of target trait methods for the resolved traits.
- Reuse existing LSP capabilities only. Do not add any new LSP request methods.
- Each target trait method must have a dependency ID represented as a string formatted similarly to `ChatService.call_fn_name`.
- The final dependency set must be deduplicated by dependency ID.
- Only trait method implementations whose resolved locations are inside at least one `--target-dir` are eligible for the target trait method set.

### 5. Recursive Dependency Analysis
- Start analysis from the resolved entry function only.
- Reuse the existing `outgoing-calls-recursive` implementation without modifying its behavior.
- Recursive traversal is allowed to pass through non-trait functions.
- Do not stop traversal simply because an intermediate function is not in the target trait set.
- During traversal, record a dependency only when a reachable call maps to a target trait method from the resolved trait set, and that matched trait method implementation is inside at least one `--target-dir`.
- Ignore reachable calls that do not map to target trait methods.
- Deduplicate matched dependencies in the final result.
- For each matched dependency, preserve at least one concrete call stack from the entry function to the matched dependency target.
- A call stack should be ordered from entry function -> intermediate function(s) -> matched target trait method.

### 6. Graph Semantics
- Use a directed `petgraph` graph internally if needed for traversal bookkeeping or dependency management.
- The only logical analysis node in the CLI output is the resolved entry function.
- Do not expose external or intermediate functions as output nodes.
- Do not expose non-target functions in the final dependency list.

### 7. Output Preparation
- CLI output must be a valid JSON array.
- The array should contain exactly one item on success, representing the resolved entry function.
- The output item must contain:
  ```json
  {
    "function_name": "<resolved entry function full name>",
    "entry_uri": "<input entry file URI>",
    "entry_position": { "line": <number>, "character": <number> },
    "file_uri": "<file URI where the entry function is defined>",
    "range": {
      "start": { "line": <number>, "character": <number> },
      "end": { "line": <number>, "character": <number> }
    },
    "dependencies": [
      {
        "dependency_id": "ChatService.call_fn_name",
        "callStack": [
          "create_chat",
          "load_chat_context",
          "ChatService.call_fn_name"
        ]
      },
      {
        "dependency_id": "OtherTrait.other_method",
        "callStack": [
          "create_chat",
          "helper_fn",
          "OtherTrait.other_method"
        ]
      }
    ]
  }
  ```
- `function_name`, `entry_uri`, `entry_position`, `file_uri`, and `range` refer to the resolved entry function, not to any dependency item.
- `dependencies` is an array of dependency objects.
- Each dependency object must contain:
  - `dependency_id`: the matched target trait method ID
  - `callStack`: one ordered call stack showing how the entry function reaches that dependency
- `callStack` must begin with the resolved entry function and end with the matched target trait method ID.

## Hard Constraints (MUST FOLLOW)

### 1. Interface Requirements
- Implement a new public method in `multilspy-rust/src/logic.rs` as part of the `LSPClient` struct.
- The public API shape and implementation style must follow the same pattern as `outgoing_calls_recursive`.
- Expose this method to the CLI crate properly.

### 2. Reuse Requirements
- Reuse all existing LSP request interfaces.
- Do not implement any new LSP request methods.
- Reuse the existing `outgoing-calls-recursive` implementation without modification.

### 3. Graph Requirements
- Use `petgraph` exclusively for graph management.
- If edges are used internally, they represent call reachability discovered during traversal.
- Internal graph details must not change the required CLI output schema.

### 4. Error Handling Requirements
- Return clear errors when:
  - the entry function location cannot be resolved
  - the entry function location does not point to a function or method
  - the entry function location resolves ambiguously
  - any trait cannot be resolved
  - any trait resolves to multiple symbols
  - required inputs are missing or invalid
- All LSP requests must use proper error handling and existing timeout behavior.

## Edge Cases
- The entry file URI is invalid or unreadable.
- The provided line/character is outside the file or outside any function-like symbol.
- The entry position ambiguously resolves to multiple function-like symbols.
- A trait name resolves to no symbols.
- A trait name resolves to multiple symbols.
- A resolved trait has no methods.
- Trait implementations are found but none of their methods are reachable from the entry function.
- Reachable calls exist, but none map to the target trait method set.
- Multiple call paths reach the same target trait method; the dependency must appear only once.
- If multiple call paths reach the same target trait method, the implementation may return one representative call stack unless the product requirement explicitly expands to all call stacks.
- Overlapping trait implementations must not produce duplicate dependency IDs.

## Quality Assurance Checklist
- [ ] Input validation enforces exactly one entry function `--uri + --line + --character`, one or more trait names, and one or more target directories.
- [ ] The CLI requires `--uri` for this command and does not accept `--relative-path` as the entry function input.
- [ ] Entry function resolution returns an error when the location does not resolve to exactly one function-like symbol.
- [ ] Trait resolution returns an error on zero or multiple matches.
- [ ] Trait resolution ignores same-named trait declarations outside all provided `--target-dir` values.
- [ ] Recursive traversal can pass through non-trait functions.
- [ ] Only matched target trait methods are recorded in `dependencies`.
- [ ] `dependencies` is a deduplicated dependency-object array keyed by IDs like `ChatService.call_fn_name`.
- [ ] Every dependency object includes a `callStack` from the entry function to the matched trait method.
- [ ] `function_name`, `entry_uri`, `entry_position`, `file_uri`, and `range` refer to the resolved entry function.
- [ ] JSON output is valid and matches the specified schema.
- [ ] Existing CLI commands continue to work unchanged.
- [ ] The new method in `LSPClient` includes proper documentation comments.
- [ ] The CLI command includes clear help text and usage examples.
