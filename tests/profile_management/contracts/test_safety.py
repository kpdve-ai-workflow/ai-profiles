from tests.profile_management.contracts.support import ContractCase
import json


class SafetyContractTests(ContractCase):
    def test_source_checkout_is_not_an_install_target(self):
        result = self.run_cli("install", "backend-go", "--project", str(self.source))
        self.assertEqual(result.returncode, 1)
        self.assertIn("repozytorium ai-profiles", result.stderr.lower())

    def test_modified_managed_file_blocks_update_and_uninstall_without_partial_changes(self):
        installed = self.run_cli("install", "backend-go", "--project", str(self.project))
        self.assertEqual(installed.returncode, 0, installed.stderr)
        managed = self.project / ".opencode/skills/go-backend/SKILL.md"
        managed.write_text(managed.read_text() + "\nlocal edit\n")

        def snapshot():
            return {p.relative_to(self.project).as_posix(): p.read_bytes()
                    for p in self.project.rglob("*") if p.is_file()}

        before = snapshot()
        for action in ("update", "uninstall"):
            result = self.run_cli(action, "--project", str(self.project))
            self.assertEqual(result.returncode, 1)
            self.assertEqual(snapshot(), before)

    def test_older_state_without_version_or_source_commit_remains_updateable(self):
        installed = self.run_cli("install", "backend-go", "--project", str(self.project))
        self.assertEqual(installed.returncode, 0, installed.stderr)
        state_path = self.project / ".opencode/ai-profile-state.json"
        state = json.loads(state_path.read_text())
        state.pop("version", None)
        state.pop("source_commit", None)
        state_path.write_text(json.dumps(state))
        updated = self.run_cli("update", "--project", str(self.project))
        self.assertEqual(updated.returncode, 0, updated.stderr)
