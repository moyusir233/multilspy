#!/usr/bin/env python3
"""
Rust Analyzer LSP CLI - A command-line interface for Rust code analysis using LSP.

This CLI provides a convenient interface to the Rust Analyzer LSP server,
allowing you to query type definitions, references, document symbols,
implementations, and call hierarchy information.
"""

import argparse
import json
import os
import sys
from typing import Optional, Sequence

import logging

logging.basicConfig(level=logging.INFO)


def print_json(data: object) -> None:
    """Print data as JSON."""
    print(json.dumps(data, indent=2, ensure_ascii=False))


def print_error(message: str, exit_code: int = 1) -> None:
    """Print an error message as JSON and exit."""
    print_json({"status": "error", "message": message})
    sys.exit(exit_code)


def get_parser() -> argparse.ArgumentParser:
    """Create the argument parser."""
    parser = argparse.ArgumentParser(
        description="Rust Analyzer LSP CLI - Analyze Rust code using LSP",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Start the LSP server for the current project
  ra-lsp server start

  # Get definition at a position
  ra-lsp definition src/lib.rs 42 10

  # Get type definition at a position
  ra-lsp type-definition src/lib.rs 42 10

  # Get all references to a symbol
  ra-lsp references src/lib.rs 42 10

  # List all symbols in a file
  ra-lsp document-symbols src/lib.rs

  # Get implementations of a trait or function
  ra-lsp implementation src/lib.rs 42 10

  # Get all callers of a function
  ra-lsp incoming-calls src/lib.rs 42 10

  # Get all functions called by a function
  ra-lsp outgoing-calls src/lib.rs 42 10

  # Get recursive incoming calls (with default max depth 10)
  ra-lsp incoming-calls-recursive src/lib.rs 42 10

  # Get recursive outgoing calls with custom max depth
  ra-lsp outgoing-calls-recursive src/lib.rs 42 10 --max-depth 5

  # Shutdown the server
  ra-lsp server stop

Note:
- All line and column numbers are 1-based.
- File paths must be relative to the project root directory.
- By default, the current working directory is used as the project root.
        """,
    )
    parser.add_argument(
        "--project",
        "-p",
        help="Path to the Rust project root (default: current working directory)",
        default=None,
    )

    subparsers = parser.add_subparsers(title="Commands", dest="command", required=True)

    # Server commands
    server_parser = subparsers.add_parser(
        "server",
        help="Manage the LSP server",
        description="Manage the LSP server lifecycle",
    )
    server_subparsers = server_parser.add_subparsers(
        title="Server Commands", dest="server_command", required=True
    )

    server_start_parser = server_subparsers.add_parser(
        "start",
        help="Start the LSP server",
        description="Start the LSP server in the background",
    )
    server_start_parser.add_argument(
        "--no-daemon",
        action="store_true",
        help="Run in foreground instead of daemon mode",
    )

    server_subparsers.add_parser(
        "stop", help="Stop the LSP server", description="Stop the running LSP server"
    )

    server_subparsers.add_parser(
        "status",
        help="Show server status",
        description="Check if the server is running",
    )

    # LSP feature commands
    def add_position_args(parser: argparse.ArgumentParser, help_suffix: str) -> None:
        parser.add_argument(
            "file",
            help=f"Path to the Rust file {help_suffix} (relative to project root)",
        )
        parser.add_argument("line", type=int, help=f"1-based line number {help_suffix}")
        parser.add_argument(
            "column", type=int, help=f"1-based column number {help_suffix}"
        )

    # Definition
    def_parser = subparsers.add_parser(
        "definition",
        help="Get definition of a symbol",
        description="Find the source location where a Rust symbol is defined",
    )
    add_position_args(def_parser, "of the symbol")

    # Type definition
    type_def_parser = subparsers.add_parser(
        "type-definition",
        help="Get type definition of a symbol",
        description="Find the source location where a Rust struct, function, or trait is defined",
    )
    add_position_args(type_def_parser, "of the symbol")

    # References
    references_parser = subparsers.add_parser(
        "references",
        help="Get all references to a symbol",
        description="Find all locations where a Rust symbol is referenced",
    )
    add_position_args(references_parser, "of the symbol")

    # Document symbols
    doc_symbols_parser = subparsers.add_parser(
        "document-symbols",
        help="Get all symbols in a document",
        description="List all functions, structs, traits, and other symbols in a Rust file",
    )
    doc_symbols_parser.add_argument(
        "file",
        help="Path to the Rust file (relative to project root)",
    )

    # Implementation
    impl_parser = subparsers.add_parser(
        "implementation",
        help="Get implementations of a function or trait",
        description="Find all implementation locations for a Rust function or trait",
    )
    add_position_args(impl_parser, "of the symbol")

    # Incoming calls
    incoming_parser = subparsers.add_parser(
        "incoming-calls",
        help="Get all callers of a function",
        description="Find all functions that call the target function",
    )
    add_position_args(incoming_parser, "of the function")

    # Outgoing calls
    outgoing_parser = subparsers.add_parser(
        "outgoing-calls",
        help="Get all functions called by a function",
        description="Find all functions called by the target function",
    )
    add_position_args(outgoing_parser, "of the function")

    # Incoming calls recursive
    incoming_recursive_parser = subparsers.add_parser(
        "incoming-calls-recursive",
        help="Get recursive incoming calls to a function",
        description="Find all functions that call the target function, recursively",
    )
    add_position_args(incoming_recursive_parser, "of the function")
    incoming_recursive_parser.add_argument(
        "--max-depth",
        type=int,
        default=10,
        help="Maximum recursion depth (default: 10)",
    )

    # Outgoing calls recursive
    outgoing_recursive_parser = subparsers.add_parser(
        "outgoing-calls-recursive",
        help="Get recursive outgoing calls from a function",
        description="Find all functions called by the target function, recursively",
    )
    add_position_args(outgoing_recursive_parser, "of the function")
    outgoing_recursive_parser.add_argument(
        "--max-depth",
        type=int,
        default=10,
        help="Maximum recursion depth (default: 10)",
    )

    return parser


def resolve_project_path(project_path: Optional[str]) -> str:
    """Resolve the project path, finding Cargo.toml if needed."""
    if project_path is None:
        project_path = os.getcwd()

    project_path = os.path.abspath(project_path)

    # If it's a file, use its directory
    if os.path.isfile(project_path):
        project_path = os.path.dirname(project_path)

    # Look for Cargo.toml
    current = project_path
    while True:
        cargo_toml = os.path.join(current, "Cargo.toml")
        if os.path.exists(cargo_toml):
            return current
        parent = os.path.dirname(current)
        if parent == current:  # Reached filesystem root
            break
        current = parent

    return project_path


def resolve_file_path(project_path: str, file_path: str) -> str:
    """Resolve a file path relative to the project."""
    if os.path.isabs(file_path):
        # If absolute, make it relative to project
        try:
            return os.path.relpath(file_path, project_path)
        except ValueError:
            # If not under project, return as is
            return file_path
    return file_path


def main(argv: Optional[Sequence[str]] = None) -> None:
    """Main entry point."""
    parser = get_parser()
    args = parser.parse_args(argv)

    # Handle server commands
    if args.command == "server":
        from .client import start_server_command, stop_server_command, status_command

        if args.server_command == "start":
            start_server_command(daemon=not args.no_daemon)
        elif args.server_command == "stop":
            stop_server_command()
        elif args.server_command == "status":
            status_command()
        return

    # Handle LSP commands
    project_path = resolve_project_path(args.project)

    # Import client here to avoid slow startup for server commands
    from .client import LSPClient, LSPClientError

    try:
        logger = logging.getLogger("LSPClient")
        client = LSPClient(logger, project_path)

        if args.command == "definition":
            file_path = resolve_file_path(project_path, args.file)
            result = client.definition(file_path, args.line, args.column)
            print_json({"status": "ok", "result": result})

        elif args.command == "type-definition":
            file_path = resolve_file_path(project_path, args.file)
            result = client.type_definition(file_path, args.line, args.column)
            print_json({"status": "ok", "result": result})

        elif args.command == "references":
            file_path = resolve_file_path(project_path, args.file)
            result = client.references(file_path, args.line, args.column)
            print_json({"status": "ok", "result": result})

        elif args.command == "document-symbols":
            file_path = resolve_file_path(project_path, args.file)
            result = client.document_symbols(file_path)
            print_json({"status": "ok", "result": result})

        elif args.command == "implementation":
            file_path = resolve_file_path(project_path, args.file)
            result = client.implementation(file_path, args.line, args.column)
            print_json({"status": "ok", "result": result})

        elif args.command == "incoming-calls":
            file_path = resolve_file_path(project_path, args.file)
            result = client.incoming_calls(file_path, args.line, args.column)
            print_json({"status": "ok", "result": result})

        elif args.command == "outgoing-calls":
            file_path = resolve_file_path(project_path, args.file)
            result = client.outgoing_calls(file_path, args.line, args.column)
            print_json({"status": "ok", "result": result})

        elif args.command == "incoming-calls-recursive":
            file_path = resolve_file_path(project_path, args.file)
            result = client.incoming_calls_recursive(
                file_path, args.line, args.column, args.max_depth
            )
            print_json({"status": "ok", "result": result})

        elif args.command == "outgoing-calls-recursive":
            file_path = resolve_file_path(project_path, args.file)
            result = client.outgoing_calls_recursive(
                file_path, args.line, args.column, args.max_depth
            )
            print_json({"status": "ok", "result": result})

    except LSPClientError as e:
        print_error(str(e))
    except KeyboardInterrupt:
        print_error("Interrupted by user", exit_code=130)
    except Exception as e:
        print_error(f"Unexpected error: {e}", exit_code=1)


if __name__ == "__main__":
    main()
