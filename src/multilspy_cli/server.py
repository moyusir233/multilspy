"""
HTTP server that manages persistent LSP server instances.

This server runs in the background and manages LSP server instances for
different Rust projects, allowing CLI commands to communicate with a
single long-running LSP server instead of starting a new one for each
command.
"""

import logging
import os
import sys
import tempfile
import threading
import time
import asyncio
from dataclasses import dataclass, field
from typing import Dict, Any, Optional, Tuple

from quart import Quart, request, jsonify
import uvicorn

from multilspy import LanguageServer
from multilspy.multilspy_config import MultilspyConfig, Language
from multilspy.multilspy_logger import MultilspyLogger

from .position_utils import (
    raw_to_lsp_position,
    convert_all_locations_to_raw,
    convert_document_symbols_to_raw,
    convert_incoming_calls_to_raw,
    convert_outgoing_calls_to_raw,
    get_call_hierarchy_key,
    extract_call_hierarchy_item_info,
    convert_call_hierarchy_item_to_raw,
    convert_lsp_range_to_raw,
)

import logging

logging.basicConfig(level=logging.INFO)


@dataclass
class LSPInstance:
    """Represents a running LSP server instance for a project."""

    project_path: str
    lsp: LanguageServer
    context_manager: Any
    lock: asyncio.Lock = field(default_factory=asyncio.Lock)
    ready: asyncio.Event = field(default_factory=asyncio.Event)
    last_used: float = field(default_factory=time.time)


class LSPManager:
    """Manages multiple LSP server instances."""

    def __init__(self, idle_timeout: int = 300):
        """
        Initialize the LSP manager.

        :param idle_timeout: Time in seconds after which idle servers are shut down
        """
        self.instances: Dict[str, LSPInstance] = {}
        self.idle_timeout = idle_timeout
        self.lock: asyncio.Lock = asyncio.Lock()
        self.logger = MultilspyLogger()

        # Start cleanup task
        self._cleanup_task: Optional[asyncio.Task] = None

    async def initialize(self) -> None:
        """Initialize the manager, start background tasks."""
        self._cleanup_task = asyncio.create_task(self._cleanup_loop())

    async def _initialize_lsp_instance(self, project_path: str) -> LSPInstance:
        """Initialize a new LSP instance and wait for it to be ready."""
        config = MultilspyConfig.from_dict({"code_language": Language.RUST})
        lsp = LanguageServer.create(config, self.logger, project_path)
        ctx = lsp.start_server()
        await ctx.__aenter__()

        instance = LSPInstance(project_path=project_path, lsp=lsp, context_manager=ctx)

        # Mark instance as ready after initialization
        instance.ready.set()
        return instance

    async def get_or_create_instance(
        self, project_path: str
    ) -> Tuple[LSPInstance, bool]:
        """
        Get an existing LSP instance or create a new one.

        :param project_path: Absolute path to the Rust project
        :return: Tuple of (LSPInstance, is_new)
        """
        project_path = os.path.abspath(project_path)

        async with self.lock:
            if project_path in self.instances:
                instance = self.instances[project_path]
                instance.last_used = time.time()
                return instance, False

            # Create new instance
            instance = await self._initialize_lsp_instance(project_path)
            self.instances[project_path] = instance
            return instance, True

    async def shutdown_instance(self, project_path: str) -> bool:
        """
        Shutdown a specific LSP instance.

        :param project_path: Absolute path to the Rust project
        :return: True if an instance was shutdown, False otherwise
        """
        project_path = os.path.abspath(project_path)

        async with self.lock:
            if project_path not in self.instances:
                return False

            instance = self.instances.pop(project_path)
            try:
                await instance.context_manager.__aexit__(None, None, None)
            except Exception:
                pass
            return True

    async def shutdown_all(self) -> None:
        """Shutdown all LSP instances."""
        async with self.lock:
            for project_path in list(self.instances.keys()):
                instance = self.instances.pop(project_path)
                try:
                    await instance.context_manager.__aexit__(None, None, None)
                except Exception:
                    pass

        if self._cleanup_task:
            self._cleanup_task.cancel()
            try:
                await self._cleanup_task
            except asyncio.CancelledError:
                pass

    async def _cleanup_loop(self) -> None:
        """Background task that cleans up idle instances."""
        while True:
            await asyncio.sleep(60)  # Check every minute

            now = time.time()
            async with self.lock:
                to_shutdown = []
                for project_path, instance in self.instances.items():
                    if now - instance.last_used > self.idle_timeout:
                        to_shutdown.append(project_path)

                for project_path in to_shutdown:
                    instance = self.instances.pop(project_path)
                    try:
                        await instance.context_manager.__aexit__(None, None, None)
                    except Exception:
                        pass

    async def stop(self) -> None:
        """Stop the manager and cleanup all resources."""
        await self.shutdown_all()


def create_app(manager: LSPManager) -> Quart:
    """Create and configure the Quart application."""
    app = Quart(__name__)

    @app.route("/health", methods=["GET"])
    async def health():
        return jsonify({"status": "ok"})

    @app.route("/instances", methods=["GET"])
    async def list_instances():
        async with manager.lock:
            instances = list(manager.instances.keys())
        return jsonify({"instances": instances})

    @app.route("/start", methods=["POST"])
    async def start():
        data = await request.get_json(silent=True) or {}
        project_path = data.get("project_path")

        if not project_path:
            return jsonify(
                {"status": "error", "message": "project_path is required"}
            ), 400

        if not os.path.isdir(project_path):
            return jsonify(
                {
                    "status": "error",
                    "message": f"Project path does not exist: {project_path}",
                }
            ), 400

        instance, is_new = await manager.get_or_create_instance(project_path)
        # Wait for instance to be fully ready
        await instance.ready.wait()

        return jsonify({"status": "ok", "project_path": project_path, "is_new": is_new})

    @app.route("/shutdown", methods=["POST"])
    async def shutdown():
        data = await request.get_json(silent=True) or {}
        project_path = data.get("project_path")

        if not project_path:
            return jsonify(
                {"status": "error", "message": "project_path is required"}
            ), 400

        success = await manager.shutdown_instance(project_path)
        return jsonify(
            {"status": "ok" if success else "not_found", "project_path": project_path}
        )

    @app.route("/definition", methods=["POST"])
    async def definition():
        data = await request.get_json(silent=True) or {}
        project_path = data.get("project_path")
        file_path = data.get("file_path")
        line = data.get("line")
        column = data.get("column")

        if not all(
            [
                project_path is not None,
                file_path is not None,
                line is not None,
                column is not None,
            ]
        ):
            return jsonify(
                {
                    "status": "error",
                    "message": "project_path, file_path, line, and column are required",
                }
            ), 400

        instance, _ = await manager.get_or_create_instance(project_path)
        # Wait for instance to be ready
        await instance.ready.wait()

        async with instance.lock:
            instance.last_used = time.time()
            lsp_pos = raw_to_lsp_position(line, column)
            result = await instance.lsp.request_definition(
                file_path, lsp_pos["line"], lsp_pos["character"]
            )
            converted = convert_all_locations_to_raw(result)
            return jsonify({"status": "ok", "result": converted})

    @app.route("/type-definition", methods=["POST"])
    async def type_definition():
        data = await request.get_json(silent=True) or {}
        project_path = data.get("project_path")
        file_path = data.get("file_path")
        line = data.get("line")
        column = data.get("column")

        if not all(
            [
                project_path is not None,
                file_path is not None,
                line is not None,
                column is not None,
            ]
        ):
            return jsonify(
                {
                    "status": "error",
                    "message": "project_path, file_path, line, and column are required",
                }
            ), 400

        instance, _ = await manager.get_or_create_instance(project_path)
        await instance.ready.wait()

        async with instance.lock:
            instance.last_used = time.time()
            lsp_pos = raw_to_lsp_position(line, column)
            result = await instance.lsp.request_type_definition(
                file_path, lsp_pos["line"], lsp_pos["character"]
            )
            converted = convert_all_locations_to_raw(result)
            return jsonify({"status": "ok", "result": converted})

    @app.route("/references", methods=["POST"])
    async def references():
        data = await request.get_json(silent=True) or {}
        project_path = data.get("project_path")
        file_path = data.get("file_path")
        line = data.get("line")
        column = data.get("column")

        if not all(
            [
                project_path is not None,
                file_path is not None,
                line is not None,
                column is not None,
            ]
        ):
            return jsonify(
                {
                    "status": "error",
                    "message": "project_path, file_path, line, and column are required",
                }
            ), 400

        instance, _ = await manager.get_or_create_instance(project_path)
        await instance.ready.wait()

        async with instance.lock:
            instance.last_used = time.time()
            lsp_pos = raw_to_lsp_position(line, column)
            result = await instance.lsp.request_references(
                file_path, lsp_pos["line"], lsp_pos["character"]
            )
            converted = convert_all_locations_to_raw(result)
            return jsonify({"status": "ok", "result": converted})

    @app.route("/document-symbols", methods=["POST"])
    async def document_symbols():
        data = await request.get_json(silent=True) or {}
        project_path = data.get("project_path")
        file_path = data.get("file_path")

        if not all([project_path is not None, file_path is not None]):
            return jsonify(
                {
                    "status": "error",
                    "message": "project_path and file_path are required",
                }
            ), 400

        instance, _ = await manager.get_or_create_instance(project_path)
        await instance.ready.wait()

        async with instance.lock:
            instance.last_used = time.time()
            symbols, _ = await instance.lsp.request_document_symbols(file_path)
            converted = convert_document_symbols_to_raw(symbols)
            return jsonify({"status": "ok", "result": converted})

    @app.route("/implementation", methods=["POST"])
    async def implementation():
        data = await request.get_json(silent=True) or {}
        project_path = data.get("project_path")
        file_path = data.get("file_path")
        line = data.get("line")
        column = data.get("column")

        if not all(
            [
                project_path is not None,
                file_path is not None,
                line is not None,
                column is not None,
            ]
        ):
            return jsonify(
                {
                    "status": "error",
                    "message": "project_path, file_path, line, and column are required",
                }
            ), 400

        instance, _ = await manager.get_or_create_instance(project_path)
        await instance.ready.wait()

        async with instance.lock:
            instance.last_used = time.time()
            lsp_pos = raw_to_lsp_position(line, column)
            result = await instance.lsp.request_implementation(
                file_path, lsp_pos["line"], lsp_pos["character"]
            )
            converted = convert_all_locations_to_raw(result)
            return jsonify({"status": "ok", "result": converted})

    @app.route("/incoming-calls", methods=["POST"])
    async def incoming_calls():
        data = await request.get_json(silent=True) or {}
        project_path = data.get("project_path")
        file_path = data.get("file_path")
        line = data.get("line")
        column = data.get("column")

        if not all(
            [
                project_path is not None,
                file_path is not None,
                line is not None,
                column is not None,
            ]
        ):
            return jsonify(
                {
                    "status": "error",
                    "message": "project_path, file_path, line, and column are required",
                }
            ), 400

        instance, _ = await manager.get_or_create_instance(project_path)
        await instance.ready.wait()

        async with instance.lock:
            instance.last_used = time.time()
            lsp_pos = raw_to_lsp_position(line, column)
            prepare_result = await instance.lsp.request_prepare_call_hierarchy(
                file_path, lsp_pos["line"], lsp_pos["character"]
            )

            if not prepare_result:
                return jsonify({"status": "ok", "result": []})

            item = prepare_result[0]
            result = await instance.lsp.request_incoming_calls(file_path, item)

            if result is None:
                return jsonify({"status": "ok", "result": []})

            converted = convert_incoming_calls_to_raw(result)
            return jsonify({"status": "ok", "result": converted})

    @app.route("/outgoing-calls", methods=["POST"])
    async def outgoing_calls():
        data = await request.get_json(silent=True) or {}
        project_path = data.get("project_path")
        file_path = data.get("file_path")
        line = data.get("line")
        column = data.get("column")

        if not all(
            [
                project_path is not None,
                file_path is not None,
                line is not None,
                column is not None,
            ]
        ):
            return jsonify(
                {
                    "status": "error",
                    "message": "project_path, file_path, line, and column are required",
                }
            ), 400

        instance, _ = await manager.get_or_create_instance(project_path)
        await instance.ready.wait()

        async with instance.lock:
            instance.last_used = time.time()
            lsp_pos = raw_to_lsp_position(line, column)
            prepare_result = await instance.lsp.request_prepare_call_hierarchy(
                file_path, lsp_pos["line"], lsp_pos["character"]
            )

            if not prepare_result:
                return jsonify({"status": "ok", "result": []})

            item = prepare_result[0]
            result = await instance.lsp.request_outgoing_calls(file_path, item)

            if result is None:
                return jsonify({"status": "ok", "result": []})

            converted = convert_outgoing_calls_to_raw(result)
            return jsonify({"status": "ok", "result": converted})

    @app.route("/incoming-calls-recursive", methods=["POST"])
    async def incoming_calls_recursive():
        data = await request.get_json(silent=True) or {}
        project_path = data.get("project_path")
        file_path = data.get("file_path")
        line = data.get("line")
        column = data.get("column")
        max_depth = data.get("max_depth", 10)

        if not all(
            [
                project_path is not None,
                file_path is not None,
                line is not None,
                column is not None,
            ]
        ):
            return jsonify(
                {
                    "status": "error",
                    "message": "project_path, file_path, line, and column are required",
                }
            ), 400

        instance, _ = await manager.get_or_create_instance(project_path)
        await instance.ready.wait()

        async with instance.lock:
            instance.last_used = time.time()
            lsp_pos = raw_to_lsp_position(line, column)
            prepare_result = await instance.lsp.request_prepare_call_hierarchy(
                file_path, lsp_pos["line"], lsp_pos["character"]
            )

            if not prepare_result:
                return jsonify({"status": "ok", "result": {}})

            root_item = prepare_result[0]

            # Recursive traversal function
            async def traverse_incoming(
                item: Dict[str, Any],
                current_depth: int,
                visited: set
            ) -> Dict[str, Any]:
                key = get_call_hierarchy_key(item)

                if key in visited or current_depth > max_depth:
                    return {}

                visited.add(key)

                # Convert and store the current item
                converted_item = convert_call_hierarchy_item_to_raw(item)
                item_info = extract_call_hierarchy_item_info(converted_item)

                # Get incoming calls for this item
                incoming_result = await instance.lsp.request_incoming_calls(file_path, item)

                child_keys = []
                result = {}

                if incoming_result:
                    for call in incoming_result:
                        from_item = call.get("from", {})
                        from_key = get_call_hierarchy_key(from_item)

                        # Convert fromRanges
                        from_ranges = call.get("fromRanges", [])
                        converted_ranges = [convert_lsp_range_to_raw(r) for r in from_ranges]

                        child_keys.append({
                            "key": from_key,
                            "fromRanges": converted_ranges
                        })

                        # Recursively traverse children
                        child_result = await traverse_incoming(from_item, current_depth + 1, visited)
                        result.update(child_result)

                # Add current item to result
                result[key] = {
                    "info": item_info,
                    "incoming_calls": child_keys
                }

                return result

            # Start traversal from root item
            visited: set = set()
            root_key = get_call_hierarchy_key(root_item)
            converted_root = convert_call_hierarchy_item_to_raw(root_item)
            root_info = extract_call_hierarchy_item_info(converted_root)

            # First get the root's incoming calls
            root_incoming = await instance.lsp.request_incoming_calls(file_path, root_item)
            root_child_keys = []

            if root_incoming:
                for call in root_incoming:
                    from_item = call.get("from", {})
                    from_key = get_call_hierarchy_key(from_item)
                    from_ranges = call.get("fromRanges", [])
                    converted_ranges = [convert_lsp_range_to_raw(r) for r in from_ranges]
                    root_child_keys.append({
                        "key": from_key,
                        "fromRanges": converted_ranges
                    })

            # Build the result starting with root
            final_result = {
                root_key: {
                    "info": root_info,
                    "incoming_calls": root_child_keys
                }
            }

            # Now traverse all children
            if root_incoming:
                for call in root_incoming:
                    from_item = call.get("from", {})
                    child_result = await traverse_incoming(from_item, 1, visited)
                    final_result.update(child_result)

            return jsonify({"status": "ok", "result": final_result})

    @app.route("/outgoing-calls-recursive", methods=["POST"])
    async def outgoing_calls_recursive():
        data = await request.get_json(silent=True) or {}
        project_path = data.get("project_path")
        file_path = data.get("file_path")
        line = data.get("line")
        column = data.get("column")
        max_depth = data.get("max_depth", 10)

        if not all(
            [
                project_path is not None,
                file_path is not None,
                line is not None,
                column is not None,
            ]
        ):
            return jsonify(
                {
                    "status": "error",
                    "message": "project_path, file_path, line, and column are required",
                }
            ), 400

        instance, _ = await manager.get_or_create_instance(project_path)
        await instance.ready.wait()

        async with instance.lock:
            instance.last_used = time.time()
            lsp_pos = raw_to_lsp_position(line, column)
            prepare_result = await instance.lsp.request_prepare_call_hierarchy(
                file_path, lsp_pos["line"], lsp_pos["character"]
            )

            if not prepare_result:
                return jsonify({"status": "ok", "result": {}})

            root_item = prepare_result[0]

            # Recursive traversal function
            async def traverse_outgoing(
                item: Dict[str, Any],
                current_depth: int,
                visited: set
            ) -> Dict[str, Any]:
                key = get_call_hierarchy_key(item)

                if key in visited or current_depth > max_depth:
                    return {}

                visited.add(key)

                # Convert and store the current item
                converted_item = convert_call_hierarchy_item_to_raw(item)
                item_info = extract_call_hierarchy_item_info(converted_item)

                # Get outgoing calls for this item
                outgoing_result = await instance.lsp.request_outgoing_calls(file_path, item)

                child_keys = []
                result = {}

                if outgoing_result:
                    for call in outgoing_result:
                        to_item = call.get("to", {})
                        to_key = get_call_hierarchy_key(to_item)

                        # Convert fromRanges
                        from_ranges = call.get("fromRanges", [])
                        converted_ranges = [convert_lsp_range_to_raw(r) for r in from_ranges]

                        child_keys.append({
                            "key": to_key,
                            "fromRanges": converted_ranges
                        })

                        # Recursively traverse children
                        child_result = await traverse_outgoing(to_item, current_depth + 1, visited)
                        result.update(child_result)

                # Add current item to result
                result[key] = {
                    "info": item_info,
                    "outgoing_calls": child_keys
                }

                return result

            # Start traversal from root item
            visited: set = set()
            root_key = get_call_hierarchy_key(root_item)
            converted_root = convert_call_hierarchy_item_to_raw(root_item)
            root_info = extract_call_hierarchy_item_info(converted_root)

            # First get the root's outgoing calls
            root_outgoing = await instance.lsp.request_outgoing_calls(file_path, root_item)
            root_child_keys = []

            if root_outgoing:
                for call in root_outgoing:
                    to_item = call.get("to", {})
                    to_key = get_call_hierarchy_key(to_item)
                    from_ranges = call.get("fromRanges", [])
                    converted_ranges = [convert_lsp_range_to_raw(r) for r in from_ranges]
                    root_child_keys.append({
                        "key": to_key,
                        "fromRanges": converted_ranges
                    })

            # Build the result starting with root
            final_result = {
                root_key: {
                    "info": root_info,
                    "outgoing_calls": root_child_keys
                }
            }

            # Now traverse all children
            if root_outgoing:
                for call in root_outgoing:
                    to_item = call.get("to", {})
                    child_result = await traverse_outgoing(to_item, 1, visited)
                    final_result.update(child_result)

            return jsonify({"status": "ok", "result": final_result})

    return app


def get_socket_path() -> str:
    """Get the path to the Unix socket file."""
    temp_dir = tempfile.gettempdir()
    return os.path.join(temp_dir, f"ra-lsp-server-{os.getuid()}.sock")


def get_pid_path() -> str:
    """Get the path to the PID file."""
    temp_dir = tempfile.gettempdir()
    return os.path.join(temp_dir, f"ra-lsp-server-{os.getuid()}.pid")


def is_server_running() -> bool:
    """Check if the server is already running."""
    pid_path = get_pid_path()
    if not os.path.exists(pid_path):
        return False

    try:
        with open(pid_path, "r") as f:
            pid = int(f.read().strip())
        os.kill(pid, 0)  # Check if process is alive
        return True
    except (OSError, ValueError):
        # Clean up stale PID file
        try:
            os.unlink(pid_path)
        except OSError:
            pass
        return False


async def run_server(app: Quart, host: str, port: int, manager: LSPManager) -> None:
    """Run the uvicorn server with proper initialization."""
    # Initialize the manager
    await manager.initialize()

    config = uvicorn.Config(
        app,
        host=host,
        port=port,
        log_level="error",
        access_log=False,
        workers=1,
        timeout_keep_alive=1200,  # 20 minutes in seconds
        limit_concurrency=None,  # No limit on concurrency
    )
    server = uvicorn.Server(config)

    try:
        await server.serve()
    finally:
        await manager.stop()


def start_server(host: str = "127.0.0.1", port: int = 0, daemon: bool = False) -> None:
    """
    Start the LSP server.

    :param host: Host to bind to
    :param port: Port to bind to (0 for random)
    :param daemon: Whether to run as a daemon process
    """
    if daemon:
        # Daemonize the process
        if os.fork():
            sys.exit(0)
        os.setsid()
        if os.fork():
            sys.exit(0)
        sys.stdout.flush()
        sys.stderr.flush()
        with open("/dev/null", "r") as null:
            os.dup2(null.fileno(), sys.stdin.fileno())
            os.dup2(null.fileno(), sys.stdout.fileno())
            os.dup2(null.fileno(), sys.stderr.fileno())

    # Write PID file
    pid_path = get_pid_path()
    with open(pid_path, "w") as f:
        f.write(str(os.getpid()))

    manager = LSPManager()
    app = create_app(manager)

    # Disable logging
    log = logging.getLogger("quart")
    log.setLevel(logging.ERROR)
    log = logging.getLogger("uvicorn")
    log.setLevel(logging.ERROR)
    log = logging.getLogger("uvicorn.access")
    log.setLevel(logging.ERROR)

    try:
        # Find a free port if 0 was specified
        if port == 0:
            import socket

            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.bind((host, 0))
                port = s.getsockname()[1]

        # Write port file
        port_path = get_pid_path().replace(".pid", ".port")
        with open(port_path, "w") as f:
            f.write(str(port))

        # Run the server
        asyncio.run(run_server(app, host, port, manager))

    finally:
        try:
            os.unlink(pid_path)
            os.unlink(port_path)
        except OSError:
            pass


def stop_server() -> bool:
    """
    Stop the running server.

    :return: True if server was stopped, False otherwise
    """
    pid_path = get_pid_path()
    if not os.path.exists(pid_path):
        return False

    try:
        with open(pid_path, "r") as f:
            pid = int(f.read().strip())
        os.kill(pid, 15)  # SIGTERM

        # Wait for process to die
        for _ in range(10):
            try:
                os.kill(pid, 0)
                time.sleep(0.1)
            except OSError:
                break

        # Clean up files
        for ext in [".pid", ".port", ".sock"]:
            path = get_pid_path().replace(".pid", ext)
            try:
                os.unlink(path)
            except OSError:
                pass

        return True
    except (OSError, ValueError):
        return False


def get_server_address() -> Optional[Tuple[str, int]]:
    """
    Get the address of the running server.

    :return: Tuple of (host, port) or None if server not running
    """
    port_path = get_pid_path().replace(".pid", ".port")
    if not os.path.exists(port_path):
        return None

    try:
        with open(port_path, "r") as f:
            port = int(f.read().strip())
        return ("127.0.0.1", port)
    except (OSError, ValueError):
        return None
