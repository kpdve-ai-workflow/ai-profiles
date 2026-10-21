# Container smoke test

From the repository root:

```sh
docker build --tag ai-profiles-playwright:dependency-smoke mcp/images/playwright
sh mcp/images/playwright/tests/run.sh ai-profiles-playwright:dependency-smoke
```

Requires Docker and host `timeout` (GNU coreutils), not host Node/npm. No manual
container startup, network service, browser override, or package installation is
needed. The image's Node starts the production entrypoint over MCP stdio; the
entrypoint drops to UID/GID 1000:1000. The harness also drops to that identity.
The fixture HTTP server is internal to the container, not an MCP HTTP transport.

The test discovers MCP schemas, navigates to a controlled fixture, checks axe
4.14.0 and the `button-name`/`image-alt` violations, captures a PNG, and navigates
again to check the init script survives navigation. It checks that the project
mount is read-only and the nested report mount writable. Raw MCP scan output,
parsed axe results, tool discovery, and the screenshot are retained in the fresh
temporary directory printed by the runner (set `TMPDIR` to choose its parent).
Only that test-created directory is made writable for UID 1000; no existing
project or report permissions are changed. Remove the printed test directory
when its artifacts are no longer needed.

MCP requests have 30-second deadlines, the harness a 120-second deadline, and
Docker a 180-second outer deadline. The runner removes only its uniquely named
container on exit, including failures and interrupts. It never publishes an
image. Output reports the local image architecture and the runtime architecture;
testing one architecture does not verify the other supported architecture.
