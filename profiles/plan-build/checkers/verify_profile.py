#!/usr/bin/env python3
"""Plan-Build profile self-validation (WCP-007): verify the installed
bundle against its own manifest, read-only.

The `command_exit` guard row the Machine Class template prepends at
intake runs this validator with its working directory the addressed Run's
workspace. The bundle resolves from this script's own location, never the
working directory, so the check answers the same from anywhere. It reads
`manifest.toml` beside the script and resolves, in order: the manifest's
bytes (readable, UTF-8, TOML); the profile format this checker reads; the
checker contract it honors; the runbook dialect the packaged template
speaks; every artifact row's known role, presence, and content digest; the
schema artifacts' declared identities; the Machine Class template's parse;
and the bundle's freedom from unlisted files. A bundle is exactly its
manifest inventory.

The validator writes nothing anywhere (PGE-004): its only outputs are its
exit code, one stdout line on success, and one stderr line on refusal,
every line ending in one LF on every platform.
"""
from __future__ import annotations

import hashlib
import os
import sys
import tomllib

PREFIX = "verify_profile"
FORMAT = 1
CHECKER_CONTRACT = 1
DIALECT = "states-v1"
MANIFEST_NAME = "manifest.toml"
MACHINE_CLASS_ROLE = "machine-class"
SCHEMA_ROLE = "schema"
KNOWN_ROLES = [
    "machine-class",
    "schema",
    "checker",
    "blank",
    "terms",
    "instructions",
]
SCHEMA_IDENTITIES = {
    "schemas/issue.toml": "issue-bundle-v1",
    "schemas/residual.toml": "residual-record-v1",
    "schemas/ticket.toml": "ticket-record-v1",
    "schemas/goal.toml": "goal-bundle-v1",
}


def refuse(message: str):
    print(f"{PREFIX}: {message}", file=sys.stderr)
    raise SystemExit(1)


def shown(path: str) -> str:
    return path.replace("\\", "/")


def artifact_path(bundle: str, relative: str) -> str:
    return os.path.join(bundle, *relative.split("/"))


def load_toml(path: str, spelling: str, what: str):
    """One file's parsed TOML, refused by its own unreadable/not-UTF-8/
    unparseable rows. `what` names the artifact kind for the parse row."""
    try:
        with open(path, "rb") as handle:
            raw = handle.read()
    except OSError as error:
        refuse(f"cannot read the {what} {spelling}: {error}")
    try:
        return tomllib.loads(raw.decode("utf-8"))
    except UnicodeDecodeError:
        refuse(f"the {what} {spelling} is not valid UTF-8")
    except tomllib.TOMLDecodeError as error:
        refuse(f"the {what} {spelling} does not parse as TOML: {error}")


def digest_of(path: str) -> str:
    with open(path, "rb") as handle:
        return hashlib.sha256(handle.read()).hexdigest()


def check(bundle: str):
    manifest_path = os.path.join(bundle, MANIFEST_NAME)
    data = load_toml(manifest_path, shown(manifest_path), "manifest")

    profile = data.get("profile", {})
    name = profile.get("name")
    observed_format = profile.get("format")
    if observed_format != FORMAT:
        refuse(
            f"the installed profile declares format {observed_format}, not "
            f"{FORMAT}; this checker reads format {FORMAT} - export a fresh "
            "bundle with a matching engine"
        )

    compatibility = data.get("compatibility", {})
    contract = compatibility.get("checkers")
    if contract != CHECKER_CONTRACT:
        refuse(
            f"the manifest declares checker contract {contract}, not "
            f"{CHECKER_CONTRACT}; this checker honors {CHECKER_CONTRACT}"
        )
    dialect = compatibility.get("runbook")
    if dialect != DIALECT:
        refuse(
            f'the manifest declares runbook dialect "{dialect}", not '
            f'"{DIALECT}"; the packaged Machine Class template speaks '
            f"{DIALECT}"
        )

    rows = data.get("artifact")
    if not isinstance(rows, list):
        refuse(f"the manifest {shown(manifest_path)} lists no artifact rows")
    listed: set[str] = set()
    template: str | None = None
    schemas: list[str] = []
    for row in rows:
        path = row.get("path")
        role = row.get("role")
        recorded = row.get("sha256")
        if role not in KNOWN_ROLES:
            refuse(
                f"the manifest lists {path} with role \"{role}\", which this "
                "checker does not know; known roles: "
                + ", ".join(KNOWN_ROLES)
            )
        artifact = artifact_path(bundle, path)
        if not os.path.isfile(artifact):
            refuse(
                f"the manifest lists {path} (role {role}), but the bundle "
                "holds no such file"
            )
        observed = digest_of(artifact)
        if observed != recorded:
            refuse(
                f"the artifact {path} (role {role}) hashes to {observed}, "
                f"but the manifest records {recorded}; restore the artifact "
                "or export a fresh bundle"
            )
        listed.add(path)
        if role == MACHINE_CLASS_ROLE:
            template = path
        elif role == SCHEMA_ROLE:
            schemas.append(path)

    for path in schemas:
        expected = SCHEMA_IDENTITIES.get(path)
        if expected is None:
            continue
        document = load_toml(
            artifact_path(bundle, path), path, "schema"
        )
        declared = document.get("schema")
        if declared != expected:
            refuse(
                f"the schema {path} declares schema = \"{declared}\", not "
                f'"{expected}"; the packaged checkers read {expected}'
            )

    if template is not None:
        load_toml(artifact_path(bundle, template), template, "Machine Class template")

    for directory, _, filenames in os.walk(bundle):
        for filename in sorted(filenames):
            relative = shown(
                os.path.relpath(os.path.join(directory, filename), bundle)
            )
            if relative == MANIFEST_NAME or relative in listed:
                continue
            refuse(
                f"the bundle holds {relative}, which the manifest does not "
                "list; a bundle is exactly its manifest inventory - remove "
                "the file or extend the manifest through the profile's owner"
            )

    return name, observed_format, len(rows)


def main(argv: list[str]) -> int:
    if os.name == "nt":
        sys.stdout.reconfigure(newline="\n")
        sys.stderr.reconfigure(newline="\n")
    if argv != ["check"]:
        print(f"{PREFIX}: usage: verify_profile.py check", file=sys.stderr)
        return 2
    script = os.path.dirname(os.path.abspath(__file__))
    bundle = os.path.dirname(script)
    name, profile_format, artifacts = check(bundle)
    print(
        f"profile verified: {name} "
        f"(format {profile_format}, {artifacts} artifacts)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
