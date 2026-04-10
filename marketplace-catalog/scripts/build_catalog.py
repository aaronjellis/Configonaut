#!/usr/bin/env python3
"""
build_catalog.py — Generate the Configonaut MCP marketplace catalog.

The catalog is authored as a Python list of dicts (easy to edit, review, diff)
and serialized to JSON. Each entry loosely follows the MCP registry server.json
schema but adds marketplace UX fields (popularity, tags, icon, helpUrl, etc.).

Run: python3 build_catalog.py > ../catalog.json
"""
import json
import sys
from datetime import date

CATALOG_VERSION = "1.0.0"

CATEGORIES = [
    {"id": "reference",      "label": "Reference",           "icon": "sparkles",     "description": "Official reference servers from Anthropic."},
    {"id": "development",    "label": "Developer Tools",     "icon": "code",          "description": "Source control, CI/CD, and code platforms."},
    {"id": "database",       "label": "Databases",           "icon": "database",      "description": "SQL, NoSQL, and data warehouses."},
    {"id": "cloud",          "label": "Cloud & Infra",       "icon": "cloud",         "description": "Hosting, compute, and cloud platforms."},
    {"id": "browser",        "label": "Browser & Web",       "icon": "globe",         "description": "Web automation, scraping, and fetching."},
    {"id": "search",         "label": "Search & Research",   "icon": "search",        "description": "Web search and research APIs."},
    {"id": "productivity",   "label": "Productivity",        "icon": "briefcase",     "description": "Notes, tasks, and knowledge management."},
    {"id": "communication",  "label": "Communication",       "icon": "message-circle","description": "Chat, email, and messaging platforms."},
    {"id": "design",         "label": "Design",              "icon": "palette",       "description": "Figma, Canva, and design tools."},
    {"id": "monitoring",     "label": "Monitoring",          "icon": "activity",      "description": "Observability, logs, and incident response."},
    {"id": "finance",        "label": "Finance & Payments",  "icon": "dollar-sign",   "description": "Stripe, billing, and payment APIs."},
    {"id": "ai",             "label": "AI & ML",             "icon": "cpu",           "description": "Model providers and inference tools."},
    {"id": "media",          "label": "Media",               "icon": "music",         "description": "Audio, video, and image generation."},
    {"id": "crm",            "label": "CRM & Sales",         "icon": "users",         "description": "Customer and sales platforms."},
    {"id": "filesystem",     "label": "Local Files",         "icon": "folder",        "description": "Local file system and editor integration."},
]


# Helper: build a standard stdio npx entry.
def npx(package, env=None):
    cfg = {"command": "npx", "args": ["-y", package]}
    if env:
        cfg["env"] = env
    return cfg

def uvx(package, env=None, args_extra=None):
    args = [package]
    if args_extra:
        args += args_extra
    cfg = {"command": "uvx", "args": args}
    if env:
        cfg["env"] = env
    return cfg

def docker(image, env_names=None, extra_args=None):
    args = ["run", "-i", "--rm"]
    if env_names:
        for n in env_names:
            args += ["-e", n]
    if extra_args:
        args += extra_args
    args.append(image)
    cfg = {"command": "docker", "args": args}
    if env_names:
        cfg["env"] = {n: "" for n in env_names}
    return cfg

def http(url, headers=None):
    cfg = {"url": url}
    if headers:
        cfg["headers"] = headers
    return cfg


# Environment variable helper
def env(name, description, required=True, secret=True, placeholder="", help_url=None):
    e = {
        "name": name,
        "description": description,
        "required": required,
        "secret": secret,
        "placeholder": placeholder,
    }
    if help_url:
        e["helpUrl"] = help_url
    return e


# ========================================================================
# CATALOG
# ========================================================================
SERVERS = [
    # --- REFERENCE (official Anthropic) ---
    {
        "id": "filesystem",
        "name": "Filesystem",
        "description": "Read, write, and navigate files on your computer with configurable access controls.",
        "category": "filesystem",
        "tags": ["files", "local", "reference"],
        "publisher": {"name": "Anthropic", "type": "official", "verified": True},
        "homepage": "https://github.com/modelcontextprotocol/servers/tree/main/src/filesystem",
        "repository": "https://github.com/modelcontextprotocol/servers",
        "license": "MIT",
        "popularity": 10,
        "config": {
            "command": "npx",
            "args": ["-y", "@modelcontextprotocol/server-filesystem", "<PATH>"],
        },
        "setupNotes": "Replace <PATH> with one or more directories to expose, e.g. /Users/you/projects.",
        "envVars": [],
    },
    {
        "id": "fetch",
        "name": "Fetch",
        "description": "Fetch web content and convert HTML to markdown for efficient LLM use.",
        "category": "browser",
        "tags": ["web", "http", "reference"],
        "publisher": {"name": "Anthropic", "type": "official", "verified": True},
        "homepage": "https://github.com/modelcontextprotocol/servers/tree/main/src/fetch",
        "repository": "https://github.com/modelcontextprotocol/servers",
        "license": "MIT",
        "popularity": 10,
        "config": uvx("mcp-server-fetch"),
        "envVars": [],
    },
    {
        "id": "git",
        "name": "Git",
        "description": "Read, search, and manipulate local Git repositories.",
        "category": "development",
        "tags": ["git", "version-control", "reference"],
        "publisher": {"name": "Anthropic", "type": "official", "verified": True},
        "homepage": "https://github.com/modelcontextprotocol/servers/tree/main/src/git",
        "repository": "https://github.com/modelcontextprotocol/servers",
        "license": "MIT",
        "popularity": 9,
        "config": uvx("mcp-server-git", args_extra=["--repository", "<REPO_PATH>"]),
        "setupNotes": "Replace <REPO_PATH> with the absolute path to a git repository.",
        "envVars": [],
    },
    {
        "id": "memory",
        "name": "Memory",
        "description": "Knowledge-graph based persistent memory across conversations.",
        "category": "reference",
        "tags": ["memory", "knowledge-graph", "reference"],
        "publisher": {"name": "Anthropic", "type": "official", "verified": True},
        "homepage": "https://github.com/modelcontextprotocol/servers/tree/main/src/memory",
        "repository": "https://github.com/modelcontextprotocol/servers",
        "license": "MIT",
        "popularity": 8,
        "config": npx("@modelcontextprotocol/server-memory"),
        "envVars": [],
    },
    {
        "id": "sequential-thinking",
        "name": "Sequential Thinking",
        "description": "Structured step-by-step reasoning and reflective problem-solving.",
        "category": "reference",
        "tags": ["reasoning", "reference"],
        "publisher": {"name": "Anthropic", "type": "official", "verified": True},
        "homepage": "https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking",
        "repository": "https://github.com/modelcontextprotocol/servers",
        "license": "MIT",
        "popularity": 7,
        "config": npx("@modelcontextprotocol/server-sequential-thinking"),
        "envVars": [],
    },
    {
        "id": "time",
        "name": "Time",
        "description": "Time and timezone conversions.",
        "category": "reference",
        "tags": ["time", "timezone", "reference"],
        "publisher": {"name": "Anthropic", "type": "official", "verified": True},
        "homepage": "https://github.com/modelcontextprotocol/servers/tree/main/src/time",
        "repository": "https://github.com/modelcontextprotocol/servers",
        "license": "MIT",
        "popularity": 6,
        "config": uvx("mcp-server-time"),
        "envVars": [],
    },
    {
        "id": "everything",
        "name": "Everything",
        "description": "Test server exercising all MCP protocol features (prompts, resources, tools).",
        "category": "reference",
        "tags": ["test", "reference", "dev"],
        "publisher": {"name": "Anthropic", "type": "official", "verified": True},
        "homepage": "https://github.com/modelcontextprotocol/servers/tree/main/src/everything",
        "repository": "https://github.com/modelcontextprotocol/servers",
        "license": "MIT",
        "popularity": 3,
        "config": npx("@modelcontextprotocol/server-everything"),
        "envVars": [],
    },

    # --- DEVELOPER TOOLS ---
    {
        "id": "github",
        "name": "GitHub",
        "description": "Official GitHub MCP server. Create issues, manage PRs, search code, and more.",
        "category": "development",
        "tags": ["git", "github", "issues", "pull-requests"],
        "publisher": {"name": "GitHub", "type": "vendor", "verified": True},
        "homepage": "https://github.com/github/github-mcp-server",
        "repository": "https://github.com/github/github-mcp-server",
        "license": "MIT",
        "popularity": 10,
        "config": docker("ghcr.io/github/github-mcp-server", env_names=["GITHUB_PERSONAL_ACCESS_TOKEN"]),
        "envVars": [
            env("GITHUB_PERSONAL_ACCESS_TOKEN",
                "Fine-grained or classic PAT with required repo scopes.",
                placeholder="ghp_...",
                help_url="https://github.com/settings/tokens")
        ],
        "setupNotes": "Runs locally via Docker with a PAT. A hosted remote MCP is also available at https://api.githubcopilot.com/mcp/ (OAuth, Copilot required)."
    },
    {
        "id": "gitlab",
        "name": "GitLab",
        "description": "Official GitLab MCP server. Manage projects, issues, merge requests, and pipelines.",
        "category": "development",
        "tags": ["git", "gitlab", "ci"],
        "publisher": {"name": "GitLab", "type": "vendor", "verified": True},
        "homepage": "https://docs.gitlab.com/user/gitlab_duo/model_context_protocol/mcp_server/",
        "repository": "https://gitlab.com/gitlab-org/gitlab",
        "license": "MIT",
        "popularity": 7,
        "config": http("https://gitlab.com/api/v4/mcp", headers={"Authorization": "Bearer <GITLAB_TOKEN>"}),
        "envVars": [
            env("GITLAB_TOKEN", "GitLab personal access token with api scope.",
                placeholder="glpat-...",
                help_url="https://gitlab.com/-/user_settings/personal_access_tokens")
        ],
    },
    {
        "id": "sentry",
        "name": "Sentry",
        "description": "Official Sentry MCP server. Query issues, releases, and error details.",
        "category": "monitoring",
        "tags": ["errors", "observability", "sentry"],
        "publisher": {"name": "Sentry", "type": "vendor", "verified": True},
        "homepage": "https://docs.sentry.io/product/sentry-mcp/",
        "repository": "https://github.com/getsentry/sentry-mcp",
        "license": "Apache-2.0",
        "popularity": 8,
        "config": http("https://mcp.sentry.dev/mcp"),
        "envVars": [],
        "setupNotes": "Remote server uses OAuth in-app; no static env var required."
    },
    {
        "id": "atlassian",
        "name": "Atlassian (Jira + Confluence)",
        "description": "Official remote Atlassian MCP server covering Jira, Confluence, and Compass.",
        "category": "productivity",
        "tags": ["jira", "confluence", "atlassian"],
        "publisher": {"name": "Atlassian", "type": "vendor", "verified": True},
        "homepage": "https://www.atlassian.com/platform/remote-mcp-server",
        "repository": "https://www.atlassian.com/platform/remote-mcp-server",
        "license": "Proprietary",
        "popularity": 9,
        "config": http("https://mcp.atlassian.com/v1/sse"),
        "envVars": [],
        "setupNotes": "Uses OAuth flow in Claude on first use; no token required here."
    },
    {
        "id": "linear",
        "name": "Linear",
        "description": "Official Linear MCP server. Query issues, projects, and cycles via OAuth.",
        "category": "productivity",
        "tags": ["tasks", "issues", "linear"],
        "publisher": {"name": "Linear", "type": "vendor", "verified": True},
        "homepage": "https://linear.app/docs/mcp",
        "repository": "https://linear.app/docs/mcp",
        "license": "Proprietary",
        "popularity": 9,
        "config": http("https://mcp.linear.app/sse"),
        "envVars": [],
        "setupNotes": "Uses OAuth. No static env var required."
    },
    {
        "id": "notion",
        "name": "Notion",
        "description": "Official Notion MCP server. Search and edit pages, databases, and blocks.",
        "category": "productivity",
        "tags": ["notes", "wiki", "notion"],
        "publisher": {"name": "Notion", "type": "vendor", "verified": True},
        "homepage": "https://github.com/makenotion/notion-mcp-server",
        "repository": "https://github.com/makenotion/notion-mcp-server",
        "license": "MIT",
        "popularity": 10,
        "config": {
            "command": "npx",
            "args": ["-y", "@notionhq/notion-mcp-server"],
            "env": {
                "OPENAPI_MCP_HEADERS": '{"Authorization": "Bearer ntn_<YOUR_TOKEN>", "Notion-Version": "2025-09-03"}'
            }
        },
        "envVars": [
            env("OPENAPI_MCP_HEADERS",
                "JSON object with Authorization and Notion-Version headers.",
                placeholder='{"Authorization": "Bearer ntn_...", "Notion-Version": "2025-09-03"}',
                help_url="https://www.notion.so/profile/integrations")
        ],
    },
    {
        "id": "stripe",
        "name": "Stripe",
        "description": "Official Stripe MCP server. Query customers, charges, subscriptions, and products.",
        "category": "finance",
        "tags": ["payments", "stripe", "billing"],
        "publisher": {"name": "Stripe", "type": "vendor", "verified": True},
        "homepage": "https://github.com/stripe/agent-toolkit",
        "repository": "https://github.com/stripe/agent-toolkit",
        "license": "MIT",
        "popularity": 9,
        "config": {
            "command": "npx",
            "args": ["-y", "@stripe/mcp", "--api-key=<STRIPE_API_KEY>"],
        },
        "envVars": [
            env("STRIPE_API_KEY", "Stripe secret key (use a restricted key when possible).",
                placeholder="sk_test_... or sk_live_...",
                help_url="https://dashboard.stripe.com/apikeys")
        ],
    },
    {
        "id": "cloudflare",
        "name": "Cloudflare",
        "description": "Official Cloudflare MCP. Manage Workers, Pages, KV, R2, DNS, and observability.",
        "category": "cloud",
        "tags": ["workers", "dns", "cloudflare"],
        "publisher": {"name": "Cloudflare", "type": "vendor", "verified": True},
        "homepage": "https://github.com/cloudflare/mcp-server-cloudflare",
        "repository": "https://github.com/cloudflare/mcp-server-cloudflare",
        "license": "Apache-2.0",
        "popularity": 8,
        "config": http("https://bindings.mcp.cloudflare.com/mcp"),
        "envVars": [],
        "setupNotes": "Uses OAuth via Cloudflare. Cloudflare publishes multiple tool-specific MCP endpoints — see repo for the full list."
    },
    {
        "id": "vercel",
        "name": "Vercel",
        "description": "Official Vercel MCP server. Deploy, query projects, and manage environments.",
        "category": "cloud",
        "tags": ["deploy", "hosting", "vercel"],
        "publisher": {"name": "Vercel", "type": "vendor", "verified": True},
        "homepage": "https://vercel.com/docs/mcp/vercel-mcp",
        "repository": "https://github.com/vercel/mcp",
        "license": "Proprietary",
        "popularity": 7,
        "config": http("https://mcp.vercel.com/"),
        "envVars": [],
        "setupNotes": "Uses OAuth. Authenticate via Claude on first use."
    },
    {
        "id": "netlify",
        "name": "Netlify",
        "description": "Official Netlify MCP server. Manage sites, deploys, and environment variables.",
        "category": "cloud",
        "tags": ["deploy", "hosting", "netlify"],
        "publisher": {"name": "Netlify", "type": "vendor", "verified": True},
        "homepage": "https://docs.netlify.com/welcome/build-with-ai/netlify-mcp-server/",
        "repository": "https://github.com/netlify/netlify-mcp",
        "license": "Proprietary",
        "popularity": 7,
        "config": npx("@netlify/mcp"),
        "envVars": [],
        "setupNotes": "Authenticates via Netlify CLI login."
    },
    {
        "id": "aws",
        "name": "AWS (AWS Labs)",
        "description": "Official AWS MCP servers from AWS Labs. Query S3, DynamoDB, CloudWatch and more.",
        "category": "cloud",
        "tags": ["aws", "s3", "cloudwatch"],
        "publisher": {"name": "Amazon Web Services", "type": "vendor", "verified": True},
        "homepage": "https://github.com/awslabs/mcp",
        "repository": "https://github.com/awslabs/mcp",
        "license": "Apache-2.0",
        "popularity": 8,
        "config": uvx("awslabs.core-mcp-server"),
        "envVars": [
            env("AWS_PROFILE", "AWS CLI profile name to use.", required=False, secret=False, placeholder="default"),
            env("AWS_REGION", "AWS region.", required=False, secret=False, placeholder="us-east-1"),
        ],
        "setupNotes": "Uses your local AWS credentials. Pick a specific awslabs.* server for targeted features."
    },
    {
        "id": "azure",
        "name": "Azure",
        "description": "Official Microsoft Azure MCP server. Query resources, subscriptions, and services.",
        "category": "cloud",
        "tags": ["azure", "microsoft", "cloud"],
        "publisher": {"name": "Microsoft", "type": "vendor", "verified": True},
        "homepage": "https://github.com/microsoft/mcp",
        "repository": "https://github.com/microsoft/mcp",
        "license": "MIT",
        "popularity": 7,
        "config": npx("@azure/mcp"),
        "envVars": [],
        "setupNotes": "Uses Azure CLI login (`az login`)."
    },
    {
        "id": "azure-devops",
        "name": "Azure DevOps",
        "description": "Official Microsoft Azure DevOps MCP server. Manage repos, work items, and pipelines.",
        "category": "development",
        "tags": ["azure", "devops", "ci"],
        "publisher": {"name": "Microsoft", "type": "vendor", "verified": True},
        "homepage": "https://github.com/microsoft/azure-devops-mcp",
        "repository": "https://github.com/microsoft/azure-devops-mcp",
        "license": "MIT",
        "popularity": 6,
        "config": npx("@azure-devops/mcp", env={"ADO_ORGANIZATION": ""}),
        "envVars": [
            env("ADO_ORGANIZATION", "Azure DevOps organization name.", required=True, secret=False, placeholder="myorg")
        ],
    },
    # --- DATABASES ---
    {
        "id": "postgres",
        "name": "PostgreSQL",
        "description": "Read-only SQL access to a Postgres database via the official reference server.",
        "category": "database",
        "tags": ["postgres", "sql", "database"],
        "publisher": {"name": "Community (archived reference)", "type": "community", "verified": False},
        "homepage": "https://github.com/modelcontextprotocol/servers-archived/tree/main/src/postgres",
        "repository": "https://github.com/modelcontextprotocol/servers-archived",
        "license": "MIT",
        "popularity": 8,
        "config": {
            "command": "npx",
            "args": ["-y", "@modelcontextprotocol/server-postgres", "postgresql://<USER>:<PASS>@<HOST>/<DB>"],
        },
        "envVars": [],
        "setupNotes": "Replace the connection string in the args. Archived reference but still widely used."
    },
    {
        "id": "sqlite",
        "name": "SQLite",
        "description": "Query and explore a local SQLite database.",
        "category": "database",
        "tags": ["sqlite", "sql", "database"],
        "publisher": {"name": "Community (archived reference)", "type": "community", "verified": False},
        "homepage": "https://github.com/modelcontextprotocol/servers-archived/tree/main/src/sqlite",
        "repository": "https://github.com/modelcontextprotocol/servers-archived",
        "license": "MIT",
        "popularity": 7,
        "config": uvx("mcp-server-sqlite", args_extra=["--db-path", "<PATH_TO_DB>"]),
        "envVars": [],
        "setupNotes": "Replace <PATH_TO_DB> with the path to your .sqlite file."
    },
    {
        "id": "mongodb",
        "name": "MongoDB",
        "description": "Official MongoDB MCP server. Query collections and run aggregations.",
        "category": "database",
        "tags": ["mongo", "nosql", "database"],
        "publisher": {"name": "MongoDB", "type": "vendor", "verified": True},
        "homepage": "https://github.com/mongodb-js/mongodb-mcp-server",
        "repository": "https://github.com/mongodb-js/mongodb-mcp-server",
        "license": "Apache-2.0",
        "popularity": 7,
        "config": npx("mongodb-mcp-server", env={"MDB_MCP_CONNECTION_STRING": ""}),
        "envVars": [
            env("MDB_MCP_CONNECTION_STRING", "MongoDB connection URI.",
                placeholder="mongodb+srv://user:pass@cluster.mongodb.net/",
                help_url="https://www.mongodb.com/docs/manual/reference/connection-string/")
        ],
    },
    {
        "id": "redis",
        "name": "Redis",
        "description": "Official Redis MCP server. Inspect keys, run commands, and query data.",
        "category": "database",
        "tags": ["redis", "cache", "database"],
        "publisher": {"name": "Redis", "type": "vendor", "verified": True},
        "homepage": "https://github.com/redis/mcp-redis",
        "repository": "https://github.com/redis/mcp-redis",
        "license": "MIT",
        "popularity": 7,
        "config": {
            "command": "uvx",
            "args": ["--from", "redis-mcp-server@latest", "redis-mcp-server", "--url", "<REDIS_URL>"],
        },
        "envVars": [],
        "setupNotes": "Replace <REDIS_URL> with your Redis connection URL (redis://...). REDIS_HOST/REDIS_PORT/REDIS_PASSWORD env vars are also supported as an alternative."
    },
    {
        "id": "neon",
        "name": "Neon",
        "description": "Official Neon MCP server. Manage serverless Postgres branches and queries.",
        "category": "database",
        "tags": ["postgres", "neon", "serverless"],
        "publisher": {"name": "Neon", "type": "vendor", "verified": True},
        "homepage": "https://github.com/neondatabase/mcp-server-neon",
        "repository": "https://github.com/neondatabase/mcp-server-neon",
        "license": "MIT",
        "popularity": 7,
        "config": http("https://mcp.neon.tech/mcp"),
        "envVars": [],
        "setupNotes": "Remote MCP uses OAuth (recommended). For local stdio, run `npx -y @neondatabase/mcp-server-neon start <NEON_API_KEY>` instead."
    },
    {
        "id": "supabase",
        "name": "Supabase",
        "description": "Official Supabase MCP server. Query Postgres, manage auth, and edge functions.",
        "category": "database",
        "tags": ["postgres", "supabase", "backend"],
        "publisher": {"name": "Supabase", "type": "vendor", "verified": True},
        "homepage": "https://github.com/supabase-community/supabase-mcp",
        "repository": "https://github.com/supabase-community/supabase-mcp",
        "license": "Apache-2.0",
        "popularity": 8,
        "config": http("https://mcp.supabase.com/mcp"),
        "envVars": [],
        "setupNotes": "Remote MCP uses OAuth (recommended). Append `?read_only=true&project_ref=<ref>` to scope. For local stdio, use `@supabase/mcp-server-supabase` with SUPABASE_ACCESS_TOKEN instead."
    },
    {
        "id": "bigquery",
        "name": "BigQuery",
        "description": "Query Google BigQuery datasets from Claude.",
        "category": "database",
        "tags": ["bigquery", "gcp", "sql", "warehouse"],
        "publisher": {"name": "Community (LucasHild)", "type": "community", "verified": False},
        "homepage": "https://github.com/LucasHild/mcp-server-bigquery",
        "repository": "https://github.com/LucasHild/mcp-server-bigquery",
        "license": "MIT",
        "popularity": 6,
        "config": uvx("mcp-server-bigquery", args_extra=["--project", "<GCP_PROJECT_ID>"]),
        "envVars": [],
        "setupNotes": "Replace <GCP_PROJECT_ID>. Uses Application Default Credentials (gcloud auth)."
    },
    {
        "id": "snowflake",
        "name": "Snowflake",
        "description": "Official Snowflake Labs MCP server. Query warehouses and Cortex AI.",
        "category": "database",
        "tags": ["snowflake", "warehouse", "sql"],
        "publisher": {"name": "Snowflake", "type": "vendor", "verified": True},
        "homepage": "https://github.com/Snowflake-Labs/mcp",
        "repository": "https://github.com/Snowflake-Labs/mcp",
        "license": "Apache-2.0",
        "popularity": 7,
        "config": uvx("snowflake-labs-mcp"),
        "envVars": [
            env("SNOWFLAKE_ACCOUNT", "Account identifier.", secret=False, placeholder="xy12345.us-east-1"),
            env("SNOWFLAKE_USER", "Username.", secret=False),
            env("SNOWFLAKE_PASSWORD", "Password or PAT.",
                help_url="https://docs.snowflake.com/en/user-guide/admin-user-management"),
            env("SNOWFLAKE_WAREHOUSE", "Warehouse name.", required=False, secret=False),
            env("SNOWFLAKE_ROLE", "Role name.", required=False, secret=False),
        ],
        "setupNotes": "Env vars are the simple path; a connections.toml config file is also supported."
    },
    {
        "id": "duckdb",
        "name": "DuckDB",
        "description": "Query DuckDB / Parquet / CSV datasets in place.",
        "category": "database",
        "tags": ["duckdb", "sql", "analytics"],
        "publisher": {"name": "MotherDuck", "type": "vendor", "verified": True},
        "homepage": "https://github.com/motherduckdb/mcp-server-motherduck",
        "repository": "https://github.com/motherduckdb/mcp-server-motherduck",
        "license": "MIT",
        "popularity": 6,
        "config": uvx("mcp-server-motherduck"),
        "envVars": [
            env("MOTHERDUCK_TOKEN", "MotherDuck token (leave empty to run fully local DuckDB).",
                required=False,
                help_url="https://app.motherduck.com/settings"),
        ],
    },
    {
        "id": "turso",
        "name": "Turso",
        "description": "Query Turso / libSQL databases at the edge.",
        "category": "database",
        "tags": ["turso", "libsql", "sqlite"],
        "publisher": {"name": "Community (spences10)", "type": "community", "verified": False},
        "homepage": "https://github.com/spences10/mcp-turso-cloud",
        "repository": "https://github.com/spences10/mcp-turso-cloud",
        "license": "MIT",
        "popularity": 5,
        "config": npx("mcp-turso-cloud", env={"TURSO_API_TOKEN": "", "TURSO_ORGANIZATION": ""}),
        "envVars": [
            env("TURSO_API_TOKEN", "Turso API token.", placeholder="eyJ...",
                help_url="https://app.turso.tech/account/settings"),
            env("TURSO_ORGANIZATION", "Organization slug.", secret=False),
            env("TURSO_DEFAULT_DATABASE", "Default database name (optional).", required=False, secret=False),
        ],
    },

    # --- BROWSER & WEB ---
    {
        "id": "playwright",
        "name": "Playwright",
        "description": "Official Microsoft Playwright MCP. Control a headless browser with accessibility tree.",
        "category": "browser",
        "tags": ["browser", "automation", "playwright"],
        "publisher": {"name": "Microsoft", "type": "vendor", "verified": True},
        "homepage": "https://github.com/microsoft/playwright-mcp",
        "repository": "https://github.com/microsoft/playwright-mcp",
        "license": "Apache-2.0",
        "popularity": 10,
        "config": npx("@playwright/mcp"),
        "envVars": [],
    },
    {
        "id": "chrome-devtools",
        "name": "Chrome DevTools",
        "description": "Official Chrome DevTools MCP. Inspect pages, profile performance, debug network.",
        "category": "browser",
        "tags": ["chrome", "devtools", "debug"],
        "publisher": {"name": "Google / Chrome DevTools", "type": "vendor", "verified": True},
        "homepage": "https://github.com/ChromeDevTools/chrome-devtools-mcp",
        "repository": "https://github.com/ChromeDevTools/chrome-devtools-mcp",
        "license": "Apache-2.0",
        "popularity": 8,
        "config": npx("chrome-devtools-mcp"),
        "envVars": [],
    },
    {
        "id": "puppeteer",
        "name": "Puppeteer",
        "description": "Control a headless Chromium via Puppeteer for scraping and automation.",
        "category": "browser",
        "tags": ["puppeteer", "chromium", "scraping"],
        "publisher": {"name": "Community (archived reference)", "type": "community", "verified": False},
        "homepage": "https://github.com/modelcontextprotocol/servers-archived/tree/main/src/puppeteer",
        "repository": "https://github.com/modelcontextprotocol/servers-archived",
        "license": "MIT",
        "popularity": 6,
        "config": npx("@modelcontextprotocol/server-puppeteer"),
        "envVars": [],
    },
    {
        "id": "browserbase",
        "name": "Browserbase",
        "description": "Run headless browsers in the cloud via Browserbase with session replay.",
        "category": "browser",
        "tags": ["browser", "cloud", "automation"],
        "publisher": {"name": "Browserbase", "type": "vendor", "verified": True},
        "homepage": "https://github.com/browserbase/mcp-server-browserbase",
        "repository": "https://github.com/browserbase/mcp-server-browserbase",
        "license": "MIT",
        "popularity": 7,
        "config": npx("@browserbasehq/mcp", env={"BROWSERBASE_API_KEY": "", "BROWSERBASE_PROJECT_ID": ""}),
        "envVars": [
            env("BROWSERBASE_API_KEY", "Browserbase API key.", placeholder="bb_...",
                help_url="https://www.browserbase.com/settings"),
            env("BROWSERBASE_PROJECT_ID", "Browserbase project ID.", secret=False),
        ],
    },
    {
        "id": "firecrawl",
        "name": "Firecrawl",
        "description": "Official Firecrawl MCP. Scrape, crawl, and extract structured data from any site.",
        "category": "browser",
        "tags": ["scraping", "crawl", "web"],
        "publisher": {"name": "Firecrawl", "type": "vendor", "verified": True},
        "homepage": "https://github.com/firecrawl/firecrawl-mcp-server",
        "repository": "https://github.com/firecrawl/firecrawl-mcp-server",
        "license": "MIT",
        "popularity": 8,
        "config": npx("firecrawl-mcp", env={"FIRECRAWL_API_KEY": ""}),
        "envVars": [
            env("FIRECRAWL_API_KEY", "Firecrawl API key.", placeholder="fc-...",
                help_url="https://www.firecrawl.dev/app/api-keys")
        ],
    },
    {
        "id": "apify",
        "name": "Apify",
        "description": "Run thousands of Apify actors for web scraping and automation.",
        "category": "browser",
        "tags": ["scraping", "actors", "apify"],
        "publisher": {"name": "Apify", "type": "vendor", "verified": True},
        "homepage": "https://github.com/apify/actors-mcp-server",
        "repository": "https://github.com/apify/actors-mcp-server",
        "license": "Apache-2.0",
        "popularity": 6,
        "config": npx("@apify/actors-mcp-server", env={"APIFY_TOKEN": ""}),
        "envVars": [
            env("APIFY_TOKEN", "Apify API token.", placeholder="apify_api_...",
                help_url="https://console.apify.com/account/integrations")
        ],
    },

    # --- SEARCH & RESEARCH ---
    {
        "id": "brave-search",
        "name": "Brave Search",
        "description": "Official Brave Search MCP. Private web search with image and news.",
        "category": "search",
        "tags": ["search", "web", "brave"],
        "publisher": {"name": "Brave", "type": "vendor", "verified": True},
        "homepage": "https://github.com/brave/brave-search-mcp-server",
        "repository": "https://github.com/brave/brave-search-mcp-server",
        "license": "MIT",
        "popularity": 9,
        "config": npx("@brave/brave-search-mcp-server", env={"BRAVE_API_KEY": ""}),
        "envVars": [
            env("BRAVE_API_KEY", "Brave Search API key.", placeholder="BSA...",
                help_url="https://brave.com/search/api/")
        ],
    },
    {
        "id": "tavily",
        "name": "Tavily",
        "description": "Official Tavily MCP server. AI-native search with extraction.",
        "category": "search",
        "tags": ["search", "research", "tavily"],
        "publisher": {"name": "Tavily", "type": "vendor", "verified": True},
        "homepage": "https://github.com/tavily-ai/tavily-mcp",
        "repository": "https://github.com/tavily-ai/tavily-mcp",
        "license": "MIT",
        "popularity": 8,
        "config": npx("tavily-mcp", env={"TAVILY_API_KEY": ""}),
        "envVars": [
            env("TAVILY_API_KEY", "Tavily API key.", placeholder="tvly-...",
                help_url="https://app.tavily.com/home")
        ],
    },
    {
        "id": "exa",
        "name": "Exa",
        "description": "Neural web search for LLMs by Exa Labs.",
        "category": "search",
        "tags": ["search", "neural", "exa"],
        "publisher": {"name": "Exa Labs", "type": "vendor", "verified": True},
        "homepage": "https://github.com/exa-labs/exa-mcp-server",
        "repository": "https://github.com/exa-labs/exa-mcp-server",
        "license": "MIT",
        "popularity": 7,
        "config": npx("exa-mcp-server", env={"EXA_API_KEY": ""}),
        "envVars": [
            env("EXA_API_KEY", "Exa API key.",
                help_url="https://dashboard.exa.ai/api-keys")
        ],
    },
    {
        "id": "kagi",
        "name": "Kagi Search",
        "description": "Official Kagi Search MCP. Premium ad-free web search.",
        "category": "search",
        "tags": ["search", "kagi"],
        "publisher": {"name": "Kagi", "type": "vendor", "verified": True},
        "homepage": "https://github.com/kagisearch/kagimcp",
        "repository": "https://github.com/kagisearch/kagimcp",
        "license": "MIT",
        "popularity": 6,
        "config": uvx("kagimcp", env={"KAGI_API_KEY": ""}),
        "envVars": [
            env("KAGI_API_KEY", "Kagi API key.",
                help_url="https://kagi.com/settings?p=api")
        ],
    },
    {
        "id": "perplexity",
        "name": "Perplexity",
        "description": "Official Perplexity MCP. Ask Sonar for sourced, up-to-date answers.",
        "category": "search",
        "tags": ["search", "ai", "perplexity"],
        "publisher": {"name": "Perplexity", "type": "vendor", "verified": True},
        "homepage": "https://github.com/ppl-ai/modelcontextprotocol",
        "repository": "https://github.com/ppl-ai/modelcontextprotocol",
        "license": "MIT",
        "popularity": 7,
        "config": npx("@perplexity-ai/mcp-server", env={"PERPLEXITY_API_KEY": ""}),
        "envVars": [
            env("PERPLEXITY_API_KEY", "Perplexity API key.", placeholder="pplx-...",
                help_url="https://www.perplexity.ai/settings/api")
        ],
    },

    # --- COMMUNICATION ---
    {
        "id": "slack",
        "name": "Slack",
        "description": "Community-maintained Slack MCP server (formerly official). Read and post to channels.",
        "category": "communication",
        "tags": ["slack", "chat", "team"],
        "publisher": {"name": "Zencoder", "type": "community", "verified": True},
        "homepage": "https://github.com/zencoderai/slack-mcp-server",
        "repository": "https://github.com/zencoderai/slack-mcp-server",
        "license": "MIT",
        "popularity": 9,
        "config": npx("@zencoderai/slack-mcp-server", env={"SLACK_BOT_TOKEN": "", "SLACK_TEAM_ID": ""}),
        "envVars": [
            env("SLACK_BOT_TOKEN", "Slack Bot User OAuth token.", placeholder="xoxb-...",
                help_url="https://api.slack.com/apps"),
            env("SLACK_TEAM_ID", "Workspace team ID.", secret=False, placeholder="T01234567"),
        ],
    },
    {
        "id": "discord",
        "name": "Discord",
        "description": "Community Discord MCP. Read and post messages via a bot token.",
        "category": "communication",
        "tags": ["discord", "chat"],
        "publisher": {"name": "Community", "type": "community", "verified": False},
        "homepage": "https://www.npmjs.com/package/discord-mcp-server",
        "repository": "https://www.npmjs.com/package/discord-mcp-server",
        "license": "MIT",
        "popularity": 6,
        "config": npx("discord-mcp-server", env={"DISCORD_TOKEN": ""}),
        "envVars": [
            env("DISCORD_TOKEN", "Discord bot token.",
                help_url="https://discord.com/developers/applications")
        ],
    },
    {
        "id": "gmail",
        "name": "Gmail",
        "description": "Send, search, and read Gmail messages.",
        "category": "communication",
        "tags": ["gmail", "email", "google"],
        "publisher": {"name": "Community (GongRzhe)", "type": "community", "verified": False},
        "homepage": "https://github.com/GongRzhe/Gmail-MCP-Server",
        "repository": "https://github.com/GongRzhe/Gmail-MCP-Server",
        "license": "MIT",
        "popularity": 7,
        "config": npx("@gongrzhe/server-gmail-autoauth-mcp"),
        "envVars": [],
        "setupNotes": "Follow the repo README to create a Google OAuth client; stores credentials locally."
    },
    {
        "id": "twilio",
        "name": "Twilio",
        "description": "Official Twilio Labs MCP. Send SMS, make calls, manage numbers.",
        "category": "communication",
        "tags": ["sms", "twilio", "voice"],
        "publisher": {"name": "Twilio", "type": "vendor", "verified": True},
        "homepage": "https://github.com/twilio-labs/mcp",
        "repository": "https://github.com/twilio-labs/mcp",
        "license": "MIT",
        "popularity": 6,
        "config": npx("@twilio-alpha/mcp", env={"TWILIO_ACCOUNT_SID": "", "TWILIO_API_KEY": "", "TWILIO_API_SECRET": ""}),
        "envVars": [
            env("TWILIO_ACCOUNT_SID", "Twilio Account SID.", placeholder="AC...",
                help_url="https://www.twilio.com/docs/iam/api-keys"),
            env("TWILIO_API_KEY", "Twilio API key SID.", placeholder="SK...",
                help_url="https://www.twilio.com/console/project/api-keys"),
            env("TWILIO_API_SECRET", "Twilio API key secret.",
                help_url="https://www.twilio.com/console/project/api-keys"),
        ],
    },
    {
        "id": "intercom",
        "name": "Intercom",
        "description": "Query and respond to Intercom conversations and contacts.",
        "category": "communication",
        "tags": ["support", "intercom", "customer"],
        "publisher": {"name": "Community (vineethnkrishnan)", "type": "community", "verified": False},
        "homepage": "https://www.npmjs.com/package/@vineethnkrishnan/intercom-mcp",
        "repository": "https://www.npmjs.com/package/@vineethnkrishnan/intercom-mcp",
        "license": "MIT",
        "popularity": 5,
        "config": npx("@vineethnkrishnan/intercom-mcp", env={"INTERCOM_ACCESS_TOKEN": ""}),
        "envVars": [
            env("INTERCOM_ACCESS_TOKEN", "Intercom access token.",
                help_url="https://developers.intercom.com/building-apps/docs/authentication-types")
        ],
    },

    # --- PRODUCTIVITY ---
    {
        "id": "asana",
        "name": "Asana",
        "description": "Query and update Asana tasks, projects, and workspaces.",
        "category": "productivity",
        "tags": ["tasks", "asana", "pm"],
        "publisher": {"name": "Community (roychri)", "type": "community", "verified": False},
        "homepage": "https://github.com/roychri/mcp-server-asana",
        "repository": "https://github.com/roychri/mcp-server-asana",
        "license": "MIT",
        "popularity": 6,
        "config": npx("@roychri/mcp-server-asana", env={"ASANA_ACCESS_TOKEN": ""}),
        "envVars": [
            env("ASANA_ACCESS_TOKEN", "Asana personal access token.",
                help_url="https://app.asana.com/0/my-apps")
        ],
    },
    {
        "id": "clickup",
        "name": "ClickUp",
        "description": "Manage ClickUp tasks, lists, and spaces.",
        "category": "productivity",
        "tags": ["clickup", "tasks", "pm"],
        "publisher": {"name": "Community (TaazKareem)", "type": "community", "verified": False},
        "homepage": "https://github.com/TaazKareem/clickup-mcp-server",
        "repository": "https://github.com/TaazKareem/clickup-mcp-server",
        "license": "MIT",
        "popularity": 5,
        "config": npx("@taazkareem/clickup-mcp-server", env={"CLICKUP_API_KEY": "", "CLICKUP_TEAM_ID": ""}),
        "envVars": [
            env("CLICKUP_API_KEY", "ClickUp personal API token.", placeholder="pk_...",
                help_url="https://app.clickup.com/settings/apps"),
            env("CLICKUP_TEAM_ID", "ClickUp team (workspace) ID.", secret=False),
        ],
    },
    {
        "id": "trello",
        "name": "Trello",
        "description": "Manage Trello boards, lists, and cards.",
        "category": "productivity",
        "tags": ["trello", "kanban", "pm"],
        "publisher": {"name": "Community (lioarce01)", "type": "community", "verified": False},
        "homepage": "https://github.com/lioarce01/trello-mcp-server",
        "repository": "https://github.com/lioarce01/trello-mcp-server",
        "license": "MIT",
        "popularity": 5,
        "config": npx("trello-mcp-server", env={"TRELLO_API_KEY": "", "TRELLO_TOKEN": ""}),
        "envVars": [
            env("TRELLO_API_KEY", "Trello API key.",
                help_url="https://trello.com/app-key"),
            env("TRELLO_TOKEN", "Trello token (generated by clicking the 'Token' link on the app-key page).",
                help_url="https://trello.com/app-key"),
        ],
        "setupNotes": "Visit the Power-Up admin page to get an API key, then click the Token link on that page to authorize and generate a token."
    },
    {
        "id": "obsidian",
        "name": "Obsidian",
        "description": "Read and search an Obsidian markdown vault.",
        "category": "productivity",
        "tags": ["obsidian", "notes", "markdown"],
        "publisher": {"name": "Community (StevenStavrakis)", "type": "community", "verified": False},
        "homepage": "https://github.com/StevenStavrakis/obsidian-mcp",
        "repository": "https://github.com/StevenStavrakis/obsidian-mcp",
        "license": "MIT",
        "popularity": 7,
        "config": {
            "command": "npx",
            "args": ["-y", "obsidian-mcp", "<PATH_TO_VAULT>"]
        },
        "envVars": [],
        "setupNotes": "Replace <PATH_TO_VAULT> with the absolute path to your Obsidian vault directory."
    },
    {
        "id": "google-drive",
        "name": "Google Drive",
        "description": "Browse and read Google Drive files (archived reference).",
        "category": "productivity",
        "tags": ["google", "drive", "docs"],
        "publisher": {"name": "Community (archived reference)", "type": "community", "verified": False},
        "homepage": "https://github.com/modelcontextprotocol/servers-archived/tree/main/src/gdrive",
        "repository": "https://github.com/modelcontextprotocol/servers-archived",
        "license": "MIT",
        "popularity": 7,
        "config": npx("@modelcontextprotocol/server-gdrive"),
        "envVars": [],
        "setupNotes": "Requires OAuth client setup via Google Cloud — see repo README."
    },
    {
        "id": "google-maps",
        "name": "Google Maps",
        "description": "Places, geocoding, and directions via Google Maps (archived reference).",
        "category": "productivity",
        "tags": ["maps", "google", "geocoding"],
        "publisher": {"name": "Community (archived reference)", "type": "community", "verified": False},
        "homepage": "https://github.com/modelcontextprotocol/servers-archived/tree/main/src/google-maps",
        "repository": "https://github.com/modelcontextprotocol/servers-archived",
        "license": "MIT",
        "popularity": 6,
        "config": npx("@modelcontextprotocol/server-google-maps", env={"GOOGLE_MAPS_API_KEY": ""}),
        "envVars": [
            env("GOOGLE_MAPS_API_KEY", "Google Maps Platform API key.",
                help_url="https://console.cloud.google.com/google/maps-apis/credentials")
        ],
    },

    # --- CRM & SALES ---
    {
        "id": "hubspot",
        "name": "HubSpot",
        "description": "Official HubSpot MCP server. Query contacts, companies, and deals.",
        "category": "crm",
        "tags": ["hubspot", "crm", "sales"],
        "publisher": {"name": "HubSpot", "type": "vendor", "verified": True},
        "homepage": "https://developer.hubspot.com/mcp",
        "repository": "https://developer.hubspot.com/mcp",
        "license": "Proprietary",
        "popularity": 7,
        "config": http("https://mcp.hubspot.com/anthropic"),
        "envVars": [],
        "setupNotes": "Uses OAuth via HubSpot."
    },
    {
        "id": "salesforce",
        "name": "Salesforce",
        "description": "Query and modify Salesforce records with SOQL and metadata APIs.",
        "category": "crm",
        "tags": ["salesforce", "crm", "soql"],
        "publisher": {"name": "Community (tsmztech)", "type": "community", "verified": False},
        "homepage": "https://github.com/tsmztech/mcp-server-salesforce",
        "repository": "https://github.com/tsmztech/mcp-server-salesforce",
        "license": "MIT",
        "popularity": 6,
        "config": npx("@tsmztech/mcp-server-salesforce",
                     env={"SALESFORCE_CONNECTION_TYPE": "User_Password",
                          "SALESFORCE_USERNAME": "", "SALESFORCE_PASSWORD": "", "SALESFORCE_TOKEN": "",
                          "SALESFORCE_INSTANCE_URL": ""}),
        "envVars": [
            env("SALESFORCE_CONNECTION_TYPE",
                "Auth method: 'User_Password', 'OAuth_2.0_Client_Credentials', or 'Salesforce_CLI'.",
                secret=False, placeholder="User_Password"),
            env("SALESFORCE_USERNAME", "Salesforce username (User_Password mode).",
                required=False, secret=False),
            env("SALESFORCE_PASSWORD", "Salesforce password (User_Password mode).", required=False),
            env("SALESFORCE_TOKEN", "Salesforce security token (User_Password mode).",
                required=False,
                help_url="https://help.salesforce.com/s/articleView?id=sf.user_security_token.htm"),
            env("SALESFORCE_INSTANCE_URL",
                "Your org URL (optional for User_Password, REQUIRED for OAuth_2.0_Client_Credentials).",
                required=False, secret=False,
                placeholder="https://your-domain.my.salesforce.com"),
        ],
        "setupNotes": "Three auth modes: User_Password (default, needs username/password/token), OAuth_2.0_Client_Credentials (needs SALESFORCE_CLIENT_ID/SECRET/INSTANCE_URL), or Salesforce_CLI (requires `sf` CLI)."
    },

    # --- DESIGN ---
    {
        "id": "figma",
        "name": "Figma",
        "description": "Official Figma MCP server. Fetch design context, variables, and frames.",
        "category": "design",
        "tags": ["figma", "design", "components"],
        "publisher": {"name": "Figma", "type": "vendor", "verified": True},
        "homepage": "https://www.figma.com/developers/mcp",
        "repository": "https://www.figma.com/developers/mcp",
        "license": "Proprietary",
        "popularity": 9,
        "config": http("https://mcp.figma.com/mcp"),
        "envVars": [],
        "setupNotes": "Remote MCP. Authenticates via Figma OAuth on first use. Local desktop-app MCP (http://127.0.0.1:3845/mcp) is also available if you enable it in Figma Preferences."
    },
    {
        "id": "figma-context",
        "name": "Figma (Framelink)",
        "description": "Community Figma context MCP. Lightweight frame-to-markdown extractor.",
        "category": "design",
        "tags": ["figma", "design"],
        "publisher": {"name": "Community (GLips)", "type": "community", "verified": False},
        "homepage": "https://github.com/GLips/Figma-Context-MCP",
        "repository": "https://github.com/GLips/Figma-Context-MCP",
        "license": "MIT",
        "popularity": 6,
        "config": npx("figma-developer-mcp", env={"FIGMA_API_KEY": ""}),
        "envVars": [
            env("FIGMA_API_KEY", "Figma personal access token.", placeholder="figd_...",
                help_url="https://www.figma.com/developers/api#access-tokens")
        ],
    },

    # --- MONITORING ---
    {
        "id": "pagerduty",
        "name": "PagerDuty",
        "description": "Official PagerDuty MCP. Query incidents, services, and on-call.",
        "category": "monitoring",
        "tags": ["pagerduty", "incident", "oncall"],
        "publisher": {"name": "PagerDuty", "type": "vendor", "verified": True},
        "homepage": "https://github.com/PagerDuty/pagerduty-mcp-server",
        "repository": "https://github.com/PagerDuty/pagerduty-mcp-server",
        "license": "Apache-2.0",
        "popularity": 7,
        "config": uvx("pagerduty-mcp-server",
                      env={"PAGERDUTY_USER_API_KEY": "", "PAGERDUTY_API_HOST": "https://api.pagerduty.com"}),
        "envVars": [
            env("PAGERDUTY_USER_API_KEY", "PagerDuty user-level API token.",
                help_url="https://support.pagerduty.com/docs/api-access-keys"),
            env("PAGERDUTY_API_HOST", "PagerDuty API host (use https://api.eu.pagerduty.com for EU accounts).",
                required=False, secret=False, placeholder="https://api.pagerduty.com"),
        ],
    },
    {
        "id": "grafana",
        "name": "Grafana",
        "description": "Official Grafana MCP. Query dashboards, datasources, and alerts.",
        "category": "monitoring",
        "tags": ["grafana", "metrics", "observability"],
        "publisher": {"name": "Grafana Labs", "type": "vendor", "verified": True},
        "homepage": "https://github.com/grafana/mcp-grafana",
        "repository": "https://github.com/grafana/mcp-grafana",
        "license": "Apache-2.0",
        "popularity": 7,
        "config": {
            "command": "mcp-grafana",
            "args": [],
            "env": {"GRAFANA_URL": "", "GRAFANA_SERVICE_ACCOUNT_TOKEN": ""}
        },
        "envVars": [
            env("GRAFANA_URL", "Grafana base URL.", secret=False, placeholder="https://grafana.example.com"),
            env("GRAFANA_SERVICE_ACCOUNT_TOKEN", "Grafana service account token.",
                help_url="https://grafana.com/docs/grafana/latest/administration/service-accounts/"),
        ],
        "setupNotes": "Install `mcp-grafana` binary separately (`go install github.com/grafana/mcp-grafana/cmd/mcp-grafana@latest`)."
    },
    {
        "id": "datadog",
        "name": "Datadog",
        "description": "Query Datadog metrics, logs, and monitors.",
        "category": "monitoring",
        "tags": ["datadog", "metrics", "observability"],
        "publisher": {"name": "Community (GeLi2001)", "type": "community", "verified": False},
        "homepage": "https://github.com/GeLi2001/datadog-mcp-server",
        "repository": "https://github.com/GeLi2001/datadog-mcp-server",
        "license": "MIT",
        "popularity": 6,
        "config": npx("datadog-mcp-server",
                     env={"DD_API_KEY": "", "DD_APP_KEY": "", "DD_SITE": "datadoghq.com"}),
        "envVars": [
            env("DD_API_KEY", "Datadog API key.",
                help_url="https://app.datadoghq.com/organization-settings/api-keys"),
            env("DD_APP_KEY", "Datadog application key.",
                help_url="https://app.datadoghq.com/organization-settings/application-keys"),
            env("DD_SITE", "Datadog site (e.g., datadoghq.com, datadoghq.eu).",
                required=False, secret=False, placeholder="datadoghq.com"),
        ],
    },

    # --- AI & ML ---
    {
        "id": "huggingface",
        "name": "Hugging Face",
        "description": "Official Hugging Face MCP. Search models, datasets, and run inference.",
        "category": "ai",
        "tags": ["huggingface", "models", "ai"],
        "publisher": {"name": "Hugging Face", "type": "vendor", "verified": True},
        "homepage": "https://huggingface.co/settings/mcp",
        "repository": "https://huggingface.co/docs/hub/mcp",
        "license": "Apache-2.0",
        "popularity": 8,
        "config": http("https://hf.co/mcp", headers={"Authorization": "Bearer <HF_TOKEN>"}),
        "envVars": [
            env("HF_TOKEN", "Hugging Face access token.", placeholder="hf_...",
                help_url="https://huggingface.co/settings/tokens")
        ],
    },
    {
        "id": "elevenlabs",
        "name": "ElevenLabs",
        "description": "Official ElevenLabs MCP. Text-to-speech, voice cloning, and audio tools.",
        "category": "media",
        "tags": ["tts", "audio", "elevenlabs"],
        "publisher": {"name": "ElevenLabs", "type": "vendor", "verified": True},
        "homepage": "https://github.com/elevenlabs/elevenlabs-mcp",
        "repository": "https://github.com/elevenlabs/elevenlabs-mcp",
        "license": "MIT",
        "popularity": 7,
        "config": uvx("elevenlabs-mcp", env={"ELEVENLABS_API_KEY": ""}),
        "envVars": [
            env("ELEVENLABS_API_KEY", "ElevenLabs API key.",
                help_url="https://elevenlabs.io/app/settings/api-keys")
        ],
    },

]


def main():
    catalog = {
        "$schema": "https://configonaut.app/schemas/catalog-v1.json",
        "version": CATALOG_VERSION,
        "generatedAt": date.today().isoformat(),
        "source": "https://github.com/aaronellis/configonaut-catalog",
        "categories": CATEGORIES,
        "servers": SERVERS,
    }
    json.dump(catalog, sys.stdout, indent=2, ensure_ascii=False)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
