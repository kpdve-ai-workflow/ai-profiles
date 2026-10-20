from tests.profile_management.implementations.python.support import *
from core.profile_management.implementations.python.manager.actions.installation import apply_profile


class StatusTests(ProfileTestCase):
    def test_status_labels_installation_and_available_update(self):
        apply_profile(self.context, self.project, "backend-go")
        self.assertIn("Status: UP TO DATE", self.status_output())
        self.set_version("1.1.0")
        self.assertIn("Status: UPDATE AVAILABLE", self.status_output())

    def test_status_reports_content_change_and_local_modifications(self):
        apply_profile(self.context, self.project, "backend-go")
        skill = self.project / ".opencode/skills/go-backend/SKILL.md"
        skill.write_text("locally changed\n")
        output = self.status_output()
        self.assertIn("Local modifications: .opencode/skills/go-backend/SKILL.md", output)
