#!/usr/bin/env python3
# Copyright 2026 Cortex Contributors
# SPDX-License-Identifier: Apache-2.0
"""Conformance test runner for Cortex clients.

Starts a local HTTP server with test fixtures, starts Cortex, runs test cases
against both Python and TypeScript clients, and reports results.

Usage:
    python runner.py [--client python|typescript|both] [--suite map|query|pathfind|all]
"""
from __future__ import annotations

import argparse
import http.server
import json
import os
import subprocess
import sys
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Fixture server
# ---------------------------------------------------------------------------

FIXTURE_HTML = {
    "/": "<html><head><title>Home</title></head><body>"
    '<h1>Test Site</h1><a href="/about">About</a>'
    '<a href="/products">Products</a>'
    '<a href="/contact">Contact</a></body></html>',
    "/about": "<html><head><title>About</title></head><body>"
    "<h1>About Us</h1><p>We are a test site.</p>"
    '<a href="/">Home</a></body></html>',
    "/products": "<html><head><title>Products</title></head><body>"
    "<h1>Products</h1>"
    '<a href="/products/widget">Widget</a>'
    '<a href="/products/gadget">Gadget</a>'
    '<a href="/">Home</a></body></html>',
    "/products/widget": "<html><head><title>Widget</title></head><body>"
    "<h1>Widget</h1><p>Price: $29.99</p>"
    '<a href="/products">Back</a></body></html>',
    "/products/gadget": "<html><head><title>Gadget</title></head><body>"
    "<h1>Gadget</h1><p>Price: $49.99</p>"
    '<a href="/products">Back</a></body></html>',
    "/contact": "<html><head><title>Contact</title></head><body>"
    '<h1>Contact</h1><form><input name="email"/>'
    '<button type="submit">Send</button></form>'
    '<a href="/">Home</a></body></html>',
    "/sitemap.xml": '<?xml version="1.0" encoding="UTF-8"?>'
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">'
    "<url><loc>http://localhost:{{PORT}}/</loc></url>"
    "<url><loc>http://localhost:{{PORT}}/about</loc></url>"
    "<url><loc>http://localhost:{{PORT}}/products</loc></url>"
    "<url><loc>http://localhost:{{PORT}}/products/widget</loc></url>"
    "<url><loc>http://localhost:{{PORT}}/products/gadget</loc></url>"
    "<url><loc>http://localhost:{{PORT}}/contact</loc></url>"
    "</urlset>",
    "/robots.txt": "User-agent: *\nAllow: /\nSitemap: http://localhost:{{PORT}}/sitemap.xml\n",
}


class FixtureHandler(http.server.BaseHTTPRequestHandler):
    port: int = 0

    def do_GET(self) -> None:
        content = FIXTURE_HTML.get(self.path)
        if content:
            body = content.replace("{{PORT}}", str(self.port))
            self.send_response(200)
            if self.path.endswith(".xml"):
                self.send_header("Content-Type", "application/xml")
            else:
                self.send_header("Content-Type", "text/html")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body.encode())
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format: str, *args: Any) -> None:
        pass  # Suppress logs


# ---------------------------------------------------------------------------
# Test infrastructure
# ---------------------------------------------------------------------------


@dataclass
class TestResult:
    case_id: str
    passed: bool
    message: str = ""


@dataclass
class SuiteResult:
    suite: str
    client: str
    results: list[TestResult] = field(default_factory=list)

    @property
    def passed(self) -> int:
        return sum(1 for r in self.results if r.passed)

    @property
    def failed(self) -> int:
        return sum(1 for r in self.results if not r.passed)


def check_assertion(
    value: Any, assertion: dict[str, Any]
) -> tuple[bool, str]:
    """Evaluate a single assertion against a value."""
    op = assertion["op"]
    expected = assertion["value"]

    if assertion.get("if_not_null") and value is None:
        return True, "skipped (null)"

    if op == "eq":
        ok = value == expected
    elif op == "gte":
        ok = value is not None and value >= expected
    elif op == "lte":
        ok = value is not None and value <= expected
    elif op == "in":
        ok = value in expected
    elif op == "all_eq":
        ok = isinstance(value, list) and all(v == expected for v in value)
    else:
        return False, f"unknown op: {op}"

    if ok:
        return True, ""
    return False, f"expected {op} {expected}, got {value}"


def extract_field(data: Any, field_path: str) -> Any:
    """Extract a field from response data using a simple path expression."""
    if field_path == "length":
        return len(data) if isinstance(data, list) else 0
    if field_path == "result_type":
        return "null" if data is None else "path"
    if field_path.startswith("[*]."):
        attr = field_path[4:]
        if isinstance(data, list):
            return [item.get(attr) if isinstance(item, dict) else getattr(item, attr, None) for item in data]
        return []
    if field_path.startswith("nodes["):
        idx = int(field_path.split("[")[1].split("]")[0])
        if isinstance(data, dict) and "nodes" in data:
            nodes = data["nodes"]
            return nodes[idx] if idx < len(nodes) else None
        return None
    if isinstance(data, dict):
        return data.get(field_path)
    return getattr(data, field_path, None)


# ---------------------------------------------------------------------------
# Python client runner
# ---------------------------------------------------------------------------


def run_python_case(
    case: dict[str, Any], port: int
) -> TestResult:
    """Execute a single test case using the Python client."""
    try:
        import cortex_client

        action = case["action"]
        method = action["method"]
        args = {
            k: (v.replace("{{PORT}}", str(port)) if isinstance(v, str) else v)
            for k, v in action["args"].items()
        }

        if method == "map":
            result = cortex_client.map(
                args["domain"],
                max_nodes=args.get("max_nodes", 50000),
                max_render=args.get("max_render", 200),
                max_time_ms=args.get("max_time_ms", 10000),
            )
            data: Any = {
                "node_count": result.node_count,
                "edge_count": result.edge_count,
                "domain": result.domain,
            }
        elif method == "query":
            sm = cortex_client.map(args["domain"], max_render=5)
            matches = sm.filter(
                page_type=args.get("page_type"),
                limit=args.get("limit", 100),
            )
            data = [
                {"index": m.index, "url": m.url, "page_type": m.page_type}
                for m in matches
            ]
        elif method == "pathfind":
            sm = cortex_client.map(args["domain"], max_render=5)
            path = sm.pathfind(
                args["from_node"],
                args["to_node"],
                minimize=args.get("minimize", "hops"),
            )
            if path is None:
                data = None
            else:
                data = {
                    "nodes": path.nodes,
                    "hops": path.hops,
                    "total_weight": path.total_weight,
                }
        else:
            return TestResult(case["id"], False, f"unknown method: {method}")

        for assertion in case.get("assertions", []):
            field_path = assertion["field"]
            value = extract_field(data, field_path)
            ok, msg = check_assertion(value, assertion)
            if not ok:
                return TestResult(
                    case["id"], False, f"assertion failed on {field_path}: {msg}"
                )

        return TestResult(case["id"], True)

    except Exception as e:
        return TestResult(case["id"], False, f"exception: {e}")


# ---------------------------------------------------------------------------
# TypeScript client runner
# ---------------------------------------------------------------------------


def run_typescript_case(
    case: dict[str, Any], port: int
) -> TestResult:
    """Execute a single test case via TypeScript subprocess."""
    try:
        action = case["action"]
        script = _build_ts_script(action, port)
        ts_dir = Path(__file__).parent.parent / "typescript"

        proc = subprocess.run(
            ["node", "-e", script],
            cwd=str(ts_dir),
            capture_output=True,
            text=True,
            timeout=30,
        )

        if proc.returncode != 0:
            return TestResult(case["id"], False, f"node error: {proc.stderr.strip()}")

        data = json.loads(proc.stdout.strip()) if proc.stdout.strip() else None

        for assertion in case.get("assertions", []):
            field_path = assertion["field"]
            value = extract_field(data, field_path)
            ok, msg = check_assertion(value, assertion)
            if not ok:
                return TestResult(
                    case["id"], False, f"assertion failed on {field_path}: {msg}"
                )

        return TestResult(case["id"], True)

    except subprocess.TimeoutExpired:
        return TestResult(case["id"], False, "timeout")
    except Exception as e:
        return TestResult(case["id"], False, f"exception: {e}")


def _build_ts_script(action: dict[str, Any], port: int) -> str:
    """Build a Node.js script that calls the TS client and prints JSON."""
    method = action["method"]
    args = action["args"]
    domain = args.get("domain", "").replace("{{PORT}}", str(port))

    if method == "map":
        return f"""
const {{ map }} = require('./dist/index');
(async () => {{
  const sm = await map('{domain}', {{
    maxNodes: {args.get('max_nodes', 50000)},
    maxRender: {args.get('max_render', 200)},
    maxTimeMs: {args.get('max_time_ms', 10000)},
  }});
  console.log(JSON.stringify({{
    node_count: sm.nodeCount,
    edge_count: sm.edgeCount,
    domain: sm.domain,
  }}));
}})().catch(e => {{ console.error(e.message); process.exit(1); }});
"""
    elif method == "query":
        pt = args.get("page_type", "undefined")
        return f"""
const {{ map }} = require('./dist/index');
(async () => {{
  const sm = await map('{domain}', {{ maxRender: 5 }});
  const matches = await sm.filter({{
    pageType: {pt},
    limit: {args.get('limit', 100)},
  }});
  console.log(JSON.stringify(matches.map(m => ({{
    index: m.index, url: m.url, page_type: m.pageType,
  }}))));
}})().catch(e => {{ console.error(e.message); process.exit(1); }});
"""
    elif method == "pathfind":
        return f"""
const {{ map }} = require('./dist/index');
(async () => {{
  const sm = await map('{domain}', {{ maxRender: 5 }});
  const path = await sm.pathfind({args['from_node']}, {args['to_node']}, {{
    minimize: '{args.get('minimize', 'hops')}',
  }});
  if (path === null) {{
    console.log('null');
  }} else {{
    console.log(JSON.stringify({{
      nodes: path.nodes,
      hops: path.hops,
      total_weight: path.totalWeight,
    }}));
  }}
}})().catch(e => {{ console.error(e.message); process.exit(1); }});
"""
    return 'console.error("unknown method"); process.exit(1);'


# ---------------------------------------------------------------------------
# Main runner
# ---------------------------------------------------------------------------


def load_suite(name: str) -> dict[str, Any]:
    """Load a test suite JSON file."""
    suite_path = Path(__file__).parent / f"test_{name}.json"
    with open(suite_path) as f:
        return json.load(f)


def run_suite(
    suite_data: dict[str, Any],
    client: str,
    port: int,
) -> SuiteResult:
    """Run all cases in a suite for a specific client."""
    suite_name = suite_data["suite"]
    result = SuiteResult(suite=suite_name, client=client)

    for case in suite_data["cases"]:
        case_text = json.dumps(case)
        case = json.loads(case_text.replace("{{PORT}}", str(port)))

        if client == "python":
            tr = run_python_case(case, port)
        else:
            tr = run_typescript_case(case, port)
        result.results.append(tr)

    return result


def main() -> int:
    parser = argparse.ArgumentParser(description="Cortex conformance test runner")
    parser.add_argument(
        "--client",
        choices=["python", "typescript", "both"],
        default="both",
    )
    parser.add_argument(
        "--suite",
        choices=["map", "query", "pathfind", "all"],
        default="all",
    )
    parser.add_argument("--port", type=int, default=0)
    args = parser.parse_args()

    # Start fixture server
    server = http.server.HTTPServer(("127.0.0.1", args.port), FixtureHandler)
    port = server.server_address[1]
    FixtureHandler.port = port
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    print(f"Fixture server on port {port}")

    # Determine suites
    suite_names = ["map", "query", "pathfind"] if args.suite == "all" else [args.suite]
    clients = ["python", "typescript"] if args.client == "both" else [args.client]

    all_results: list[SuiteResult] = []
    for suite_name in suite_names:
        suite_data = load_suite(suite_name)
        for client in clients:
            print(f"\n--- {suite_name} / {client} ---")
            sr = run_suite(suite_data, client, port)
            all_results.append(sr)
            for tr in sr.results:
                status = "PASS" if tr.passed else "FAIL"
                msg = f"  {status}: {tr.case_id}"
                if tr.message:
                    msg += f" ({tr.message})"
                print(msg)

    # Summary
    total_pass = sum(sr.passed for sr in all_results)
    total_fail = sum(sr.failed for sr in all_results)
    print(f"\n{'='*40}")
    print(f"Total: {total_pass} passed, {total_fail} failed")

    server.shutdown()
    return 1 if total_fail > 0 else 0


if __name__ == "__main__":
    sys.exit(main())
