import hashlib
import json
import os
import subprocess
import sys
from tests.profile_management.contracts.support import ContractCase


class RuntimeContractTests(ContractCase):
    def test_installed_runtime_is_standalone_and_byte_identical(self):
        result = self.run_cli("install", "backend-go", "--project", str(self.project))
        self.assertEqual(result.returncode, 0, result.stderr)
        delivered = self.project / ".opencode/ai-profile-mcp.py"
        original = self.source / "core/profile_management/implementations/python/runtime/mcp_runtime.py"
        self.assertEqual(hashlib.sha256(delivered.read_bytes()).digest(), hashlib.sha256(original.read_bytes()).digest())
        local = self.project / ".opencode/ai-profile.local.json"
        local.write_text(json.dumps({"mcp": {"postgres": {"command": [
            sys.executable, "-c", "import os; print(os.environ['CONTRACT_VALUE'])"],
            "environment": {"DATABASE_URI": "synthetic-db", "CONTRACT_VALUE": "from-local-config"}}}}))
        import shutil
        shutil.rmtree(self.source)
        env = dict(os.environ, CONTRACT_VALUE="from-inherited-environment")
        check = subprocess.run([sys.executable, str(delivered), "postgres"], cwd=self.project,
                               env=env, capture_output=True, text=True)
        self.assertEqual(check.returncode, 0, check.stderr)
        self.assertEqual(check.stdout.strip(), "from-local-config")
