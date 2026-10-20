from tests.profile_management.implementations.python.support import *


class PayloadTests(ProfileTestCase):

    def test_repository_local_skill_and_fresh_install_scope(self):
        local = self.source / ".opencode/skills/brain-to-implementer/SKILL.md"
        original = local.read_bytes()
        self.assertNotIn(".opencode/skills/brain-to-implementer/SKILL.md", desired_files(self.context, "backend-go"))
        apply_profile(self.context, self.project, "backend-go")
        self.assertNotIn(RULE, self.state()["files"])
        self.assertNotIn(".opencode/skills/brain-to-implementer/SKILL.md", self.state()["files"])
        self.assertFalse((self.project / RULE).exists())
        self.assertFalse((self.project / ".opencode/skills/brain-to-implementer").exists())
        self.assertNotIn("instructions", self.cfg())
        self.assertEqual(local.read_bytes(), original)
