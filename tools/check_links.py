#!/usr/bin/env python3
"""Working-rule shape check: resolve live relative Markdown links under .arca/
and verify issue dispositions, physical carriers, and accepted goal IDs across
intake, deferred, and archive.

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


# Historical roots: archived records are preserved byte-for-byte (AOI-002,
# EXT-003 historical allowlist), so their links are frozen provenance, not live
# routing. The archive-preservation oracle owns them; link resolution does not.
HISTORICAL = (
    os.path.join(".arca", "issue", "archive"),
    os.path.join(".arca", "ticket", "archive"),
)


def is_historical(path: str) -> bool:
    rel = os.path.relpath(path, ROOT)
    return any(rel.startswith(root + os.sep) for root in HISTORICAL)


def markdown_files(rel_root: str):
    for base, _dirs, names in os.walk(os.path.join(ROOT, rel_root)):
        for name in names:
            if name.endswith(".md"):
                path = os.path.join(base, name)
                if not is_historical(path):
                    yield path


def issue_rows(text: str):
    """Yield (requirement id, exact disposition) from issue requirement tables."""
    for line in text.splitlines():
        if not line.lstrip().startswith("|"):
            continue
        cells = [cell.strip().strip("`") for cell in line.strip().strip("|").split("|")]
        if not cells or not re.fullmatch(r"[A-Z]+-\d+", cells[0]):
            continue
        disposition = next(
            (
                cell
                for cell in cells[1:]
                if cell in {"accepted", "rejected", "duplicate", "deferred"}
            ),
            None,
        )
        if disposition is not None:
            yield cells[0], disposition


def issue_bundles():
    issue_root = os.path.join(ROOT, ".arca", "issue")
    for bucket, location in (
        ("", "intake"),
        ("deferred", "deferred"),
        ("archive", "archive"),
    ):
        directory = os.path.join(issue_root, bucket)
        if not os.path.isdir(directory):
            continue
        for entry in sorted(os.listdir(directory)):
            folder = os.path.join(directory, entry)
            if entry.startswith("i-") and os.path.isdir(folder):
                yield entry, folder, location


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

    goal_spec = read(os.path.join(ROOT, ".arca/goal/spec.md"))
    working_authority = read(os.path.join(ROOT, ".arca/schema.md"))
    expected = {"index.md", "spec.md", "design.md", "test-plan.md", "ubi-lang.md"}
    seen: dict[str, str] = {}
    for entry, folder, location in issue_bundles():
        shown = os.path.relpath(folder, ROOT)
        if entry in seen:
            failures.append(
                f"{shown}: issue id {entry} duplicates carrier {seen[entry]}"
            )
        else:
            seen[entry] = shown

        present = {name for name in os.listdir(folder) if name.endswith(".md")}
        if present != expected:
            failures.append(f"{shown}: five-file shape broken: {sorted(present)}")
            continue
        index_text = read(os.path.join(folder, "index.md"))
        match = STATUS.search(index_text)
        status = match.group(1) if match else "<none>"
        rows = list(issue_rows(read(os.path.join(folder, "spec.md"))))
        accepted = {req for req, disposition in rows if disposition == "accepted"}
        duplicate = {req for req, disposition in rows if disposition == "duplicate"}
        deferred = {req for req, disposition in rows if disposition == "deferred"}

        if location == "intake":
            if deferred:
                failures.append(
                    f"{shown}: deferred asks require .arca/issue/deferred and status deferred"
                )
            if status not in {"integrated", "rejected"}:
                failures.append(
                    f"{shown}: intake status {status!r} is neither integrated nor rejected"
                )
        elif location == "deferred":
            if status != "deferred":
                failures.append(
                    f"{shown}: deferred location requires status 'deferred', found {status!r}"
                )
            if not deferred:
                failures.append(f"{shown}: deferred carrier has no deferred ask")
        else:
            if status not in {"integrated", "rejected"}:
                failures.append(
                    f"{shown}: archive requires integrated or rejected, found {status!r}"
                )
            if deferred:
                failures.append(f"{shown}: archived issue still has deferred asks")

        if status == "integrated" and not (accepted or duplicate):
            failures.append(
                f"{shown}: integrated issue has no accepted or duplicate ask"
            )
        if status == "rejected" and accepted:
            failures.append(f"{shown}: rejected issue still has accepted asks")
        if status in {"integrated", "deferred"}:
            for req in sorted(accepted):
                if f"| {req} |" in goal_spec:
                    continue
                if f"### {req} " in working_authority:
                    continue
                failures.append(
                    f"{shown}: accepted {req} resolves to neither a goal spec row"
                    f" nor a working-authority requirement heading"
                )

    for failure in failures:
        print(f"FAIL {failure}")
    if failures:
        print(f"{len(failures)} failure(s)")
        return 1
    print("intake shape check: all links resolve, all accepted requirement IDs present")
    return 0


if __name__ == "__main__":
    sys.exit(main())
