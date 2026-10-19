import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class MVPTest(unittest.TestCase):
    def run_py(self, *args):
        return subprocess.run([sys.executable, *map(str, args)], cwd=ROOT, text=True,
                              capture_output=True, check=True)

    def test_install_switch_gate(self):
        with tempfile.TemporaryDirectory() as tmp:
            dest = Path(tmp)
            (dest / 'opencode.json').write_text(json.dumps({'model': 'my/provider', 'mcp': {'custom': {'type': 'remote', 'url': 'https://example.org/mcp'}}}))
            self.run_py(ROOT / 'install.py', 'backend-go', dest)
            conf = json.loads((dest / 'opencode.json').read_text())
            self.assertEqual(conf['model'], 'my/provider')
            self.assertIn('postgres', conf['mcp'])
            self.assertIn('custom', conf['mcp'])
            self.assertTrue((dest / '.opencode/skills/implementation-breakdown/SKILL.md').exists())
            self.assertTrue((dest / '.opencode/skills/refine-plan/SKILL.md').exists())
            self.assertTrue((dest / '.opencode/skills/go-backend/SKILL.md').exists())
            self.assertIn('LOCKED', self.run_py(ROOT / 'gate.py', dest, 'status').stdout)
            self.run_py(ROOT / 'gate.py', dest, 'approve', 'TASK-001')
            self.assertIn('UNLOCKED', self.run_py(ROOT / 'gate.py', dest, 'status').stdout)
            self.run_py(ROOT / 'install.py', 'frontend-react', dest)
            conf = json.loads((dest / 'opencode.json').read_text())
            self.assertNotIn('postgres', conf['mcp'])
            self.assertIn('figma', conf['mcp'])
            self.assertIn('custom', conf['mcp'])
            self.assertFalse((dest / '.opencode/skills/go-backend').exists())
            self.assertTrue((dest / '.opencode/skills/react-frontend/SKILL.md').exists())
            self.assertIn('LOCKED', self.run_py(ROOT / 'gate.py', dest, 'status').stdout)


if __name__ == '__main__':
    unittest.main()
