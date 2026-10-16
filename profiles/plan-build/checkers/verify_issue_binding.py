#!/usr/bin/env python3
"""Selected-issue binding validation (WCP-003): resolve one bound turn's
item and issue bindings against the workflow's own records, read-only.

The `command_exit` guard row the ticket class prepends in every working
State runs this validator with its working directory the addressed Run's
workspace. It resolves, in order: the Engine root exactly as the Engine
does (ENS-002); the live spawn-ledger entry whose recorded workspace is
that working directory - live is the ledger's own meaning,
`abandoned = false` (PCR-007); the entry's `item` and `issue` binding
values; the declared `issue`, `residual`, and `ticket` roots; the item
record and its `residual-ids` records' `goal-requirement-ref` values; the
selected issue bundle at one location of the three-location issue
namespace (PGE-001); and the item's declared check identifiers. Every
requirement must occur verbatim in the selected issue's `spec.md` and
every declared check verbatim in its `test-plan.md` (PGE-001, PCR-008).

The validator writes nothing anywhere (PGE-004): its only outputs are its
exit code, one stdout line on success, and one stderr line on refusal,
every line ending in one LF on every platform.
"""
from __future__ import annotations

import os
import subprocess
import sys
import tomllib

PREFIX = "verify_issue_binding"
RUNBOOK_RELATIVE = os.path.join(".ratmac", "ratmac.toml")
ENGINE_DIR = ".ratmac"
LEDGER_NAME = "spawn-ledger"


def refuse(message: str):
    print(f"{PREFIX}: {message}", file=sys.stderr)
    raise SystemExit(1)


# --- the Engine root ------------------------------------------------------------


def git(cwd: str, arguments: list[str]) -> subprocess.CompletedProcess:
    """One read-only Git query with no inherited Git or trace environment."""
    environment = {
        name: value
        for name, value in os.environ.items()
        if name
        not in ("GIT_DIR", "GIT_COMMON_DIR", "GIT_WORK_TREE", "RATMAC_TRACE")
    }
    try:
        return subprocess.run(
            ["git", *arguments],
            cwd=cwd,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=environment,
        )
    except OSError:
        # A missing or unspawnable Git binary is a failed query, never a
        # crash: the caller takes its checkout-local fallback, exactly as
        # the Engine's own resolution maps a failed query (src/root.rs).
        return subprocess.CompletedProcess(["git", *arguments], 1)


def engine_root(cwd: str) -> str:
    """The shared `.ratmac/` the Engine resolves for this checkout (ENS-002):
    in a Git worktree the primary checkout's directory, otherwise the
    working directory's own checkout root. Every failed or unsuitable Git
    query deliberately falls back to the checkout-local root, query for
    query with the Engine's own resolution: inside a work tree, the first
    non-bare record of the porcelain listing supplies the primary checkout,
    and the Git directories the repository reports must name directories,
    not the storage a bare checkout would share as its primary.
    """
    local = os.path.join(cwd, ENGINE_DIR)
    inside = git(cwd, ["rev-parse", "--is-inside-work-tree"])
    if inside.returncode != 0 or inside.stdout.strip() != "true":
        return local
    listed = git(cwd, ["worktree", "list", "--porcelain"])
    if listed.returncode != 0:
        return local
    record = next(
        (
            part
            for part in listed.stdout.split("\n\n")
            if any(part.splitlines())
            and not any(line == "bare" for line in part.splitlines())
        ),
        None,
    )
    primary = (
        next(
            (
                line[len("worktree ") :]
                for line in record.splitlines()
                if line.startswith("worktree ")
            ),
            None,
        )
        if record is not None
        else None
    )
    if primary is None or not os.path.isabs(primary):
        return local
    dirs = git(
        cwd,
        ["rev-parse", "--path-format=absolute", "--git-dir", "--git-common-dir"],
    )
    if dirs.returncode != 0:
        return local
    lines = dirs.stdout.splitlines()
    if len(lines) < 2:
        return local
    git_dir, common_dir = lines[0], lines[1]
    if not os.path.isabs(git_dir) or not os.path.isabs(common_dir) or primary == git_dir:
        return local

    return os.path.join(primary, ENGINE_DIR)


# --- the live ledger entry --------------------------------------------------------


def canonical(directory: str) -> str:
    """One directory's canonical spelling, so the ledger's verbatim `\\\\?\\`
    prefix and Windows letter case never split one directory in two."""
    if directory.startswith("\\\\?\\UNC\\"):
        directory = "\\\\" + directory[len("\\\\?\\UNC\\") :]
    elif directory.startswith("\\\\?\\"):
        directory = directory[len("\\\\?\\") :]
    return os.path.normcase(os.path.realpath(directory))


def live_entries(root: str, workspace: str) -> list[dict]:
    """Every live entry under the Engine root whose recorded workspace is
    `workspace`: the set the join counts, `abandoned = false`."""
    matches = []
    runs = os.path.join(root, "runs")
    try:
        run_ids = sorted(os.listdir(runs))
    except OSError:
        return matches
    for run_id in run_ids:
        ledger = os.path.join(runs, run_id, LEDGER_NAME)
        try:
            with open(ledger, "rb") as handle:
                data = tomllib.load(handle)
        except (OSError, tomllib.TOMLDecodeError):
            continue
        children = data.get("children")
        if not isinstance(children, list):
            continue
        for entry in children:
            if not isinstance(entry, dict) or entry.get("abandoned") is not False:
                continue
            recorded = entry.get("workspace")
            if isinstance(recorded, str) and recorded:
                if canonical(recorded) == canonical(workspace):
                    matches.append(entry)
    return matches


# --- the workflow records ---------------------------------------------------------


def read_record(path: str) -> str:
    with open(path, "r", encoding="utf-8", errors="replace") as handle:
        return handle.read()


def read_or_empty(path: str) -> str:
    """A bundle document's bytes, or nothing when it cannot be read: a
    missing row then simply never occurs."""
    try:
        return read_record(path)
    except OSError:
        return ""


def yaml_field(text: str, key: str) -> str | None:
    """Read `key: "value"` from a record, the landed intake gate's idiom."""
    for line in text.splitlines():
        trimmed = line.strip()
        if trimmed.startswith(key + ":"):
            return trimmed[len(key) + 1 :].strip().strip('"')
    return None


def yaml_list(text: str, key: str) -> list[str]:
    """Read a `key:` block list of `- "value"` entries."""
    values = []
    inside = False
    for line in text.splitlines():
        trimmed = line.rstrip()
        if trimmed.lstrip().startswith(key + ":"):
            inside = True
            continue
        if inside:
            item = trimmed.strip()
            if item.startswith("- "):
                values.append(item[2:].strip().strip('"'))
            elif not item:
                continue
            else:
                break
    return values


def under(root: str, *parts: str) -> str:
    return os.path.join(root, *parts)


def shown(path: str) -> str:
    return path.replace("\\", "/")


def declared_roots(cwd: str) -> dict[str, str]:
    path = os.path.join(cwd, RUNBOOK_RELATIVE)
    relative = shown(RUNBOOK_RELATIVE)
    try:
        with open(path, "rb") as handle:
            data = tomllib.load(handle)
    except OSError as error:
        refuse(f"cannot read the declared roots from {relative}: {error}")
    except tomllib.TOMLDecodeError as error:
        refuse(f"cannot read the declared roots from {relative}: {error}")
    roots = data.get("roots")
    if not isinstance(roots, dict):
        refuse(f"cannot read the declared roots from {relative}: no [roots] table")
    declared = {}
    for name in ("ticket", "issue", "residual"):
        value = roots.get(name)
        if not isinstance(value, str) or not value:
            refuse(
                f"cannot read the declared roots from {relative}: "
                f'the root "{name}" is not declared'
            )
        declared[name] = value
    return declared


# --- the check --------------------------------------------------------------------


def binding_value(entry: dict, name: str) -> str | None:
    """One entry's binding value, refused as absent or empty by its row."""
    run_id = entry.get("id")
    run = run_id if isinstance(run_id, str) and run_id else "?"
    bind = entry.get("bind")
    supplied = {}
    if isinstance(bind, dict):
        supplied = bind
    names = ", ".join(f'"{key}"' for key in sorted(supplied))
    value = supplied.get(name)
    if value is None:
        refuse(
            f"the spawn ledger entry for run {run} supplies no binding "
            f'"{name}"; it supplies {names}'
        )
    if not isinstance(value, str) or not value:
        refuse(f'the spawn ledger entry for run {run} binds "{name}" to an empty value')
    return value


def check(cwd: str):
    root = engine_root(cwd)
    matches = live_entries(root, cwd)
    if not matches:
        refuse(
            "no live spawn-ledger entry records this workspace; the validator "
            "resolves the turn's item and issue bindings from the "
            "Engine-owned ledger"
        )
    if len(matches) > 1:
        refuse(
            f"{len(matches)} live spawn-ledger entries record this workspace; "
            "the addressed turn is ambiguous"
        )
    entry = matches[0]
    item = binding_value(entry, "item")
    issue = binding_value(entry, "issue")

    roots = declared_roots(cwd)
    item_path = under(roots["ticket"], item)
    if not os.path.isfile(item_path):
        refuse(f"the item record {shown(item_path)} does not exist")
    item_text = read_record(item_path)

    issue_root = roots["issue"]
    bundle = next(
        (
            location
            for location in (
                under(issue_root, issue),
                under(issue_root, "deferred", issue),
                under(issue_root, "archive", issue),
            )
            if os.path.isdir(location)
        ),
        None,
    )
    if bundle is None:
        refuse(
            f"the selected issue {issue} resolves at no location of the issue "
            f"namespace {shown(issue_root)}"
        )

    residual_ids = yaml_list(item_text, "residual-ids")
    if not residual_ids:
        refuse(f"the item record {shown(item_path)} declares no residual-ids")
    requirements: list[str] = []
    cited_by: dict[str, str] = {}
    for residual in residual_ids:
        residual_path = under(roots["residual"], f"{residual}.md")
        if not os.path.isfile(residual_path):
            refuse(f"the residual record {shown(residual_path)} does not exist")
        requirement = yaml_field(read_record(residual_path), "goal-requirement-ref")
        if not requirement:
            refuse(
                f"the residual record {shown(residual_path)} carries no "
                "goal-requirement-ref"
            )
        if requirement not in cited_by:
            requirements.append(requirement)
            cited_by[requirement] = residual

    declared = [
        *yaml_list(item_text, "focused-tests"),
        *yaml_list(item_text, "hidden-lanes"),
    ]
    if not yaml_list(item_text, "focused-tests") and not yaml_list(
        item_text, "hidden-lanes"
    ):
        refuse(
            f"the item record {shown(item_path)} declares neither "
            "focused-tests nor hidden-lanes"
        )
    checks: list[str] = []
    for identifier in declared:
        if identifier and identifier not in checks:
            checks.append(identifier)

    spec = read_or_empty(under(bundle, "spec.md"))
    for requirement in requirements:
        if requirement not in spec:
            refuse(
                f"requirement {requirement} (cited by residual "
                f"{cited_by[requirement]}) does not occur in the selected "
                f"issue {issue}'s spec.md"
            )
    plan = read_or_empty(under(bundle, "test-plan.md"))
    for identifier in checks:
        if identifier not in plan:
            refuse(
                f"declared check {identifier} does not occur in the selected "
                f"issue {issue}'s test-plan.md"
            )

    return item, issue, len(requirements), len(checks)


def main(argv: list[str]) -> int:
    if os.name == "nt":
        sys.stdout.reconfigure(newline="\n")
        sys.stderr.reconfigure(newline="\n")
    if argv != ["check"]:
        print(f"{PREFIX}: usage: verify_issue_binding.py check", file=sys.stderr)
        return 2
    item, issue, requirements, checks = check(os.getcwd())
    noun_requirements = "requirement" if requirements == 1 else "requirements"
    noun_checks = "check" if checks == 1 else "checks"
    print(
        f"issue binding verified: {item} -> {issue} "
        f"({requirements} {noun_requirements}, {checks} {noun_checks})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
