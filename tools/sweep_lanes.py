#!/usr/bin/env python3
"""Landed-lane sweep (LNR-001..LNR-003): run every crate under the declared
lanes root in id order against the declared roster and write one report
artifact - exactly one verdict per crate (pass, expired, or red with the
failing lane ids; a crate the roster expects and the folder lacks is named
`missing`, never skipped) plus a total.

Contributor tool in the shape of tools/check_links.py, not a product gate:
the Engine keeps knowing nothing about lanes (ADR-0020). The sweep runs on
demand - when it runs is a human decision recorded where the working rules
keep it; this tool mints no schedule of its own.

An expiry marker lives inside the crate it marks (EXPIRED.toml) and names
the edition the lane last passed at. The sweep counts an expired lane as
neither pass nor red, so a red verdict always means a live regression.
Marking and unmarking are explicit acts - `mark` and `unmark` below, or the
same edits by hand - never automatic. Expired lanes are skipped by default;
`sweep --verify-expired` runs them anyway so a marker cannot quietly
outlive its lane's recovery.

Verbs:

    python tools/sweep_lanes.py sweep   [--verify-expired] [overrides]
    python tools/sweep_lanes.py check   [overrides]
    python tools/sweep_lanes.py mark    <crate> --edition <edition-NNN> \
                                        --reason <words> [--date <YYYY-MM-DD>]
    python tools/sweep_lanes.py unmark  <crate>

Declared data lives in .ratmac/lanes.toml - the runbook's [roots]-table
shape, repo-local beside the runbook (the runbook parser owns ratmac.toml's
tables and refuses unknown keys). It declares: lanes (the untracked root
swept in id order), report (the one artifact a sweep writes), and roster
(the landed crates the folder must hold). Every declared path resolves
against the config's repository root, so a fixture config sweeps a fixture.

Overrides: --config PATH, --lanes-root PATH, --report PATH, --timeout
SECONDS per crate (default 1800), and --target-dir PATH - shared build
output for every crate, an explicit opt-in; by default each crate builds
into its own target/ directory exactly as the lane always has, so the
sweep's orchestration perturbs nothing the lane can observe.
Exit codes: sweep exits 0 when every present crate reads pass or expired, 1
tool error. check exits 0 only when the report gives every rostered crate
pass or expired - a red unexpired verdict, a named-missing crate, a foreign
or absent row, and a missing or partial report each refuse with a named
reason. That refusal is what the close guard reads (LNR-003).
"""
from __future__ import annotations

import datetime
import io
import os
import re
import subprocess
import sys
import tempfile
import tomllib

TOOL_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

CONFIG_RELATIVE = os.path.join(".ratmac", "lanes.toml")
MARKER_NAME = "EXPIRED.toml"
CRATE_ID = re.compile(r"^t-\d{3}$")
EDITION_ID = re.compile(r"^edition-\d{3}$")
ROW = re.compile(r"^\| (t-\d{3}) \| ([a-z]+) \| (.*) \|$", re.MULTILINE)
FAILING_LANE = re.compile(r"^test (\S+) \.\.\. FAILED", re.MULTILINE)
PASSING_LANE = re.compile(r"^test \S+ \.\.\. ok$", re.MULTILINE)
ERROR_LINE = re.compile(r"^\s*(error(?:\[[^\]]+\])?: .+)$", re.MULTILINE)
VERDICTS = ("pass", "expired", "red", "missing")


def deny(message: str):
    print(f"sweep refused: {message}", file=sys.stderr)
    raise SystemExit(2)


# --- declared data -----------------------------------------------------------


def load_config(path):
    """The [roots]-table declaration: lanes, report, roster."""
    resolved = os.path.abspath(path) if path else os.path.join(TOOL_ROOT, CONFIG_RELATIVE)
    try:
        with open(resolved, "rb") as handle:
            data = tomllib.load(handle)
    except FileNotFoundError:
        deny(f"no lane declaration at {resolved} - pass --config or create it")
    except tomllib.TOMLDecodeError as error:
        deny(f"{resolved} is not valid TOML: {error}")
    roots = data.get("roots")
    if not isinstance(roots, dict):
        deny(f"{resolved} declares no [roots] table")
    unknown = sorted(set(roots) - {"lanes", "report", "roster"})
    if unknown:
        deny(f"{resolved} declares unknown [roots] keys {unknown}")
    for key in ("lanes", "report"):
        if not isinstance(roots.get(key), str) or not roots[key]:
            deny(f"{resolved} must declare {key} as a non-empty string")
    roster = roots.get("roster")
    if not isinstance(roster, list) or not roster:
        deny(f"{resolved} must declare roster as a non-empty list of crate ids")
    for crate in roster:
        if not isinstance(crate, str) or not CRATE_ID.match(crate):
            deny(f"{resolved} roster entry {crate!r} is not a crate id (t-###)")
    if len(set(roster)) != len(roster):
        deny(f"{resolved} roster repeats a crate id")
    return {
        "root": os.path.dirname(os.path.dirname(resolved)),
        "lanes": str(roots["lanes"]),
        "report": str(roots["report"]),
        "roster": sorted(roster),
    }


def under(root, value, default):
    """A declared or overridden path; relative values resolve against root."""
    return os.path.normpath(os.path.join(root, value if value else default))


# --- the lanes tree ----------------------------------------------------------


def crate_dir(config, crate):
    return os.path.join(config["lanes_root"], crate)


def marker_path(config, crate):
    return os.path.join(crate_dir(config, crate), MARKER_NAME)


def read_marker(config, crate):
    """The in-crate expiry marker, or None. A malformed marker refuses."""
    path = marker_path(config, crate)
    if not os.path.isfile(path):
        return None
    try:
        with open(path, "rb") as handle:
            data = tomllib.load(handle)
    except tomllib.TOMLDecodeError as error:
        deny(f"{crate}/{MARKER_NAME} is not valid TOML: {error}")
    edition = data.get("edition")
    date = data.get("date")
    reason = data.get("reason")
    if not isinstance(edition, str) or not EDITION_ID.match(edition):
        deny(f"{crate}/{MARKER_NAME} names no edition-NNN")
    if not isinstance(date, str) or not re.match(r"^\d{4}-\d{2}-\d{2}$", date):
        deny(f"{crate}/{MARKER_NAME} carries no YYYY-MM-DD date")
    if not isinstance(reason, str) or not reason.strip():
        deny(f"{crate}/{MARKER_NAME} carries no reason")
    return {"edition": edition, "date": date, "reason": reason.strip(), "path": path}


def stray_entries(config):
    """Everything under the lanes root the roster does not declare."""
    strays = []
    try:
        entries = sorted(os.listdir(config["lanes_root"]))
    except FileNotFoundError:
        deny(f"the declared lanes root {config['lanes_root']} does not exist")
    for name in entries:
        if name in config["roster"] and os.path.isdir(crate_dir(config, name)):
            continue
        if CRATE_ID.match(name):
            strays.append(f"{name} (a crate id the roster does not declare)")
        else:
            strays.append(f"{name} (not a crate id)")
    return strays


# --- running one crate -------------------------------------------------------
def run_crate(config, crate, timeout):
    """One `cargo test` in the crate's directory; (verdict, detail).

    The lane runs exactly as it always has - in its own crate, against its
    own build output - so nothing the sweep does perturbs what the lane can
    observe: no environment the historical run lacked. `--target-dir`
    deliberately opts a caller into shared build output instead.
    """
    environment = {
        key: value
        for key, value in os.environ.items()
        if not key.upper().startswith("CARGO_")
    }
    if config["target_dir"]:
        environment["CARGO_TARGET_DIR"] = config["target_dir"]
    try:
        completed = subprocess.run(
            ["cargo", "test", "--offline"],
            cwd=crate_dir(config, crate),
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return "red", f"timed out after {timeout}s"
    except OSError as error:
        return "red", f"cargo did not run: {error}"
    text = completed.stdout.decode("utf-8", errors="replace")
    if completed.returncode == 0:
        return "pass", f"{len(PASSING_LANE.findall(text))} lane(s) green"
    lanes = FAILING_LANE.findall(text)
    if lanes:
        return "red", ", ".join(lanes)
    match = ERROR_LINE.search(text)
    if match:
        detail = " ".join(match.group(1).split())[:160]
        return "red", f"build refused: {detail}"
    return "red", "cargo exited non-zero with no named lane or error line"


# --- the report --------------------------------------------------------------


def write_report(config, rows, strays):
    counts = {verdict: 0 for verdict in VERDICTS}
    for _crate, verdict, _detail in rows:
        counts[verdict] += 1
    lines = [
        "# Lane sweep report",
        "",
        f"- swept: {datetime.datetime.now(datetime.UTC).strftime('%Y-%m-%dT%H:%M:%SZ')}",
        f"- lanes-root: {config['lanes']}",
        f"- verify-expired: {'yes' if config['verify'] else 'no'}",
        (
            f"- verdicts: {counts['pass']} pass, {counts['expired']} expired,"
            f" {counts['red']} red, {counts['missing']} missing - {len(rows)} crates"
        ),
        "",
        "| crate | verdict | detail |",
        "| :--- | :--- | :--- |",
    ]
    lines.extend(f"| {crate} | {verdict} | {detail} |" for crate, verdict, detail in rows)
    if strays:
        lines.append("")
        lines.append(f"stray entries (in the folder, not in the roster): {', '.join(strays)}")
    lines.append("")
    report = "\n".join(lines)
    directory = os.path.dirname(config["report"])
    os.makedirs(directory, exist_ok=True)
    handle, temporary = tempfile.mkstemp(dir=directory, prefix=".lane-sweep-", suffix=".tmp")
    try:
        with os.fdopen(handle, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(report)
        os.replace(temporary, config["report"])
    except BaseException:
        if os.path.exists(temporary):
            os.unlink(temporary)
        raise
    return report


# --- verbs -------------------------------------------------------------------


def sweep(config):
    if not os.path.isdir(config["lanes_root"]):
        deny(f"the declared lanes root {config['lanes_root']} does not exist")
    strays = stray_entries(config)
    rows = []
    for crate in config["roster"]:
        if not os.path.isdir(crate_dir(config, crate)):
            rows.append((crate, "missing", "the roster expects this crate and the folder lacks it"))
            print(f"{crate}: missing")
            continue
        marker = read_marker(config, crate)
        if marker and not config["verify"]:
            detail = f"last passed at {marker['edition']} (marked {marker['date']})"
            rows.append((crate, "expired", detail))
            print(f"{crate}: expired")
            continue
        verdict, detail = run_crate(config, crate, config["timeout"])
        if marker:
            # Verify mode runs an expired lane anyway; the marker stays the
            # explicit fact until it is unmarked, and what the run observed
            # rides beside it so a marker cannot quietly outlive recovery.
            note = (
                "; verify: lanes green - the marker may be removed"
                if verdict == "pass"
                else f"; verify: lanes refuse - {detail}"
            )
            verdict = "expired"
            detail = f"last passed at {marker['edition']} (marked {marker['date']}){note}"
        rows.append((crate, verdict, detail))
        print(f"{crate}: {verdict}")
    write_report(config, rows, strays)
    counts = {verdict: 0 for verdict in VERDICTS}
    for _crate, verdict, _detail in rows:
        counts[verdict] += 1
    print(
        f"lane sweep: {counts['pass']} pass, {counts['expired']} expired,"
        f" {counts['red']} red, {counts['missing']} missing - {len(rows)} crates;"
        f" report: {config['report']}"
    )
    if strays:
        print(f"stray entries (in the folder, not in the roster): {', '.join(strays)}")
    return 0 if counts["red"] == 0 and counts["missing"] == 0 else 1


def check(config):
    """The verdict reader the close guard consumes: exit 0 only when every
    rostered crate reads pass or expired, over a whole, current report."""
    path = config["report"]
    if not os.path.isfile(path):
        deny(f"no lane sweep report at {path} - run the sweep before the close")
    try:
        text = io.open(path, encoding="utf-8").read()
    except OSError as error:
        deny(f"the lane sweep report {path} is unreadable: {error}")
    rows = {match.group(1): (match.group(2), match.group(3)) for match in ROW.finditer(text)}
    refusals = []
    for crate in config["roster"]:
        if crate not in rows:
            refusals.append(f"partial report: no verdict row for {crate}")
    for crate in sorted(set(rows) - set(config["roster"])):
        refusals.append(f"the report carries a verdict for {crate} the roster does not declare")
    summary = re.search(r"^- verdicts: (.+)$", text, re.MULTILINE)
    if not summary:
        refusals.append("partial report: no verdicts summary line")
    else:
        counts = {verdict: 0 for verdict in VERDICTS}
        for crate in config["roster"]:
            verdict = rows.get(crate, ("", ""))[0]
            if verdict in counts:
                counts[verdict] += 1
        expected = (
            f"{counts['pass']} pass, {counts['expired']} expired,"
            f" {counts['red']} red, {counts['missing']} missing - {len(config['roster'])} crates"
        )
        if summary.group(1).strip() != expected:
            refusals.append(
                f"partial report: the summary says {summary.group(1).strip()!r}"
                f" where the rows and roster say {expected!r}"
            )
    for crate in config["roster"]:
        row = rows.get(crate)
        if not row:
            continue
        verdict, detail = row
        if verdict not in VERDICTS:
            refusals.append(f"{crate} reads an unknown verdict {verdict!r}")
        elif verdict == "red":
            refusals.append(f"red, unexpired verdict: {crate} - {detail}")
        elif verdict == "missing":
            refusals.append(f"the roster expects {crate} and the report names it missing")
    if refusals:
        for refusal in dict.fromkeys(refusals):
            print(f"refused: {refusal}", file=sys.stderr)
        raise SystemExit(2)
    expired = sum(1 for crate in config["roster"] if rows[crate][0] == "expired")
    passing = sum(1 for crate in config["roster"] if rows[crate][0] == "pass")
    print(
        f"lane sweep report: every rostered crate reads pass or expired"
        f" ({passing} pass, {expired} expired)"
    )
    return 0


def mark(config, crate, edition, reason, date):
    if not CRATE_ID.match(crate):
        deny(f"{crate!r} is not a crate id (t-###)")
    if not os.path.isdir(crate_dir(config, crate)):
        deny(f"the lanes folder holds no crate {crate}")
    existing = read_marker(config, crate)
    if existing:
        deny(
            f"{crate} already carries a marker (last passed at {existing['edition']});"
            f" unmark it before marking again"
        )
    if not EDITION_ID.match(edition):
        deny(f"{edition!r} is not an edition id (edition-NNN)")
    if not reason.strip():
        deny("a marker carries a reason; pass --reason")
    stamped = date if date else datetime.date.today().isoformat()
    if not re.match(r"^\d{4}-\d{2}-\d{2}$", stamped):
        deny(f"{stamped!r} is not a YYYY-MM-DD date")
    body = (
        "# Lane expiry marker - written by an explicit act"
        " (tools/sweep_lanes.py mark), removed by the same explicit act"
        " (unmark).\n# The sweep counts an expired lane as neither pass nor"
        " red, so a red verdict always means a live\n# regression (LNR-002)."
        " A tracked summary may cite this marker; it never replaces it.\n"
        f'edition = "{edition}"\n'
        f'date = "{stamped}"\n'
        f'reason = "{reason.strip()}"\n'
    )
    with io.open(marker_path(config, crate), "w", encoding="utf-8", newline="\n") as stream:
        stream.write(body)
    print(f"{crate} marked expired: last passed at {edition} (marked {stamped})")
    return 0


def unmark(config, crate):
    if not CRATE_ID.match(crate):
        deny(f"{crate!r} is not a crate id (t-###)")
    marker = read_marker(config, crate)
    if not marker:
        deny(f"{crate} carries no expiry marker")
    os.unlink(marker["path"])
    print(f"{crate} unmarked - the lane is live again and must run green")
    return 0


VALUE_FLAGS = {
    "--config": "config",
    "--lanes-root": "lanes_root",
    "--report": "report",
    "--target-dir": "target_dir",
    "--timeout": "timeout",
    "--edition": "edition",
    "--reason": "reason",
    "--date": "date",
}


def main(argv):
    verb = argv[0] if argv else ""
    overrides = {"config": None, "lanes_root": None, "report": None, "target_dir": None}
    flags = {"verify": False, "timeout": "1800", "edition": None, "reason": None, "date": None}
    positional = []
    index = 1
    while index < len(argv):
        argument = argv[index]
        if argument in VALUE_FLAGS:
            index += 1
            if index >= len(argv):
                deny(f"{argument} needs a value")
            name = VALUE_FLAGS[argument]
            (overrides if name in overrides else flags)[name] = argv[index]
        elif argument == "--verify-expired":
            flags["verify"] = True
        elif argument in ("-h", "--help"):
            print(__doc__)
            return 0
        else:
            positional.append(argument)
        index += 1

    if verb not in ("sweep", "check", "mark", "unmark"):
        deny(f"unknown verb {verb!r} - sweep, check, mark, or unmark")

    config = load_config(overrides["config"])
    config["lanes_root"] = under(config["root"], overrides["lanes_root"], config["lanes"])
    config["report"] = under(config["root"], overrides["report"], config["report"])
    config["target_dir"] = (
        under(config["root"], overrides["target_dir"], None) if overrides["target_dir"] else None
    )
    config["verify"] = flags["verify"]
    try:
        config["timeout"] = int(flags["timeout"])
    except ValueError:
        deny("--timeout needs a number of seconds")

    if verb in ("mark", "unmark"):
        if len(positional) != 1:
            deny(f"{verb} takes exactly one crate id")
        if verb == "mark":
            if not flags["edition"] or not flags["reason"]:
                deny("mark needs --edition and --reason")
            return mark(config, positional[0], flags["edition"], flags["reason"], flags["date"])
        return unmark(config, positional[0])
    if positional:
        deny(f"{verb} takes no positional arguments")
    return sweep(config) if verb == "sweep" else check(config)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
