# Contributing to configonaut-catalog

Thanks for adding to the Configonaut marketplace! This is a curated list, so every
addition goes through a quick review. The bar is:

1. The server is published on npm, PyPI, a Docker registry, or runs as a remote HTTP/SSE endpoint that's publicly reachable.
2. The install command actually works as written (we test it).
3. The author or vendor is identifiable — no anonymous forks of other servers.
4. The entry covers all required environment variables with `required: true, secret: true` set appropriately.

## Adding an entry

1. Fork the repo.
2. Open `scripts/build_catalog.py` and add a new dict to the `SERVERS` list.
3. Pick the right `category` (must match a `CATEGORIES[].id`).
4. Use the `npx()`, `uvx()`, `docker()`, or `http()` helpers to build the `config` block — don't hand-write JSON unless you need an unusual shape.
5. Regenerate:
   ```bash
   python3 scripts/build_catalog.py > catalog.json
   ```
6. Validate:
   ```bash
   python3 scripts/validate.py
   ```
7. Commit both files and open a PR.

### Minimum fields

```python
{
    "id": "my-server",                      # lowercase-slug
    "name": "My Server",                    # display name
    "description": "One-sentence what it does.",
    "category": "productivity",             # one of CATEGORIES[].id
    "tags": ["notes", "demo"],
    "publisher": {"name": "Acme Inc", "type": "vendor", "verified": True},
    "homepage": "https://github.com/acme/my-server",
    "repository": "https://github.com/acme/my-server",
    "license": "MIT",
    "popularity": 5,                        # 0-10, editorial
    "config": npx("@acme/my-server-mcp", env={"ACME_API_KEY": ""}),
    "envVars": [
        env("ACME_API_KEY", "Acme API key.",
            placeholder="ak_...",
            help_url="https://app.acme.com/settings/api"),
    ],
    "setupNotes": "Optional free-form instructions."
}
```

## Quality gates enforced by `validate.py`

- `catalog.json` parses and matches `schemas/catalog-v1.json`.
- Every `category` value matches a declared category id.
- Every `id` is unique.
- For every stdio entry using `npx` or `uvx`, the package is reachable on the registry (200 from `https://registry.npmjs.org/<pkg>` or `https://pypi.org/pypi/<pkg>/json`).
- Every `helpUrl` is https.
- Every `envVar.name` matches `^[A-Z][A-Z0-9_]*$`.
- No stdio entry has real credentials baked into `env` values (they must be empty strings or placeholder templates).

## Updating an entry

If a package moves or rename happens upstream, update the entry in place and
include a short note in the PR description. Don't delete the entry unless the
project is fully abandoned — mark it `"deprecated": true` instead so existing
Configonaut installs keep resolving it.

## Removing an entry

Open a PR that deletes the dict from `SERVERS` and regenerates `catalog.json`.
Include a link to evidence the server is abandoned or broken (last commit > 9
months, broken build, etc).

## Code of conduct

Be kind, link to sources, assume good intent. This is a free side project.
