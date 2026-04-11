"""
Rust Analyzer LSP CLI - A command-line interface for Rust code analysis using LSP.
"""

from .position_utils import (
    raw_to_lsp_position,
    lsp_to_raw_position,
    convert_lsp_location_to_raw,
    convert_lsp_range_to_raw,
    convert_all_locations_to_raw,
    convert_call_hierarchy_item_to_raw,
    convert_incoming_calls_to_raw,
    convert_outgoing_calls_to_raw,
    convert_document_symbols_to_raw
)
from .client import LSPClient, LSPClientError
from .server import (
    LSPManager,
    LSPInstance,
    start_server,
    stop_server,
    is_server_running,
    get_server_address
)

__version__ = "0.1.0"
__all__ = [
    # Position utilities
    "raw_to_lsp_position",
    "lsp_to_raw_position",
    "convert_lsp_location_to_raw",
    "convert_lsp_range_to_raw",
    "convert_all_locations_to_raw",
    "convert_call_hierarchy_item_to_raw",
    "convert_incoming_calls_to_raw",
    "convert_outgoing_calls_to_raw",
    "convert_document_symbols_to_raw",
    # Client
    "LSPClient",
    "LSPClientError",
    # Server
    "LSPManager",
    "LSPInstance",
    "start_server",
    "stop_server",
    "is_server_running",
    "get_server_address",
]
