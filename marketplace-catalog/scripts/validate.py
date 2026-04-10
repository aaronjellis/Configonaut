#!/usr/bin/env python3
"""
validate.py — Check catalog.json for shape, uniqueness, and package reachability.

Run from the repo root:
  python3 scripts/validate.py
"""
import json
import re
import sys
import urllib.request
import urllib.error
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CATALOG = ROOT / "catalog.json"
SCHEMA = ROOT / "schemas" / "catalog-v1.json"

ENV_NAME_RE = re.compile(r"^[A-Z][A-Z0-9_]*$")
ID_RE = re.compile(r"^[a-z][a-z0-9-]*$")

errors: list[str] = []
warnings: list[str] = []


def err(msg: str) -> None:
    errors.append(msg)
    print(f"ERROR: {msg}", file=sys.stderr)


def warn(msg: str) -> None:
    warnings.append(msg)
    print(f"WARN:  {msg}", file=sys.stderr)


def check_shape(catalog: dict) -> None:
    required_top = {"version", "generatedAt", "categories", "servers"}
    missing = required_top - set(catalog)
    if missing:
        err(f"catalog.json missing top-level keys: {sorted(missing)}")

    cat_ids = {c["id"] for c in catalog.get("categories", [])}

    seen_ids: set[str] = set()
    for s in catalog.get("servers", []):
        sid = s.get("id", "<no-id>")
        if not ID_RE.match(sid):
            err(f"server {sid!r}: id must match {ID_RE.pattern}")
        if sid in seen_ids:
            err(f"duplicate id: {sid}")
        seen_ids.add(sid)

        if s.get("category") not in cat_ids:
            err(f"server {sid!r}: unknown category {s.get('category')!r}")

        pub = s.get("publisher") or {}
        if pub.get("type") not in {"official", "vendor", "community"}:
            err(f"server {sid!r}: publisher.type must be official|vendor|community")

        for e in s.get("envVars", []):
            name = e.get("name", "")
            if not ENV_NAME_RE.match(name):
                err(f"server {sid!r}: envVar.name {name!r} not UPPER_SNAKE")
            if e.get("helpUrl") and not e["helpUrl"].startswith("https://"):
                warn(f"server {sid!r}: helpUrl not https for {name}")

        cfg = s.get("config") or {}
        if "url" in cfg:
            if not cfg["url"].startswith(("http://", "https://")):
                err(f"server {sid!r}: remote config.url must be http(s)")
        else:
            if "command" not in cfg:
                err(f"server {sid!r}: stdio config missing command")
            env = cfg.get("env") or {}
            for k, v in env.items():
                if v and not v.startswith(("", "<")) and "your" in str(v).lower():
                    warn(f"server {sid!r}: env.{k} looks like a baked-in placeholder")


def check_packages(catalog: dict) -> None:
    """Ping npm/PyPI for each stdio entry that uses a package manager."""
    for s in catalog.get("servers", []):
        cfg = s.get("config") or {}
        cmd = cfg.get("command")
        args = cfg.get("args") or []

        if cmd == "npx":
            # Find the package name after any -y flag.
            pkg = next((a for a in args if not a.startswith("-")), None)
            if pkg:
                check_npm(s["id"], pkg)
        elif cmd == "uvx":
            pkg = next((a for a in args if not a.startswith("-")), None)
            if pkg:
                check_pypi(s["id"], pkg)
        # docker and http left alone — no generic reachability test.


def check_npm(sid: str, pkg: str) -> None:
    url = f"https://registry.npmjs.org/{pkg}"
    try:
        with urllib.request.urlopen(url, timeout=8) as r:
            if r.status != 200:
                err(f"server {sid!r}: npm package {pkg!r} returned {r.status}")
    except urllib.error.HTTPError as e:
        err(f"server {sid!r}: npm package {pkg!r} not found ({e.code})")
    except Exception as e:
        warn(f"server {sid!r}: npm check for {pkg!r} failed: {e}")


def check_pypi(sid: str, pkg: str) -> None:
    url = f"https://pypi.org/pypi/{pkg}/json"
    try:
        with urllib.request.urlopen(url, timeout=8) as r:
            if r.status != 200:
                err(f"server {sid!r}: pypi package {pkg!r} returned {r.status}")
    except urllib.error.HTTPError as e:
        err(f"server {sid!r}: pypi package {pkg!r} not found ({e.code})")
    except Exception as e:
        warn(f"server {sid!r}: pypi check for {pkg!r} failed: {e}")


def main() -> int:
    if not CATALOG.exists():
        print(f"catalog.json not found at {CATALOG}", file=sys.stderr)
        return 2
    with open(CATALOG) as f:
        catalog = json.load(f)

    check_shape(catalog)

    if "--skip-network" not in sys.argv:
        check_packages(catalog)

    print(
        f"\nValidated {len(catalog.get('servers', []))} servers. "
        f"{len(errors)} error(s), {len(warnings)} warning(s)."
    )
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
