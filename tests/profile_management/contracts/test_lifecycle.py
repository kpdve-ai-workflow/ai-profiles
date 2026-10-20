import json
from tests.profile_management.contracts.support import ContractCase


class LifecycleContractTests(ContractCase):
    def test_install_update_uninstall_preserves_user_files_and_is_idempotent(self):
        local = self.project / ".opencode/ai-profile.local.json"
        local.parent.mkdir()
        local.write_text('{"mcp":{"postgres":{"environment":{"X":"y"}}}}')
        result = self.run_cli("install", "backend-go", "--project", str(self.project))
        self.assertEqual(result.returncode, 0, result.stderr)
        managed = self.project / ".opencode/skills/go-backend/SKILL.md"
        self.assertTrue(managed.is_file())
        snap = {p.relative_to(self.project).as_posix(): p.read_bytes() for p in self.project.rglob("*") if p.is_file()}
        self.assertEqual(self.run_cli("update", "--project", str(self.project)).returncode, 0)
        current = {p.relative_to(self.project).as_posix(): p.read_bytes() for p in self.project.rglob("*") if p.is_file()}
        self.assertEqual(current, snap)
        self.assertEqual(self.run_cli("uninstall", "--project", str(self.project)).returncode, 0)
        self.assertTrue(local.exists())

        switch_project = self.base / "switch-project"
        switch_project.mkdir()
        initial = self.run_cli("install", "backend-go", "--project", str(switch_project))
        self.assertEqual(initial.returncode, 0, initial.stderr)
        switched = self.run_cli("switch", "frontend-react", "--project", str(switch_project))
        self.assertEqual(switched.returncode, 0, switched.stderr)
        state = json.loads((switch_project / ".opencode/ai-profile-state.json").read_text())
        self.assertEqual(state["profile"], "frontend-react")
        self.assertEqual(self.run_cli("uninstall", "--project", str(switch_project)).returncode, 0)
