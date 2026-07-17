import importlib.util
import unittest
from pathlib import Path


SCRIPT_PATH = Path(__file__).parents[1] / "scripts" / "validate.py"
SPEC = importlib.util.spec_from_file_location("catalog_validate", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("Unable to load validate.py")
VALIDATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATE)


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


if __name__ == "__main__":
    unittest.main()
