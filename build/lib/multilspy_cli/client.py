"""
Client for communicating with the LSP server.
"""

import json
import os
import sys
from typing import Dict, Any, Optional, List

import requests
import logging

log = logging.getLogger(__name__)

logging.basicConfig(level=logging.INFO)


from .server import is_server_running, get_server_address, start_server, stop_server


class LSPClientError(Exception):
    """Error raised by the LSP client."""

    pass


class LSPClient:
    """Client for communicating with the LSP server."""

    def __init__(self, logger: logging.Logger, project_path: Optional[str] = None):
        """
        Initialize the LSP client.

        :param project_path: Path to the Rust project (defaults to current directory)
        """
        if project_path is None:
            project_path = os.getcwd()
        self.project_path = os.path.abspath(project_path)
        self.logger = logger
        self._ensure_server_running()

    def _ensure_server_running(self) -> None:
        """Ensure the server is running, start it if not."""
        if not is_server_running():
            # Start the server in daemon mode
            import subprocess

            subprocess.Popen(
                [sys.executable, "-m", "multilspy_cli.cli", "server", "start"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True,
            )
            # Wait for server to start
            import time

            for _ in range(50):
                if is_server_running():
                    break
                time.sleep(0.1)
            else:
                raise LSPClientError("Failed to start server")

        addr = get_server_address()
        if addr is None:
            raise LSPClientError("Server running but could not get address")
        self.base_url = f"http://{addr[0]}:{addr[1]}"

    def _request(
        self, endpoint: str, data: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """
        Send a request to the server.

        :param endpoint: API endpoint
        :param data: Request data
        :return: Response data
        """
        if data is None:
            data = {}
        data.setdefault("project_path", self.project_path)

        self.logger.debug(f"Requesting {endpoint} with data: {data}")

        try:
            response = requests.post(
                f"{self.base_url}{endpoint}", json=data, timeout=1200
            )
            response.raise_for_status()
            result = response.json()

            if result.get("status") == "error":
                raise LSPClientError(result.get("message", "Unknown error"))

            return result
        except requests.RequestException as e:
            raise LSPClientError(f"Request failed: {e}") from e

    def start_server_for_project(self) -> Dict[str, Any]:
        """
        Start the LSP server for the current project.

        :return: Response data
        """
        return self._request("/start", {"project_path": self.project_path})

    def shutdown_server_for_project(self) -> Dict[str, Any]:
        """
        Shutdown the LSP server for the current project.

        :return: Response data
        """
        return self._request("/shutdown", {"project_path": self.project_path})

    def definition(
        self, file_path: str, line: int, column: int
    ) -> List[Dict[str, Any]]:
        """
        Get the definition for a symbol.

        :param file_path: Relative path to the file
        :param line: 1-based line number
        :param column: 1-based column number
        :return: List of locations
        """
        result = self._request(
            "/definition", {"file_path": file_path, "line": line, "column": column}
        )
        return result.get("result", [])

    def type_definition(
        self, file_path: str, line: int, column: int
    ) -> List[Dict[str, Any]]:
        """
        Get the type definition for a symbol.

        :param file_path: Relative path to the file
        :param line: 1-based line number
        :param column: 1-based column number
        :return: List of locations
        """
        result = self._request(
            "/type-definition", {"file_path": file_path, "line": line, "column": column}
        )
        return result.get("result", [])

    def references(
        self, file_path: str, line: int, column: int
    ) -> List[Dict[str, Any]]:
        """
        Get references to a symbol.

        :param file_path: Relative path to the file
        :param line: 1-based line number
        :param column: 1-based column number
        :return: List of locations
        """
        result = self._request(
            "/references", {"file_path": file_path, "line": line, "column": column}
        )
        return result.get("result", [])

    def document_symbols(self, file_path: str) -> List[Dict[str, Any]]:
        """
        Get all symbols in a document.

        :param file_path: Relative path to the file
        :return: List of symbols
        """
        result = self._request("/document-symbols", {"file_path": file_path})
        return result.get("result", [])

    def implementation(
        self, file_path: str, line: int, column: int
    ) -> List[Dict[str, Any]]:
        """
        Get implementations of a symbol.

        :param file_path: Relative path to the file
        :param line: 1-based line number
        :param column: 1-based column number
        :return: List of locations
        """
        result = self._request(
            "/implementation", {"file_path": file_path, "line": line, "column": column}
        )
        return result.get("result", [])

    def incoming_calls(
        self, file_path: str, line: int, column: int
    ) -> List[Dict[str, Any]]:
        """
        Get incoming calls to a function.

        :param file_path: Relative path to the file
        :param line: 1-based line number
        :param column: 1-based column number
        :return: List of incoming calls
        """
        result = self._request(
            "/incoming-calls", {"file_path": file_path, "line": line, "column": column}
        )
        return result.get("result", [])

    def outgoing_calls(
        self, file_path: str, line: int, column: int
    ) -> List[Dict[str, Any]]:
        """
        Get outgoing calls from a function.

        :param file_path: Relative path to the file
        :param line: 1-based line number
        :param column: 1-based column number
        :return: List of outgoing calls
        """
        result = self._request(
            "/outgoing-calls", {"file_path": file_path, "line": line, "column": column}
        )
        return result.get("result", [])

    def incoming_calls_recursive(
        self, file_path: str, line: int, column: int, max_depth: int = 10
    ) -> Dict[str, Any]:
        """
        Get recursive incoming calls to a function.

        :param file_path: Relative path to the file
        :param line: 1-based line number
        :param column: 1-based column number
        :param max_depth: Maximum recursion depth (default: 10)
        :return: Dictionary with call hierarchy, keys are "name|uri|range",
                 values contain "info" and "incoming_calls"
        """
        result = self._request(
            "/incoming-calls-recursive",
            {
                "file_path": file_path,
                "line": line,
                "column": column,
                "max_depth": max_depth
            }
        )
        return result.get("result", {})

    def outgoing_calls_recursive(
        self, file_path: str, line: int, column: int, max_depth: int = 10
    ) -> Dict[str, Any]:
        """
        Get recursive outgoing calls from a function.

        :param file_path: Relative path to the file
        :param line: 1-based line number
        :param column: 1-based column number
        :param max_depth: Maximum recursion depth (default: 10)
        :return: Dictionary with call hierarchy, keys are "name|uri|range",
                 values contain "info" and "outgoing_calls"
        """
        result = self._request(
            "/outgoing-calls-recursive",
            {
                "file_path": file_path,
                "line": line,
                "column": column,
                "max_depth": max_depth
            }
        )
        return result.get("result", {})


def start_server_command(daemon: bool = True) -> None:
    """
    Start the server command.

    :param daemon: Whether to run as a daemon
    """
    if is_server_running():
        print(json.dumps({"status": "already_running"}))
        return

    start_server(daemon=daemon)


def stop_server_command() -> None:
    """Stop the server command."""
    success = stop_server()
    print(json.dumps({"status": "ok" if success else "not_running"}))


def status_command() -> None:
    """Show server status."""
    running = is_server_running()
    addr = get_server_address() if running else None
    print(
        json.dumps(
            {
                "status": "running" if running else "stopped",
                "address": f"{addr[0]}:{addr[1]}" if addr else None,
            }
        )
    )
