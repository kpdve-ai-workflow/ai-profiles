from tests.profile_management.contracts.support import ContractCase


class CliContractTests(ContractCase):
    def test_help_omitted_action_and_invalid_arguments(self):
        for args in ((), ("help",)):
            result = self.run_cli(*args)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("ai-profiles", result.stdout)
        invalid = self.run_cli("not-an-action")
        self.assertEqual(invalid.returncode, 2)

    def test_validate_from_unrelated_directory_and_spaced_checkout(self):
        result = self.run_cli("validate", cwd=self.project)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
