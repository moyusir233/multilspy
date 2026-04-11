"""
Tests for the multilspy_cli module.
"""

import json
import pytest

from multilspy_cli.position_utils import (
    raw_to_lsp_position,
    lsp_to_raw_position,
    convert_lsp_location_to_raw,
    convert_lsp_range_to_raw,
    convert_all_locations_to_raw,
    convert_call_hierarchy_item_to_raw,
    convert_incoming_calls_to_raw,
    convert_outgoing_calls_to_raw,
    convert_document_symbols_to_raw,
)
from multilspy_cli.server import LSPManager, create_app


class TestPositionUtils:
    """Tests for position conversion utilities."""

    def test_raw_to_lsp_position(self):
        """Test converting raw (1-based) to LSP (0-based) positions."""
        result = raw_to_lsp_position(1, 1)
        assert result == {"line": 0, "character": 0}

        result = raw_to_lsp_position(10, 5)
        assert result == {"line": 9, "character": 4}

        result = raw_to_lsp_position(42, 100)
        assert result == {"line": 41, "character": 99}

    def test_lsp_to_raw_position(self):
        """Test converting LSP (0-based) to raw (1-based) positions."""
        result = lsp_to_raw_position(0, 0)
        assert result == {"line": 1, "column": 1}

        result = lsp_to_raw_position(9, 4)
        assert result == {"line": 10, "column": 5}

        result = lsp_to_raw_position(41, 99)
        assert result == {"line": 42, "column": 100}

    def test_round_trip_conversion(self):
        """Test that converting raw -> LSP -> raw gives back the original."""
        for line in [1, 10, 42, 100]:
            for column in [1, 5, 10, 50]:
                lsp = raw_to_lsp_position(line, column)
                raw = lsp_to_raw_position(lsp["line"], lsp["character"])
                assert raw == {"line": line, "column": column}

    def test_convert_lsp_range_to_raw(self):
        """Test converting an LSP Range object."""
        lsp_range = {
            "start": {"line": 0, "character": 0},
            "end": {"line": 9, "character": 4},
        }
        result = convert_lsp_range_to_raw(lsp_range)
        assert result["start"] == {"line": 1, "column": 1}
        assert result["end"] == {"line": 10, "column": 5}

    def test_convert_lsp_location_to_raw(self):
        """Test converting an LSP Location object."""
        lsp_location = {
            "uri": "file:///test.rs",
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 9, "character": 4},
            },
            "absolutePath": "/test.rs",
            "relativePath": "test.rs",
        }
        result = convert_lsp_location_to_raw(lsp_location)
        assert result["uri"] == "file:///test.rs"
        assert result["absolutePath"] == "/test.rs"
        assert result["relativePath"] == "test.rs"
        assert result["range"]["start"] == {"line": 1, "column": 1}
        assert result["range"]["end"] == {"line": 10, "column": 5}

    def test_convert_all_locations_to_raw(self):
        """Test converting a list of LSP Location objects."""
        locations = [
            {
                "uri": "file:///test1.rs",
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 9, "character": 4},
                },
            },
            {
                "uri": "file:///test2.rs",
                "range": {
                    "start": {"line": 10, "character": 5},
                    "end": {"line": 20, "character": 10},
                },
            },
        ]
        result = convert_all_locations_to_raw(locations)
        assert len(result) == 2
        assert result[0]["range"]["start"] == {"line": 1, "column": 1}
        assert result[0]["range"]["end"] == {"line": 10, "column": 5}
        assert result[1]["range"]["start"] == {"line": 11, "column": 6}
        assert result[1]["range"]["end"] == {"line": 21, "column": 11}

    def test_convert_call_hierarchy_item_to_raw(self):
        """Test converting a CallHierarchyItem."""
        item = {
            "name": "test_function",
            "kind": 12,
            "uri": "file:///test.rs",
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 9, "character": 4},
            },
            "selectionRange": {
                "start": {"line": 0, "character": 4},
                "end": {"line": 0, "character": 16},
            },
        }
        result = convert_call_hierarchy_item_to_raw(item)
        assert result["name"] == "test_function"
        assert result["kind"] == 12
        assert result["range"]["start"] == {"line": 1, "column": 1}
        assert result["range"]["end"] == {"line": 10, "column": 5}
        assert result["selectionRange"]["start"] == {"line": 1, "column": 5}
        assert result["selectionRange"]["end"] == {"line": 1, "column": 17}

    def test_convert_incoming_calls_to_raw(self):
        """Test converting incoming calls."""
        calls = [
            {
                "from": {
                    "name": "caller_function",
                    "kind": 12,
                    "uri": "file:///caller.rs",
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": {"line": 9, "character": 4},
                    },
                    "selectionRange": {
                        "start": {"line": 0, "character": 4},
                        "end": {"line": 0, "character": 18},
                    },
                },
                "fromRanges": [
                    {
                        "start": {"line": 5, "character": 10},
                        "end": {"line": 5, "character": 20},
                    }
                ],
            }
        ]
        result = convert_incoming_calls_to_raw(calls)
        assert len(result) == 1
        assert result[0]["from"]["name"] == "caller_function"
        assert result[0]["from"]["range"]["start"] == {"line": 1, "column": 1}
        assert result[0]["fromRanges"][0]["start"] == {"line": 6, "column": 11}
        assert result[0]["fromRanges"][0]["end"] == {"line": 6, "column": 21}

    def test_convert_outgoing_calls_to_raw(self):
        """Test converting outgoing calls."""
        calls = [
            {
                "to": {
                    "name": "callee_function",
                    "kind": 12,
                    "uri": "file:///callee.rs",
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": {"line": 9, "character": 4},
                    },
                    "selectionRange": {
                        "start": {"line": 0, "character": 4},
                        "end": {"line": 0, "character": 18},
                    },
                },
                "fromRanges": [
                    {
                        "start": {"line": 5, "character": 10},
                        "end": {"line": 5, "character": 20},
                    }
                ],
            }
        ]
        result = convert_outgoing_calls_to_raw(calls)
        assert len(result) == 1
        assert result[0]["to"]["name"] == "callee_function"
        assert result[0]["to"]["range"]["start"] == {"line": 1, "column": 1}
        assert result[0]["fromRanges"][0]["start"] == {"line": 6, "column": 11}
        assert result[0]["fromRanges"][0]["end"] == {"line": 6, "column": 21}

    def test_convert_document_symbols_to_raw(self):
        """Test converting document symbols."""
        symbols = [
            {
                "name": "test_function",
                "kind": 12,
                "location": {
                    "uri": "file:///test.rs",
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": {"line": 9, "character": 4},
                    },
                    "absolutePath": "/test.rs",
                    "relativePath": "test.rs",
                },
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 9, "character": 4},
                },
                "selectionRange": {
                    "start": {"line": 0, "character": 4},
                    "end": {"line": 0, "character": 16},
                },
            }
        ]
        result = convert_document_symbols_to_raw(symbols)
        assert len(result) == 1
        assert result[0]["name"] == "test_function"
        assert result[0]["location"]["range"]["start"] == {"line": 1, "column": 1}
        assert result[0]["range"]["start"] == {"line": 1, "column": 1}
        assert result[0]["selectionRange"]["start"] == {"line": 1, "column": 5}


class TestCLIModule:
    """Tests for the CLI module structure."""

    def test_module_imports(self):
        """Test that all modules can be imported."""
        import multilspy_cli

        assert hasattr(multilspy_cli, "raw_to_lsp_position")
        assert hasattr(multilspy_cli, "LSPClient")
        assert hasattr(multilspy_cli, "LSPManager")
        assert hasattr(multilspy_cli, "__version__")

    def test_cli_module_exists(self):
        """Test that the cli module exists."""
        import multilspy_cli.cli

        assert hasattr(multilspy_cli.cli, "main")
        assert hasattr(multilspy_cli.cli, "get_parser")

    def test_server_module_exists(self):
        """Test that the server module exists."""
        import multilspy_cli.server

        assert hasattr(multilspy_cli.server, "LSPManager")
        assert hasattr(multilspy_cli.server, "create_app")
        assert hasattr(multilspy_cli.server, "start_server")
        assert hasattr(multilspy_cli.server, "stop_server")

    def test_client_module_exists(self):
        """Test that the client module exists."""
        import multilspy_cli.client

        assert hasattr(multilspy_cli.client, "LSPClient")
        assert hasattr(multilspy_cli.client, "LSPClientError")
        assert hasattr(multilspy_cli.client, "start_server_command")
        assert hasattr(multilspy_cli.client, "stop_server_command")


class TestQuartApp:
    """Tests for the Quart application."""

    def test_create_app(self):
        """Test that the Quart app can be created."""
        manager = LSPManager()
        app = create_app(manager)
        assert app is not None
        assert app.name == "multilspy_cli.server"

    @pytest.mark.asyncio
    async def test_health_endpoint(self):
        """Test the health endpoint."""
        manager = LSPManager()
        app = create_app(manager)
        async with app.test_client() as client:
            response = await client.get("/health")
            assert response.status_code == 200
            data = await response.get_json()
            assert data["status"] == "ok"

    @pytest.mark.asyncio
    async def test_instances_endpoint(self):
        """Test the instances endpoint."""
        manager = LSPManager()
        app = create_app(manager)
        async with app.test_client() as client:
            response = await client.get("/instances")
            assert response.status_code == 200
            data = await response.get_json()
            assert "instances" in data
            assert isinstance(data["instances"], list)

    @pytest.mark.asyncio
    async def test_start_missing_project_path(self):
        """Test start endpoint with missing project_path."""
        manager = LSPManager()
        app = create_app(manager)
        async with app.test_client() as client:
            response = await client.post("/start", json={})
            assert response.status_code == 400
            data = await response.get_json()
            assert data["status"] == "error"
            assert "project_path" in data["message"]

    @pytest.mark.asyncio
    async def test_start_nonexistent_project(self):
        """Test start endpoint with nonexistent project."""
        manager = LSPManager()
        app = create_app(manager)
        async with app.test_client() as client:
            response = await client.post(
                "/start",
                json={"project_path": "/nonexistent/path/that/should/not/exist"},
            )
            assert response.status_code == 400
            data = await response.get_json()
            assert data["status"] == "error"
            assert "does not exist" in data["message"]

    @pytest.mark.asyncio
    async def test_shutdown_missing_project_path(self):
        """Test shutdown endpoint with missing project_path."""
        manager = LSPManager()
        app = create_app(manager)
        async with app.test_client() as client:
            response = await client.post("/shutdown", json={})
            assert response.status_code == 400
            data = await response.get_json()
            assert data["status"] == "error"
            assert "project_path" in data["message"]

    @pytest.mark.asyncio
    async def test_definition_missing_params(self):
        """Test definition endpoint with missing parameters."""
        manager = LSPManager()
        app = create_app(manager)
        async with app.test_client() as client:
            response = await client.post("/definition", json={})
            assert response.status_code == 400
            data = await response.get_json()
            assert data["status"] == "error"
            assert (
                "project_path, file_path, line, and column are required"
                in data["message"]
            )


def test_cli_parser_definition_command():
    """Test that the definition command exists in the CLI parser."""
    from multilspy_cli.cli import get_parser

    parser = get_parser()

    # Test that definition is in the commands
    subcommands = [action.dest for action in parser._subparsers._actions]
    assert "command" in subcommands

    # Test that definition command is recognized
    args = parser.parse_args(["definition", "src/test.rs", "42", "10"])
    assert args.command == "definition"
    assert args.file == "src/test.rs"
    assert args.line == 42
    assert args.column == 10


def test_cli_full_workflow():
    """Test the full CLI workflow: start server, query definition, stop server."""
    import subprocess
    import json
    import time
    import os
    import sys
    from pathlib import PurePath

    # Get project root to add src directory to Python path
    test_file_path = os.path.abspath(__file__)
    project_root = os.path.dirname(os.path.dirname(os.path.dirname(test_file_path)))
    src_dir = os.path.join(project_root, "src")

    # Set up environment with src directory in PYTHONPATH
    env = os.environ.copy()
    env["PYTHONPATH"] = src_dir + os.pathsep + env.get("PYTHONPATH", "")

    # Use local rust-sdk project, same as test_multilspy_rust_implementation_and_definition
    project_dir = "/home/yangchengrun/lark-client/rust-sdk"
    test_file = str(PurePath("chat/chat-modules/chat-chats/src/services/chats.rs"))
    line = 73  # 1-based line number (matches test_multilspy_rust.py's 72 0-based line)
    column = (
        15  # 1-based column number (matches test_multilspy_rust.py's 14 0-based column)
    )

    # Verify the project and test file exist
    assert os.path.exists(project_dir), f"Rust SDK project not found at {project_dir}"
    assert os.path.exists(os.path.join(project_dir, test_file)), (
        f"Test file not found at {test_file}"
    )

    # First, make sure no server is running
    subprocess.run(
        [sys.executable, "-m", "multilspy_cli.cli", "server", "stop"],
        capture_output=True,
        cwd=project_dir,
        env=env,
        check=False,
    )

    start_proc = None
    try:
        # Step 1: Start the server
        start_proc = subprocess.Popen(
            [
                sys.executable,
                "-m",
                "multilspy_cli.cli",
                "server",
                "start",
                "--no-daemon",
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=project_dir,
            env=env,
        )

        # Step 2: Wait for server to be running (max 10 minutes for rust-analyzer indexing)
        server_ready = False
        for _ in range(600):
            time.sleep(1)
            # Check if server process is still alive
            if start_proc.poll() is not None:
                stdout, stderr = start_proc.communicate()
                raise AssertionError(
                    f"Server process died unexpectedly. stdout: {stdout}, stderr: {stderr}"
                )
            # Check server status
            status_result = subprocess.run(
                [sys.executable, "-m", "multilspy_cli.cli", "server", "status"],
                capture_output=True,
                text=True,
                cwd=project_dir,
                env=env,
                check=False,
            )
            if status_result.returncode == 0:
                try:
                    status_data = json.loads(status_result.stdout)
                    if status_data["status"] == "running":
                        server_ready = True
                        break
                except json.JSONDecodeError:
                    pass
        assert server_ready, "Server did not start within 10 minutes"

        # Step 3: Test definition query
        def_result = subprocess.run(
            [
                sys.executable,
                "-m",
                "multilspy_cli.cli",
                "definition",
                test_file,
                str(line),
                str(column),
            ],
            capture_output=True,
            text=True,
            cwd=project_dir,
            env=env,
            check=False,
        )
        assert def_result.returncode == 0, (
            f"Definition query failed: {def_result.stderr}"
        )
        def_data = json.loads(def_result.stdout)
        assert def_data["status"] == "ok"
        assert len(def_data["result"]) >= 1

        # Verify the result structure is correct
        assert len(def_data["result"]) > 0
        definition_item = def_data["result"][0]
        # Verify all required fields exist
        assert "uri" in definition_item
        assert "range" in definition_item
        assert "absolutePath" in definition_item
        assert "relativePath" in definition_item
        # Verify positions are 1-based
        assert definition_item["range"]["start"]["line"] >= 1
        assert definition_item["range"]["start"]["column"] >= 1
        assert definition_item["range"]["end"]["line"] >= 1
        assert definition_item["range"]["end"]["column"] >= 1

    finally:
        # Step 4: Stop the server
        stop_result = subprocess.run(
            [sys.executable, "-m", "multilspy_cli.cli", "server", "stop"],
            capture_output=True,
            text=True,
            cwd=project_dir,
            env=env,
            check=False,
        )

        # Kill the start process if it's still running
        if start_proc is not None and start_proc.poll() is None:
            start_proc.terminate()
            try:
                start_proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                start_proc.kill()
                start_proc.wait()

        # Wait for server to stop
        time.sleep(1)

    # Verify server is stopped
    status_result = subprocess.run(
        [sys.executable, "-m", "multilspy_cli.cli", "server", "status"],
        capture_output=True,
        text=True,
        cwd=project_dir,
        env=env,
        check=False,
    )
    assert status_result.returncode == 0, (
        f"Status command failed: {status_result.stderr}"
    )
    status_data = json.loads(status_result.stdout)
    assert status_data["status"] == "stopped"
