#!/usr/bin/env python3
"""Add configFields to catalog entries.

- Entries with envVars get configFields with kind=env.
- Entries with <PLACEHOLDER> args get configFields with kind=arg or argSpread,
  and the args are updated to use {{template}} markers.
Idempotent.
"""
import json, sys
from pathlib import Path


def ev_to_cf(ev):
    cf = {
        "name": ev["name"],
        "kind": "env",
        "type": "secret" if ev.get("secret", True) else "string",
        "label": ev["name"].replace("_", " ").title(),
        "description": ev["description"],
        "required": ev.get("required", True),
    }
    if ev.get("placeholder"):
        cf["placeholder"] = ev["placeholder"]
    if ev.get("helpUrl"):
        cf["helpUrl"] = ev["helpUrl"]
    return cf


def add_configfields(data):
    servers_by_id = {s["id"]: s for s in data["servers"]}

    # 1. filesystem — argSpread + template marker
    fs = servers_by_id["filesystem"]
    fs["config"]["args"] = ["-y", "@modelcontextprotocol/server-filesystem", "{{paths}}"]
    fs.setdefault("configFields", [
        {
            "name": "paths",
            "kind": "argSpread",
            "type": "pathArray",
            "label": "Allowed paths",
            "description": "Directories the server can read/write.",
            "required": True,
        }
    ])

    # 2. git — arg for repo path
    git = servers_by_id["git"]
    git["config"]["args"] = ["mcp-server-git", "--repository", "{{repositoryPath}}"]
    git.setdefault("configFields", [
        {
            "name": "repositoryPath",
            "kind": "arg",
            "type": "path",
            "label": "Repository path",
            "description": "Path to the local Git repository.",
            "required": True,
        }
    ])

    # 3. postgres — arg for connection string
    pg = servers_by_id["postgres"]
    pg["config"]["args"] = ["-y", "@modelcontextprotocol/server-postgres", "{{connectionString}}"]
    pg.setdefault("configFields", [
        {
            "name": "connectionString",
            "kind": "arg",
            "type": "string",
            "label": "Connection string",
            "description": "PostgreSQL connection URI.",
            "required": True,
            "placeholder": "postgresql://user:pass@host/db",
        }
    ])

    # 4. sqlite — arg for db path
    sq = servers_by_id["sqlite"]
    sq["config"]["args"] = ["mcp-server-sqlite", "--db-path", "{{dbPath}}"]
    sq.setdefault("configFields", [
        {
            "name": "dbPath",
            "kind": "arg",
            "type": "path",
            "label": "Database path",
            "description": "Path to the SQLite database file.",
            "required": True,
        }
    ])

    # 5. redis — arg for url
    rd = servers_by_id["redis"]
    rd["config"]["args"] = ["--from", "redis-mcp-server@latest", "redis-mcp-server", "--url", "{{redisUrl}}"]
    rd.setdefault("configFields", [
        {
            "name": "redisUrl",
            "kind": "arg",
            "type": "url",
            "label": "Redis URL",
            "description": "Redis connection URL.",
            "required": True,
            "placeholder": "redis://localhost:6379",
        }
    ])

    # 6. twilio — arg for account/key combo
    tw = servers_by_id["twilio"]
    tw["config"]["args"] = ["-y", "@twilio-alpha/mcp", "{{accountSid}}/{{apiKey}}:{{apiSecret}}"]
    tw.setdefault("configFields", [
        {
            "name": "accountSid",
            "kind": "arg",
            "type": "string",
            "label": "Account SID",
            "description": "Twilio account SID.",
            "required": True,
            "placeholder": "ACxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        },
        {
            "name": "apiKey",
            "kind": "arg",
            "type": "string",
            "label": "API Key",
            "description": "Twilio API key SID.",
            "required": True,
            "placeholder": "SKxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        },
        {
            "name": "apiSecret",
            "kind": "arg",
            "type": "secret",
            "label": "API Secret",
            "description": "Twilio API key secret.",
            "required": True,
        },
    ])

    # 7. obsidian — arg for vault path
    ob = servers_by_id["obsidian"]
    ob["config"]["args"] = ["-y", "obsidian-mcp", "{{vaultPath}}"]
    ob.setdefault("configFields", [
        {
            "name": "vaultPath",
            "kind": "arg",
            "type": "path",
            "label": "Vault path",
            "description": "Path to the Obsidian vault directory.",
            "required": True,
        }
    ])

    # 8. stripe — --api-key= inline arg
    stripe = servers_by_id["stripe"]
    stripe["config"]["args"] = ["-y", "@stripe/mcp", "--api-key={{stripeApiKey}}"]
    stripe.setdefault("configFields", [
        {
            "name": "stripeApiKey",
            "kind": "arg",
            "type": "secret",
            "label": "Stripe API key",
            "description": "Stripe secret API key.",
            "required": True,
            "placeholder": "sk_...",
        }
    ])

    # 9. gitlab-self-hosted — arg for domain
    gl = servers_by_id.get("gitlab-self-hosted")
    if gl:
        gl["config"]["args"] = ["-y", "mcp-remote", "https://{{gitlabDomain}}/api/v4/mcp"]
        gl.setdefault("configFields", [
            {
                "name": "gitlabDomain",
                "kind": "arg",
                "type": "string",
                "label": "GitLab domain",
                "description": "Your self-hosted GitLab domain (e.g. gitlab.example.com).",
                "required": True,
                "placeholder": "gitlab.example.com",
            }
        ])

    # 10. All servers with envVars — copy to configFields with kind=env
    for s in data["servers"]:
        evs = s.get("envVars", [])
        if evs and "configFields" not in s:
            s["configFields"] = [ev_to_cf(ev) for ev in evs]

    return data


def migrate_file(path):
    data = json.loads(path.read_text())
    data = add_configfields(data)
    path.write_text(json.dumps(data, indent=2) + "\n")


if __name__ == "__main__":
    for p in sys.argv[1:]:
        migrate_file(Path(p))
        print(f"configFields added: {p}")
