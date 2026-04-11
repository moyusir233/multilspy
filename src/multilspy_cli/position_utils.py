"""
Position conversion utilities between raw file positions (1-based) and LSP positions (0-based).

According to LSP specification:
- LSP uses 0-based line numbers and 0-based UTF-16 character offsets
- Raw file positions use 1-based line numbers and 1-based column numbers
"""
from typing import Dict, Any, List, Optional


def raw_to_lsp_position(raw_line: int, raw_column: int) -> Dict[str, int]:
    """
    Convert a raw file position (1-based) to an LSP position (0-based).

    :param raw_line: 1-based line number
    :param raw_column: 1-based column number
    :return: Dictionary with "line" and "character" keys (0-based)
    """
    return {
        "line": raw_line - 1,
        "character": raw_column - 1
    }


def lsp_to_raw_position(lsp_line: int, lsp_character: int) -> Dict[str, int]:
    """
    Convert an LSP position (0-based) to a raw file position (1-based).

    :param lsp_line: 0-based line number
    :param lsp_character: 0-based character offset
    :return: Dictionary with "line" and "column" keys (1-based)
    """
    return {
        "line": lsp_line + 1,
        "column": lsp_character + 1
    }


def convert_lsp_location_to_raw(lsp_location: Dict[str, Any]) -> Dict[str, Any]:
    """
    Convert an LSP Location object's positions from 0-based to 1-based.

    :param lsp_location: LSP Location dictionary with "range" field
    :return: New dictionary with converted positions
    """
    result = dict(lsp_location)

    if "range" in result:
        result["range"] = convert_lsp_range_to_raw(result["range"])

    return result


def convert_lsp_range_to_raw(lsp_range: Dict[str, Any]) -> Dict[str, Any]:
    """
    Convert an LSP Range object's positions from 0-based to 1-based.

    :param lsp_range: LSP Range dictionary with "start" and "end" fields
    :return: New dictionary with converted positions
    """
    result = dict(lsp_range)

    if "start" in result:
        result["start"] = lsp_to_raw_position(
            result["start"]["line"],
            result["start"]["character"]
        )

    if "end" in result:
        result["end"] = lsp_to_raw_position(
            result["end"]["line"],
            result["end"]["character"]
        )

    return result


def convert_all_locations_to_raw(locations: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    """
    Convert a list of LSP Location objects to use 1-based positions.

    :param locations: List of LSP Location dictionaries
    :return: List of converted dictionaries
    """
    return [convert_lsp_location_to_raw(loc) for loc in locations]


def convert_call_hierarchy_item_to_raw(item: Dict[str, Any]) -> Dict[str, Any]:
    """
    Convert a CallHierarchyItem's positions from 0-based to 1-based.

    :param item: CallHierarchyItem dictionary
    :return: New dictionary with converted positions
    """
    result = dict(item)

    if "range" in result:
        result["range"] = convert_lsp_range_to_raw(result["range"])

    if "selectionRange" in result:
        result["selectionRange"] = convert_lsp_range_to_raw(result["selectionRange"])

    return result


def convert_incoming_calls_to_raw(calls: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    """
    Convert incoming calls (CallHierarchyIncomingCall) to use 1-based positions.

    :param calls: List of CallHierarchyIncomingCall dictionaries
    :return: List of converted dictionaries
    """
    result = []
    for call in calls:
        converted = dict(call)
        if "from" in converted:
            converted["from"] = convert_call_hierarchy_item_to_raw(converted["from"])
        if "fromRanges" in converted:
            converted["fromRanges"] = [
                convert_lsp_range_to_raw(r) for r in converted["fromRanges"]
            ]
        result.append(converted)
    return result


def convert_outgoing_calls_to_raw(calls: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    """
    Convert outgoing calls (CallHierarchyOutgoingCall) to use 1-based positions.

    :param calls: List of CallHierarchyOutgoingCall dictionaries
    :return: List of converted dictionaries
    """
    result = []
    for call in calls:
        converted = dict(call)
        if "to" in converted:
            converted["to"] = convert_call_hierarchy_item_to_raw(converted["to"])
        if "fromRanges" in converted:
            converted["fromRanges"] = [
                convert_lsp_range_to_raw(r) for r in converted["fromRanges"]
            ]
        result.append(converted)
    return result


def convert_document_symbols_to_raw(symbols: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    """
    Convert document symbols to use 1-based positions.

    :param symbols: List of UnifiedSymbolInformation dictionaries
    :return: List of converted dictionaries
    """
    result = []
    for symbol in symbols:
        converted = dict(symbol)
        if "location" in converted and converted["location"] is not None:
            converted["location"] = convert_lsp_location_to_raw(converted["location"])
        if "range" in converted:
            converted["range"] = convert_lsp_range_to_raw(converted["range"])
        if "selectionRange" in converted:
            converted["selectionRange"] = convert_lsp_range_to_raw(converted["selectionRange"])
        result.append(converted)
    return result


def get_call_hierarchy_key(item: Dict[str, Any]) -> str:
    """
    Get the unique key for a CallHierarchyItem, composed of name, uri, and range.

    :param item: CallHierarchyItem dictionary
    :return: String key in format "name|uri|start_line,start_char-end_line,end_char"
    """
    name = item.get("name", "")
    uri = item.get("uri", "")
    range_info = item.get("range", {})
    start = range_info.get("start", {})
    end = range_info.get("end", {})
    start_line = start.get("line", 0)
    start_char = start.get("character", 0)
    end_line = end.get("line", 0)
    end_char = end.get("character", 0)
    range_str = f"{start_line},{start_char}-{end_line},{end_char}"
    return f"{name}|{uri}|{range_str}"


def extract_call_hierarchy_item_info(item: Dict[str, Any]) -> Dict[str, Any]:
    """
    Extract information from a CallHierarchyItem (excluding name and uri which are used as key).

    :param item: CallHierarchyItem dictionary
    :return: Dictionary with item info (kind, tags, detail, range, selectionRange, data, etc.)
    """
    result = {}
    for key in ["kind", "tags", "detail", "range", "selectionRange", "data"]:
        if key in item:
            result[key] = item[key]
    return result


def convert_recursive_incoming_calls_to_raw(
    calls: List[Dict[str, Any]],
    visited: Optional[set] = None
) -> Dict[str, Any]:
    """
    Convert incoming calls to recursive dictionary format with 1-based positions.

    The result is a dictionary where:
    - Key: "name|uri" from CallHierarchyItem
    - Value: Dictionary with:
      - "info": extracted item info (kind, tags, detail, range, selectionRange, data)
      - "incoming_calls": list of keys for first-level incoming calls

    :param calls: List of CallHierarchyIncomingCall dictionaries
    :param visited: Set of already visited keys to prevent infinite recursion
    :return: Dictionary in recursive format
    """
    if visited is None:
        visited = set()

    result = {}

    for call in calls:
        from_item = call.get("from", {})
        key = get_call_hierarchy_key(from_item)

        if key in visited:
            continue

        # Convert and store the item
        converted_item = convert_call_hierarchy_item_to_raw(from_item)
        item_info = extract_call_hierarchy_item_info(converted_item)

        # Convert fromRanges
        from_ranges = call.get("fromRanges", [])
        converted_ranges = [convert_lsp_range_to_raw(r) for r in from_ranges]

        result[key] = {
            "info": item_info,
            "incoming_calls": converted_ranges
        }

    return result


def convert_recursive_outgoing_calls_to_raw(
    calls: List[Dict[str, Any]],
    visited: Optional[set] = None
) -> Dict[str, Any]:
    """
    Convert outgoing calls to recursive dictionary format with 1-based positions.

    The result is a dictionary where:
    - Key: "name|uri" from CallHierarchyItem
    - Value: Dictionary with:
      - "info": extracted item info (kind, tags, detail, range, selectionRange, data)
      - "outgoing_calls": list of keys for first-level outgoing calls

    :param calls: List of CallHierarchyOutgoingCall dictionaries
    :param visited: Set of already visited keys to prevent infinite recursion
    :return: Dictionary in recursive format
    """
    if visited is None:
        visited = set()

    result = {}

    for call in calls:
        to_item = call.get("to", {})
        key = get_call_hierarchy_key(to_item)

        if key in visited:
            continue

        # Convert and store the item
        converted_item = convert_call_hierarchy_item_to_raw(to_item)
        item_info = extract_call_hierarchy_item_info(converted_item)

        # Convert fromRanges
        from_ranges = call.get("fromRanges", [])
        converted_ranges = [convert_lsp_range_to_raw(r) for r in from_ranges]

        result[key] = {
            "info": item_info,
            "outgoing_calls": converted_ranges
        }

    return result
