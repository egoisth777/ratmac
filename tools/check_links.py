#!/usr/bin/env python3
"""Working-rule shape check: resolve every relative markdown link under .arca/
and confirm each integrated issue's accepted requirement IDs exist in the goal.

Contributor tool, not a product gate: PGE-001 mechanizes this inside `rtm`.
Run: python tools/check_links.py
"""
from __future__ import annotations

import io
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LINK = re.compile(r"\[[^\]]*\]\(([^)]+)\)")
REQ_ID = re.compile(r"^\| `([A-Z]{3}-\d{3})`")
STATUS = re.compile(r'status:\s*"([a-z]+)"')


def read(path: str) -> str:
    return io.open(path, encoding="utf-8").read()


def markdown_files(rel_root: str):
    for base, _dirs, names in os.walk(os.path.join(ROOT, rel_root)):
        for name in names:
            if name.endswith(".md"):
                yield os.path.join(base, name)


def main() -> int:
    failures: list[str] = []

    for path in markdown_files(".arca"):
        text = read(path)
        for target in LINK.findall(text):
            if target.startswith(("http://", "https://", "#", "mailto:")):
                continue
            clean = target.split("#", 1)[0]
            if not clean:
                continue
            resolved = os.path.normpath(os.path.join(os.path.dirname(path), clean))
            if not os.path.exists(resolved):
                failures.append(
                    f"dangling link: {os.path.relpath(path, ROOT)} -> {target}"
                )

    goal_spec = read(os.path.join(ROOT, ".arca/current/spec.md"))
    issue_root = os.path.join(ROOT, ".arca/issue")
    for entry in sorted(os.listdir(issue_root)):
        folder = os.path.join(issue_root, entry)
        if entry == "archive" or not os.path.isdir(folder):
            continue
        expected = {"index.md", "spec.md", "design.md", "test-plan.md", "ubi-lang.md"}
        present = {n for n in os.listdir(folder) if n.endswith(".md")}
        if present != expected:
            failures.append(f"{entry}: five-file shape broken: {sorted(present)}")
            continue
        index_text = read(os.path.join(folder, "index.md"))
        match = STATUS.search(index_text)
        status = match.group(1) if match else "<none>"
        if status not in {"integrated", "rejected"}:
            failures.append(f"{entry}: status {status!r} is neither integrated nor rejected")
        if status != "integrated":
            continue
        for line in read(os.path.join(folder, "spec.md")).splitlines():
            found = REQ_ID.match(line)
            if not found:
                continue
            if "| accepted |" not in line:
                continue
            req = found.group(1)
            if f"| {req} |" not in goal_spec:
                failures.append(f"{entry}: accepted {req} missing from goal spec")

    for failure in failures:
        print(f"FAIL {failure}")
    if failures:
        print(f"{len(failures)} failure(s)")
        return 1
    print("intake shape check: all links resolve, all accepted requirement IDs present")
    return 0


if __name__ == "__main__":
    sys.exit(main())
