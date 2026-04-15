#!/usr/bin/env python3
"""Add prerequisites + install to every catalog entry based on launcher.

Idempotent — running twice produces the same output.
configFields migration (from envVars + arg inference) is done manually
afterwards. This script only handles the deterministic part.
"""
import json, re, sys
from pathlib import Path

LAUNCHER_TO_PREREQ = {"npx": "node", "uvx": "uv", "docker": "docker"}

def derive_install_step(server):
    cfg = server.get("config", {})
    if "url" in cfg:
        return {"type": "none"}
    cmd = (cfg.get("command") or "").lower()
    args = cfg.get("args") or []
    if cmd == "npx":
        # First non-flag arg is the package
        pkg = next((a for a in args if not a.startswith("-")), None)
        if pkg:
            return {"type": "npmWarmup", "package": pkg}
    if cmd == "uvx":
        pkg = next((a for a in args if not a.startswith("-")), None)
        if pkg:
            return {"type": "uvxWarmup", "package": pkg}
    if cmd == "docker":
        # Image is the last positional that isn't a flag value
        image = next((a for a in reversed(args) if not a.startswith("-")), None)
        if image:
            return {"type": "dockerPull", "image": image}
    return {"type": "none"}

def derive_prereq(server):
    cmd = (server.get("config", {}).get("command") or "").lower()
    runtime = LAUNCHER_TO_PREREQ.get(cmd)
    return [{"type": runtime}] if runtime else []

def migrate_one(server):
    server.setdefault("prerequisites", derive_prereq(server))
    if not server.get("install"):
        server["install"] = [derive_install_step(server)]
    return server

def migrate_file(path):
    data = json.loads(path.read_text())
    for s in data.get("servers", []):
        migrate_one(s)
    data["version"] = "1.1.0"
    path.write_text(json.dumps(data, indent=2) + "\n")

if __name__ == "__main__":
    for p in sys.argv[1:]:
        migrate_file(Path(p))
        print(f"migrated: {p}")
