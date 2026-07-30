import importlib.util
import json
import unittest
from pathlib import Path

SCRIPT_PATH = Path(__file__).parents[1] / "scripts" / "validate.py"
SPEC = importlib.util.spec_from_file_location("catalog_validate", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("Unable to load validate.py")
VALIDATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATE)

BUILD_SCRIPT_PATH = Path(__file__).parents[1] / "scripts" / "build_catalog.py"
BUILD_SPEC = importlib.util.spec_from_file_location(
    "catalog_build",
    BUILD_SCRIPT_PATH,
)
if BUILD_SPEC is None or BUILD_SPEC.loader is None:
    raise RuntimeError("Unable to load build_catalog.py")
BUILD_CATALOG = importlib.util.module_from_spec(BUILD_SPEC)
BUILD_SPEC.loader.exec_module(BUILD_CATALOG)


class ValidateTests(unittest.TestCase):
    def test_pypi_project_name_strips_uv_version_suffix(self) -> None:
        self.assertEqual(
            VALIDATE.pypi_project_name("redis-mcp-server@latest"),
            "redis-mcp-server",
        )

    def test_pypi_project_name_strips_extras(self) -> None:
        self.assertEqual(
            VALIDATE.pypi_project_name("example-mcp[cli]@1.2.3"),
            "example-mcp",
        )

    def test_xquik_remote_header_matches_catalog_sources(self) -> None:
        source_entry = next(
            server for server in BUILD_CATALOG.SERVERS if server["id"] == "xquik"
        )
        catalog = json.loads(
            (Path(__file__).parents[1] / "catalog.json").read_text(encoding="utf-8")
        )
        catalog_entry = next(
            server for server in catalog["servers"] if server["id"] == "xquik"
        )
        baseline = json.loads(
            (
                Path(__file__).parents[2]
                / "tauri-app"
                / "src-tauri"
                / "resources"
                / "catalog-baseline.json"
            ).read_text(encoding="utf-8")
        )
        baseline_entry = next(
            server for server in baseline["servers"] if server["id"] == "xquik"
        )

        self.assertEqual(catalog_entry["config"], source_entry["config"])
        self.assertEqual(catalog_entry["envVars"], source_entry["envVars"])
        self.assertEqual(baseline_entry, catalog_entry)
        self.assertEqual(
            catalog_entry["config"]["headers"]["Authorization"],
            "Bearer {{XQUIK_API_KEY}}",
        )


if __name__ == "__main__":
    unittest.main()
