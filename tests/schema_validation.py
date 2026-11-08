
#!/usr/bin/env python3
"""Contract tests for DevInit's compile.yaml JSON Schema."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator
from ruamel.yaml import YAML


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas" / "compile.schema.json"
BINARY = ROOT / "target" / "debug" / "devinit"

YAML_PARSER = YAML(typ="safe")


class SchemaValidationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        Draft202012Validator.check_schema(schema)
        cls.validator = Draft202012Validator(schema)

        if not BINARY.is_file():
            raise RuntimeError(
                "DevInit binary not found. Run 'cargo build' first."
            )

    def check_config(self, yaml_content: str, expected_valid: bool):
        document = YAML_PARSER.load(yaml_content)

        schema_errors = list(self.validator.iter_errors(document))
        schema_valid = not schema_errors

        with tempfile.TemporaryDirectory(prefix="devinit-schema-") as tmp:
            config = Path(tmp) / "compile.yaml"
            config.write_text(yaml_content, encoding="utf-8")

            process = subprocess.run(
                [str(BINARY), "check", "-f", str(config), "--json"],
                cwd=tmp,
                capture_output=True,
                text=True,
                timeout=15,
                check=False,
            )

            try:
                result = json.loads(process.stdout)
            except json.JSONDecodeError as exc:
                self.fail(
                    f"DevInit did not return valid JSON: {exc}\n"
                    f"stdout: {process.stdout}\n"
                    f"stderr: {process.stderr}"
                )

            cli_valid = (
                process.returncode == 0
                and result.get("status") == "success"
                and result.get("valid") is True
            )

            self.assertEqual(
                schema_valid,
                expected_valid,
                f"Unexpected JSON Schema result:\n{schema_errors}",
            )

            self.assertEqual(
                cli_valid,
                expected_valid,
                f"Unexpected DevInit result:\n"
                f"stdout: {process.stdout}\n"
                f"stderr: {process.stderr}",
            )

    def test_registry_service(self):
        self.check_config(
            "services:\n  - redis\n",
            expected_valid=True,
        )

    def test_custom_service(self):
        self.check_config(
            """
services:
  - name: api
    image: nginx:alpine
    ports:
      - "8080:80"
""",
            expected_valid=True,
        )

    def test_versioned_service(self):
        self.check_config(
            """
services:
  - name: postgresql
    version: "16"
""",
            expected_valid=True,
        )

    def test_kubernetes_deployment_template(self):
        self.check_config(
            """
k8s:
  api-deployment:
    template: deployment
    name: api
    image: nginx:alpine
    replicas: 2
    port: 8080
""",
            expected_valid=True,
        )

    def test_kubernetes_service_template(self):
        self.check_config(
            """
k8s:
  api-service:
    template: service
    name: api-service
    target: api
    port: 8080
    targetPort: 3000
    type: ClusterIP
""",
            expected_valid=True,
        )

    def test_github_actions_pipeline(self):
        self.check_config(
            """
cicd:
  provider: github
  triggers:
    - type: push
      branches: [main]
  jobs:
    - name: build
      steps:
        - name: Checkout
          action: checkout
        - name: Build
          action: run
          command: cargo build
""",
            expected_valid=True,
        )

    def test_rejects_image_and_version(self):
        self.check_config(
            """
services:
  - name: redis
    image: redis:7
    version: "7"
""",
            expected_valid=False,
        )

    def test_rejects_unknown_root_field(self):
        self.check_config(
            """
services:
  - redis
unknown_setting: true
""",
            expected_valid=False,
        )

    def test_rejects_invalid_docker_discovery(self):
        self.check_config(
            """
services:
  - docker: false
""",
            expected_valid=False,
        )

    def test_rejects_invalid_kubernetes_service(self):
        self.check_config(
            """
k8s:
  api-service:
    template: service
    name: api-service
    target: api
    target_port: 3000
""",
            expected_valid=False,
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
