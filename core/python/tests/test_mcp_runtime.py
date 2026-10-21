from tests.support import *


class McpRuntimeTests(ProfileTestCase):

    def test_access_mode_survives_update_and_runtime_receives_single_flag(self):
        local = self.local_config({"access_mode": "restricted"})
        before = local.read_bytes()
        apply_profile(self.context, self.project, "backend-go")
        mcp_path = self.source / "mcp/servers/postgres.json"
        base = json.loads(mcp_path.read_text())
        base["command"].append("--transport=stdio")
        mcp_path.write_text(json.dumps(base))
        apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(local.read_bytes(), before)
        with patch.dict(mcp_runtime.os.environ, {"DATABASE_URI": "test-connection"}), \
                patch.object(mcp_runtime.os, "chdir"), patch.object(mcp_runtime.os, "execvpe") as execute:
            mcp_runtime.launch(self.project, "postgres")
        command = execute.call_args.args[1]
        self.assertEqual(command[-1], "--access-mode=restricted")
        self.assertEqual(sum(arg.startswith("--access-mode") for arg in command), 1)
        self.assertIn("--transport=stdio", command)

    def test_runtime_rejects_state_without_base_instead_of_using_rendered_command(self):
        apply_profile(self.context, self.project, "backend-go")
        state = self.state()
        del state["mcp_base"]
        (self.project / STATE).write_text(json.dumps(state))
        with patch.object(mcp_runtime.os, "execvpe") as execute:
            with self.assertRaisesRegex(mcp_runtime.ConfigError, "run ai-profiles update"):
                mcp_runtime.launch(self.project, "postgres")
        execute.assert_not_called()

    def test_runtime_loads_custom_env_and_maps_names_without_shell(self):
        self.local_config({
            "env_file": ".env.stage",
            "environment": {"DATABASE_URI": "{env:STAGE_URL}"},
            "command": [sys.executable, "-c",
                        "import os,sys,json; print(json.dumps([os.environ['DATABASE_URI'], sys.stdin.readline().strip(), os.getcwd()]))"],
        })
        (self.project / ".env.stage").write_text('STAGE_URL="literal-$(never-execute)#value"\n')
        apply_profile(self.context, self.project, "backend-go")
        command = self.cfg()["mcp"]["postgres"]["command"]
        # Launch from outside the project, with an unrelated inherited value.
        result = subprocess.run(command, cwd=self.base, input="stdio-preserved\n",
                                text=True, capture_output=True, check=True)
        self.assertEqual(json.loads(result.stdout),
                         ["literal-$(never-execute)#value", "stdio-preserved", str(self.project)])
        # Runtime reads the local file again, without requiring update.
        (self.project / ".env.stage").write_text("STAGE_URL=changed\n")
        result = subprocess.run(command, input="next\n", text=True, capture_output=True, check=True)
        self.assertEqual(json.loads(result.stdout)[0], "changed")
        self.local_config({"env_file": ".env.stage", "environment": {"DATABASE_URI": "{env:MISSING_TEST_VARIABLE}"}})
        result = subprocess.run(command, text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertNotIn("literal-", result.stderr)

    def test_runtime_dotenv_precedence_and_validation(self):
        path = self.project / ".env.custom"
        path.write_text("# comment\nexport CUSTOM='from-file' # comment\nEMPTY=\nURI=postgresql://host/db#fragment\n")
        entry = {"env_file": ".env.custom", "environment": {"DATABASE_URI": "{env:CUSTOM}"}}
        inherited = {"CUSTOM": "inherited", "KEEP": "keep"}
        result = mcp_runtime.environment_for(self.project, entry, inherited)
        self.assertEqual(result["DATABASE_URI"], "from-file")
        self.assertEqual(result["KEEP"], "keep")
        self.assertEqual(result["URI"], "postgresql://host/db#fragment")
        self.assertEqual(inherited["CUSTOM"], "inherited")
        path.write_text('CUSTOM="secret-unclosed\n')
        with self.assertRaises(mcp_runtime.ConfigError) as error:
            mcp_runtime.environment_for(self.project, entry, {})
        self.assertNotIn("secret-unclosed", str(error.exception))
