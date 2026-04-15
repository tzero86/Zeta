#!/usr/bin/env python3
"""
Zeta interactive release script.

Reads the current version from Cargo.toml, computes candidate next versions
for every release type, lets you choose, then updates Cargo.toml, commits,
tags, and pushes -- triggering the GitHub Actions release workflow.

Usage:
    python scripts/release.py
    python scripts/release.py --dry-run
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

# ---------------------------------------------------------------------------
# ANSI colour helpers -- degrade gracefully when the terminal does not support
# escape codes (redirect, dumb terminal, Windows without VT enabled).
# ---------------------------------------------------------------------------

def _supports_color() -> bool:
    if not sys.stdout.isatty():
        return False
    if sys.platform == "win32":
        # VT processing has been available since Windows 10 build 1511 and is
        # on by default in modern terminals (Windows Terminal, Warp).  Try to
        # enable it; fall back to no colour if the call fails.
        try:
            import ctypes
            kernel32 = ctypes.windll.kernel32  # type: ignore[attr-defined]
            handle = kernel32.GetStdHandle(-11)  # STD_OUTPUT_HANDLE
            mode = ctypes.c_ulong()
            kernel32.GetConsoleMode(handle, ctypes.byref(mode))
            kernel32.SetConsoleMode(handle, mode.value | 0x0004)  # ENABLE_VIRTUAL_TERMINAL_PROCESSING
        except Exception:
            return False
    return True

_COLOR = _supports_color()

_RESET  = "\033[0m"  if _COLOR else ""
_CYAN   = "\033[96m" if _COLOR else ""
_GREEN  = "\033[92m" if _COLOR else ""
_YELLOW = "\033[93m" if _COLOR else ""
_RED    = "\033[91m" if _COLOR else ""
_GRAY   = "\033[90m" if _COLOR else ""
_WHITE  = "\033[97m" if _COLOR else ""
_BOLD   = "\033[1m"  if _COLOR else ""


def _c(color: str, text: str) -> str:
    return f"{color}{text}{_RESET}"


def header(text: str) -> None:
    print(f"\n  {_c(_BOLD + _CYAN, text)}")
    print(f"  {_c(_GRAY, '-' * len(text))}")


def step(text: str) -> None:
    print(f"  {_c(_GRAY, '>>')} {_c(_GRAY, text)}")


def ok(text: str) -> None:
    print(f"  {_c(_GREEN, '[OK]')} {text}")


def warn(text: str) -> None:
    print(f"  {_c(_YELLOW, '[!]')} {text}")


def bail(text: str) -> None:
    print(f"  {_c(_RED, '[FAIL]')} {text}", file=sys.stderr)
    sys.exit(1)


# ---------------------------------------------------------------------------
# Semver
# ---------------------------------------------------------------------------

# Pre-release types recognised by the release workflow (release.yml).
PRE_TYPES = ("alpha", "beta", "rc", "dev")

_STABLE_RE = re.compile(r"^(\d+)\.(\d+)\.(\d+)$")
_PRE_RE    = re.compile(r"^(\d+)\.(\d+)\.(\d+)-([a-z]+)\.(\d+)$")


@dataclass
class Version:
    major: int
    minor: int
    patch: int
    pre_type: str = ""   # "" means stable
    pre_n: int    = 0

    @staticmethod
    def parse(raw: str) -> "Version":
        s = raw.lstrip("v")
        m = _PRE_RE.match(s)
        if m:
            return Version(int(m[1]), int(m[2]), int(m[3]), m[4], int(m[5]))
        m = _STABLE_RE.match(s)
        if m:
            return Version(int(m[1]), int(m[2]), int(m[3]))
        raise ValueError(f"Unrecognised version string: {raw!r}")

    @property
    def is_pre(self) -> bool:
        return bool(self.pre_type)

    def __str__(self) -> str:
        base = f"{self.major}.{self.minor}.{self.patch}"
        return f"{base}-{self.pre_type}.{self.pre_n}" if self.is_pre else base

    def tag(self) -> str:
        return f"v{self}"


# ---------------------------------------------------------------------------
# Candidate computation
#
# Pre-release targets stay at the *same* patch until promoted to stable.
# Bumping the same pre-type increments the counter; switching type resets to 1.
#
# Full ladder example:
#   0.3.4 -> alpha -> 0.3.5-alpha.1
#          -> alpha -> 0.3.5-alpha.2
#          -> beta  -> 0.3.5-beta.1
#          -> rc    -> 0.3.5-rc.1
#          -> patch -> 0.3.5   (promote to stable)
#          -> patch -> 0.3.6   (next stable bump)
# ---------------------------------------------------------------------------

@dataclass
class Candidate:
    key: str          # menu key, e.g. "patch"
    version: Version

    @property
    def is_pre(self) -> bool:
        return self.version.is_pre

    @property
    def label(self) -> str:
        return "pre-release" if self.is_pre else "stable"


def compute_candidates(cur: Version) -> list[Candidate]:
    # Next patch for pre-release targets: hold while on a pre, bump on stable.
    np = cur.patch if cur.is_pre else cur.patch + 1

    def make_pre(pt: str) -> Version:
        same = cur.is_pre and cur.pre_type == pt and cur.patch == np
        return Version(cur.major, cur.minor, np, pt, cur.pre_n + 1 if same else 1)

    return [
        Candidate("patch", Version(cur.major, cur.minor, np)),
        Candidate("minor", Version(cur.major, cur.minor + 1, 0)),
        Candidate("major", Version(cur.major + 1, 0, 0)),
        Candidate("alpha", make_pre("alpha")),
        Candidate("beta",  make_pre("beta")),
        Candidate("rc",    make_pre("rc")),
        Candidate("dev",   make_pre("dev")),
    ]


# ---------------------------------------------------------------------------
# Git helpers
# ---------------------------------------------------------------------------

def git(*args: str, check: bool = True) -> str:
    result = subprocess.run(
        ["git", *args],
        capture_output=True,
        text=True,
    )
    if check and result.returncode != 0:
        bail(f"git {args[0]} failed:\n{result.stderr.strip()}")
    return result.stdout.strip()


def cargo(*args: str) -> int:
    return subprocess.run(["cargo", *args]).returncode


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description="Zeta interactive release script")
    parser.add_argument("--dry-run", action="store_true", help="Preview only, no writes")
    args = parser.parse_args()
    dry_run: bool = args.dry_run

    # --- Locate repo root -----------------------------------------------
    repo_root = Path(__file__).resolve().parent.parent
    cargo_toml = repo_root / "Cargo.toml"
    if not cargo_toml.exists():
        bail(f"Cargo.toml not found at {cargo_toml}")

    # --- Read current version -------------------------------------------
    toml_text = cargo_toml.read_text(encoding="utf-8")
    version_match = re.search(r'^version\s*=\s*"([^"]+)"', toml_text, re.MULTILINE)
    if not version_match:
        bail("Could not find version field in Cargo.toml.")
    current_str = version_match.group(1)

    try:
        current = Version.parse(current_str)
    except ValueError as e:
        bail(str(e))

    # --- Git checks -----------------------------------------------------
    branch = git("rev-parse", "--abbrev-ref", "HEAD")
    dirty  = git("status", "--porcelain", check=False)
    latest_tag = git("tag", "--sort=-v:refname", check=False).splitlines()
    latest_tag_str = latest_tag[0] if latest_tag else "(none)"

    # --- Header ---------------------------------------------------------
    header("Zeta Release Script")
    print(f"  Branch  : {_c(_WHITE, branch)}")
    print(f"  Version : {_c(_WHITE, current_str)}  (Cargo.toml)")
    print(f"  Last tag: {_c(_WHITE, latest_tag_str)}")
    if dry_run:
        print(f"  Mode    : {_c(_YELLOW, 'DRY RUN -- no changes will be written')}")

    if dirty:
        print()
        warn("Working tree has uncommitted changes:")
        for line in dirty.splitlines():
            print(f"      {_c(_YELLOW, line)}")
        answer = input("\n  Continue anyway? [y/N] ").strip().lower()
        if answer != "y":
            print("  Cancelled.")
            sys.exit(0)

    # --- Menu -----------------------------------------------------------
    candidates = compute_candidates(current)

    header("Choose release type")
    for i, c in enumerate(candidates, 1):
        color = _YELLOW if c.is_pre else _GREEN
        print(
            f"  [{_c(color, str(i))}] "
            f"{_c(color, c.key.upper()):<18}"
            f"  ->  {_c(_BOLD + color, c.version.tag()):<30}"
            f"  {_c(_GRAY, c.label)}"
        )
    print(f"  {_c(_GRAY, '[0] Cancel')}")
    print()

    raw = input("  Select: ").strip()
    if raw == "0" or raw == "":
        print("  Cancelled.")
        sys.exit(0)

    try:
        choice = int(raw)
        selected = candidates[choice - 1]
        assert 1 <= choice <= len(candidates)
    except (ValueError, IndexError, AssertionError):
        bail(f"Invalid selection: {raw!r}")

    # --- Confirm --------------------------------------------------------
    print()
    pre_label = " (pre-release)" if selected.is_pre else ""
    print(f"  Release  : {_c(_BOLD + _CYAN, selected.version.tag())}{_c(_YELLOW, pre_label)}")
    print(f"  Cargo.toml will change: {current_str}  ->  {_c(_BOLD, str(selected.version))}")
    print()
    confirm = input("  Proceed? [y/N] ").strip().lower()
    if confirm != "y":
        print("  Cancelled.")
        sys.exit(0)

    # --- Guard: tag must not already exist ------------------------------
    existing = git("tag", "--list", selected.version.tag(), check=False)
    if existing:
        bail(
            f"Tag {selected.version.tag()} already exists. "
            f"Delete it first: git tag -d {selected.version.tag()}"
        )

    # --- Execute --------------------------------------------------------
    header("Executing")

    new_version_str = str(selected.version)
    new_tag         = selected.version.tag()

    if dry_run:
        step(f"[DRY RUN] Would write version {new_version_str} to Cargo.toml")
        step("[DRY RUN] Would run: cargo check --quiet")
        step("[DRY RUN] Would run: git add Cargo.toml Cargo.lock")
        step(f"[DRY RUN] Would run: git commit -m 'chore: bump version to {new_version_str}'")
        step(f"[DRY RUN] Would run: git tag {new_tag}")
        step("[DRY RUN] Would run: git push origin HEAD")
        step(f"[DRY RUN] Would run: git push origin {new_tag}")
        ok("Dry run complete -- nothing was written.")
        sys.exit(0)

    # 1. Update Cargo.toml
    step(f"Updating Cargo.toml -> {new_version_str}")
    new_toml = re.sub(
        r'(?m)(^version\s*=\s*")[^"]+(")',
        rf'\g<1>{new_version_str}\g<2>',
        toml_text,
    )
    cargo_toml.write_text(new_toml, encoding="utf-8")

    # 2. Validate + refresh Cargo.lock
    step("Running cargo check to refresh Cargo.lock")
    if cargo("check", "--quiet") != 0:
        cargo_toml.write_text(toml_text, encoding="utf-8")
        bail("cargo check failed -- Cargo.toml has been restored.")

    # 3. Commit
    step("Committing version bump")
    git("add", str(cargo_toml), str(repo_root / "Cargo.lock"))
    git("commit", "-m", f"chore: bump version to {new_version_str}")

    # 4. Tag
    step(f"Creating tag {new_tag}")
    git("tag", new_tag)

    # 5. Push branch + tag
    step("Pushing branch to origin")
    git("push", "origin", "HEAD")

    step(f"Pushing tag {new_tag} to origin")
    git("push", "origin", new_tag)

    # --- Done -----------------------------------------------------------
    print()
    ok(f"Released {new_tag}{pre_label}")
    print(f"  {_c(_GRAY, 'GitHub Actions will now build binaries and publish the release.')}")
    print(f"  {_c(_CYAN, f'https://github.com/tzero86/Zeta/releases/tag/{new_tag}')}")
    print()


if __name__ == "__main__":
    main()
