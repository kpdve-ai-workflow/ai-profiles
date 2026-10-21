You are the implementation agent. Make the changes described below in the existing project. Follow the instructions in order, minimize unrelated edits, and do not redesign the solution.

OBJECTIVE
Run the existing Rust tests and the existing, unchanged-in-scope Playwright integration test inside a persistent Debian 13 Linux virtual machine managed by Incus, with Docker running inside that VM. Preserve ordinary dependency/build caches between runs.
Do not implement timing measurements, automatic test selection, cached test results, or additional testing frameworks. Python tests are out of scope.

PROJECT CONTEXT
Verified repository details:
- AGENTS.md specifies installation through make install PROFILE=<profile> PROJECT=<project>.
- The root Makefile currently runs:
  - make test: cargo test --manifest-path core/rust/Cargo.toml --locked --all-targets.
  - make rust-build: release builds of all native binaries into core/rust/target/release.
  - make validate-rust-migration: formatting, release build, Clippy, repository validation, and Rust tests. It checks that tests actually ran and that required suites are present.
- core/rust/Cargo.toml defines one Rust package, requires Rust 1.85+, and sets autotests = false. Tests are explicitly registered modules.
- Native payload loading and executable-based repository discovery depend on binaries remaining under the repository's core/rust/target/release. Do not relocate build output without preserving these assumptions.
- The existing Playwright test is:
  - mcp/images/playwright/tests/run.sh
  - mcp/images/playwright/tests/smoke.cjs
  - mcp/images/playwright/tests/create-fixtures.cjs
- Its existing execution commands, from the repository root, are:
  docker build --tag ai-profiles-playwright:dependency-smoke mcp/images/playwright
  sh mcp/images/playwright/tests/run.sh ai-profiles-playwright:dependency-smoke
- The Playwright script performs real make install PROFILE=react, invokes the installed adapter, and exercises Docker Compose, MCP stdio, Chromium, axe, HTTPS, authentication, report paths, permissions, and read-only mounts.
- Its final integration stage requires Node on the machine running the script. In the new setup, that machine is the VM, not the physical host.
- The Playwright test requires a non-root runner and includes fixed UID/GID 1000 checks.
- Relevant documentation: README.md, core/rust/help.txt, mcp/README.md, and mcp/images/playwright/tests/README.md.
- No existing VM infrastructure was found during investigation. Recheck the working tree before editing and preserve user changes.

ARCHITECTURE
Use Incus to manage real QEMU/KVM virtual machines. Use Incus for VM creation, configuration, lifecycle, command execution, and file transfer; use cloud-init for first-boot provisioning. Do not use Vagrant, vagrant-libvirt, libvirt/virsh orchestration, Multipass, or a custom hypervisor manager. Do not substitute an Incus system container for a VM.
Configure:
- Linux x86_64 (amd64) host support for this initial implementation, with a functional local Incus daemon, QEMU/KVM hardware acceleration, storage pool, and VM networking.
- Debian 13 (Trixie) amd64 guest, using a trusted Incus-compatible VM image with cloud-init and incus-agent. Prefer the Incus images remote's Debian 13 cloud variant (typically images:debian/13/cloud); verify the actual alias, architecture, VM image type, and availability before using it. Never silently fall back to Debian 12, a container image, or an unrelated distribution.
- Record/document the actual source image fingerprint or equivalent stable identifier used at creation. Do not carry over the old Vagrant generic/debian12 box name, version, or SHA-256; those belong to a different image format and source.
- Default resources: 4 vCPUs, 8 GiB RAM, and 64 GiB root disk. Allow simple, validated overrides at first creation. Check actual guest filesystem size and available capacity; do not mistake a virtual disk setting for proof that the filesystem grew.
- One reusable, non-ephemeral VM per checkout. Derive a stable, unique, Incus-valid instance name from the canonical checkout path; verify ownership/identity before operating on an existing instance. Never alter another checkout's or another user's VM.
- Docker Engine, Buildx, and Compose plugin inside the VM.
- Rust compilation and test harness directly in the guest, not in a separate Rust container. Docker is used by the existing Playwright integration test only; do not add another Docker daemon layer.
- No host Docker socket, live host-directory mounts, SSH-agent forwarding, exposure of the host home directory, or access to unrelated host filesystem locations.
- Prefer Incus agent operations (incus exec and incus file push/pull) over SSH. Do not add SSH solely for orchestration if the agent provides the required functionality.
If host prerequisites are unavailable, document them and report the blocker. Do not silently install host packages, initialize/reconfigure the host Incus daemon, create host network/storage pools, change host group membership or permissions, switch technologies, or fall back to local tests.

IMPLEMENTATION PLAN
1. Add narrowly scoped VM infrastructure
Create the infrastructure under tests/vm/, including:
- a cloud-init configuration/template for the Incus VM;
- an idempotent guest provisioning script;
- a host orchestration script using the Incus CLI;
- a guest execution script.
Use a dedicated ignored directory such as .test-vm/ for checkout-specific state, locking, run metadata, and exported artifacts. The VM disk and lifecycle belong to Incus; do not pretend the disk is stored in .test-vm/.
Before VM creation, check:
- Linux amd64 host architecture, incus CLI and reachable initialized Incus daemon, adequate permissions, QEMU/KVM support and guest virtualization readiness;
- configured Incus storage and network with usable guest outbound connectivity; required host rsync and file-transfer tools;
- availability of a Debian 13 amd64 cloud-init-enabled VM image with working incus-agent;
- that the selected instance name is free or refers to the VM owned by this checkout.
Produce actionable failures for missing prerequisites. Do not automatically change global Incus profiles, storage pools, networks, daemon settings, or host permissions.
Use incus init/create (not launch) so cloud-init user-data is installed BEFORE the first VM boot; then start the VM. Wait for incus-agent readiness and cloud-init status --wait. Detect provisioning failures and do not report the VM ready until the full provisioning script succeeds. Do not create an incomplete readiness marker.
Provision the guest with:
- Rust/Cargo through rustup (Rust 1.85+; stable toolchain), including rustfmt and Clippy;
- Make, Git, build-essential, required native compilation dependencies, CA certificates, curl, rsync, GNU coreutils, and necessary support utilities;
- Node.js 24 LTS, installed using a trusted upstream distribution/source with version validation;
- Docker Engine, Buildx, and the Compose plugin using Docker's official Debian 13 (Trixie) packages.
Create or validate a dedicated non-root test account with UID 1000 and GID 1000 and a stable home directory (for example /home/tester). Avoid UID/GID conflicts with the base image; if the image cannot satisfy this safely, fail clearly instead of weakening tests. Install Rust for that user, give the user only the guest-local permissions required for the existing Docker tests, and verify that a new non-root execution environment can access cargo, node, Docker, and Compose. Do not run tests as root. Be aware that guest Docker-group membership is root-equivalent within the guest; retain the VM isolation boundary.
Provision only during first creation or explicit re-provisioning. Re-provisioning must be idempotent and must not routinely upgrade, reinstall toolchains, or erase caches when a test is invoked. Create a guest-only readiness marker after all checks pass. Supply an explicit documented re-provisioning command/mechanism that executes the guest provisioning script safely inside the existing VM; updating cloud-init user-data on an already-created VM alone is not sufficient because cloud-init normally runs on first boot.

2. Synchronize current source files without exposing host state
There must be no live host checkout mount inside the VM. Synchronize source files one-way from the current checkout into a stable guest workspace, such as /home/tester/ai-profiles.
Use rsync as the authoritative guest workspace reconciliation step, with explicit, safe handling of symlinks, excludes, stale file removal, and cache protection. A permitted transport is: create a fresh guest staging directory, transfer an allowlisted/excluded source snapshot through incus file push (or a safe tar stream through incus exec), then use guest-local rsync to reconcile staging into the stable workspace. If choosing another reliable Incus-compatible rsync transport, document and test it. Do not use live mounts, broad host file sharing, or unaudited SSH forwarding. Avoid copying large host dependency/build directories into staging.
Requirements:
- Include current modified and new source files; do not test only committed HEAD.
- Remove stale source files from the guest when they disappear from the host.
- Preserve executable bits, hidden build-context files such as .dockerignore, and source symlinks as symlinks. Do not follow symlinks into unrelated host locations. Explicitly avoid rsync --copy-links and any file-transfer option that dereferences source symlinks.
- Exclude .git, .opencode, .test-vm, exported artifacts, dependency directories, host build output, local credentials such as .env files, and other host-private files.
- Do not copy the host Cargo configuration, home directory, SSH keys, or Docker configuration.
- Preserve guest core/rust/target/ on synchronization, including deletions from the source tree; do not use rsync --delete-excluded in a way that removes guest caches. Keep guest Cargo home caches outside replaceable workspace content.
- Avoid introducing host UID/GID ownership into guest files. Ensure the synced workspace is owned by the UID/GID 1000 test user while preserving executable modes and symlink safety.
- Quote paths correctly, including host checkout paths containing spaces and unusual safe filename characters.
- Do not leave stale source staging directories containing deleted files or private material.
Serialize all operations involving the checkout's VM and workspace with a checkout-specific lock. If another operation is active, fail clearly rather than synchronizing over a running test. Acquire the same lock for test, VM up/stop, sync, and re-provisioning operations.

3. Keep only ordinary tool/build caches
Persist inside the VM:
- Cargo registry and Git dependency caches in the non-root user's Cargo home;
- core/rust/target/, including debug and release artifacts;
- Docker images and build layers on the VM's persistent disk.
Keep build output at the path expected by the existing Makefile and payload code.
Always execute requested tests. Never reuse previous successful test results to skip execution.
For Playwright, run docker build on each requested Playwright invocation and let Docker decide which layers are reusable. Do not skip building merely because the image tag exists.
Do not routinely run cargo clean, prune Docker, recreate the VM, or erase caches. Use fresh test scratch directories and fresh synthetic credentials for every run. Document how to inspect disk usage; do not introduce automatic destructive pruning.

4. Preserve existing test behavior inside the guest
Implement these public Make targets:
- make test: existing complete Rust test command, executed in the VM.
- make test-playwright: build the Playwright smoke image and run the existing complete Playwright script in the VM.
- make validate-rust-migration: preserve the current validation gate, but execute it in the VM.
- make test-all: execute the existing validation gate, which already includes Rust tests, followed by the existing Playwright test. Do not run Rust tests twice.
- make test-vm-up: prepare/start the reusable Incus VM without running tests.
- make test-vm-stop: stop this checkout's VM without deleting its disk or caches.
For each test entry point, automatically start the owned VM if necessary, wait for readiness, synchronize sources, execute the guest operation, and export artifacts.
Use incus exec to invoke a fixed, allowlisted guest mode, never arbitrary user-provided remote shell commands. Ensure the actual test process runs as the UID/GID 1000 account with correct HOME, PATH, Cargo, Node, and Docker group access (use a guest login/user-switching mechanism if necessary). Incus exec defaults to root; do not accidentally run test workloads as root.
Keep the original Rust and validation recipes as guarded guest-only internal targets rather than duplicating their logic. Preserve the validation gate's failure reporting, nonzero-test check, and required-suite checks.
Handle nested Make calls explicitly: the validation gate invokes make test, and Playwright invokes make install. Inside the provisioned guest, test dispatch must execute directly without launching another VM, resynchronizing sources, or reacquiring the host lock. Use the guest provisioning marker for this distinction, not merely a user-supplied environment flag.
Leave ordinary production operations—including make install, update, uninstall, switch, status, validate, and rust-build—with their existing local behavior.

5. Keep the existing Playwright integration test intact
Reuse run.sh, smoke.cjs, production Dockerfile, entrypoint, and Compose configuration.
Do not:
- replace MCP testing with direct Playwright API tests;
- mock Docker or the installed adapter;
- remove the installed-adapter stage;
- remove HTTPS, authentication, UID/GID, screenshot-path, MCP roots, or read-only mount checks;
- weaken TLS verification;
- introduce privileged containers or mount any Docker socket.
Prefer leaving existing test scripts unchanged. Only make a small compatibility adjustment if genuinely required by VM execution, preserving all assertions.
The adapter integration's network_mode: host must refer to the VM's network namespace. No physical-host network assumption should be needed.

6. Preserve logs, artifacts, and failures
Create a unique guest run directory for each invocation. Set TMPDIR appropriately so the existing Playwright script creates its disposable project beneath that run directory.
Export to a unique directory under .test-vm/artifacts/:
- operation logs;
- Playwright reports, JSON outputs, and screenshots.
Use a narrow allowlist for artifact paths and Incus file pull or an equivalently safe Incus transfer. Export after success and failure when the guest and its agent are reachable. Do not export generated private keys, authentication storage state, the whole temporary project, or unrelated guest files.
Print the host artifact path.
Preserve nonzero exit status through Make, incus exec, logging, and cleanup. If tests fail, an export error must not hide the original failure. If tests pass but required artifact export fails, return a clear failure.
Preserve existing test deadlines and ensure ordinary interruption forwards termination to the owned guest test process/process group so its existing cleanup can remove its containers. Do not stop the VM as an interruption shortcut; do not stop unrelated containers or VMs. Preserve available diagnostics.

7. Update documentation and help
Update:
- README.md;
- core/rust/help.txt;
- mcp/README.md;
- mcp/images/playwright/tests/README.md;
- .gitignore.
Add a focused docs/testing.md if needed and link it from the README.
Keep documentation in Polish, matching the repository.
Explain:
- required Linux amd64 host virtualization support, Incus CLI/daemon initialization, QEMU/KVM, available image source, host rsync, Incus network and storage pools, and permissions;
- that Incus administrative socket access is highly privileged, and that host setup must be performed consciously by the user rather than automatically by tests;
- the additional requirements apply to testing; local profile installation still has its existing Rust requirements;
- that the host does not need Cargo, Node, or Docker to run VM-backed tests;
- that provisioning uses Debian 13, Node.js 24 LTS, Docker in the VM, and requires first-run network access;
- available test and VM lifecycle commands;
- which caches persist and that tests themselves always rerun;
- where artifacts are stored;
- how to explicitly re-provision after provisioning changes, without destroying the VM;
- the initial VM implementation supports amd64 only;
- VM resource defaults and supported validated overrides, including the 64 GiB root disk;
- how the image identifier/fingerprint is recorded, and that availability must be checked before initial creation.
Fix the existing inaccurate Playwright test README claim that Node is unnecessary: Node is required inside the VM for the installed-adapter stage.

BEHAVIOR AND EDGE CASES
- Missing Incus CLI/daemon, Incus permissions, configured storage/network, available Debian 13 cloud VM image, incus-agent, or KVM support: actionable failure, never local fallback.
- Existing stopped VM: restart and reuse its caches; validate instance ownership before accessing it.
- Provisioning failure: nonzero result; do not mark the guest ready. Do not automatically destroy/recreate the VM.
- Unknown execution mode: reject it; do not accept arbitrary remote command strings.
- Concurrent invocation: clear checkout-specific lock failure.
- Changed or removed source: reflected in the next run while guest caches survive.
- Failed Docker build: fail before running against an older image.
- Rust failure or missing required suite: preserve current failure semantics.
- Interrupted tests: terminate the owned run and preserve available diagnostics.
- If incus-agent is unreachable, report a specific error and preserve the VM for diagnosis; do not silently switch to SSH or another VM type.
- Instance-name collision or mismatched checkout identity: fail without modifying the existing instance.
- Avoid accidentally invoking Incus against a remote daemon: use the intended local instance/server and validate the target.
- Keep runtime state, credentials, cache contents, and artifacts out of Git.

CONSTRAINTS / DO NOT CHANGE
- Do not add or resurrect Python tests or a Python test runner. Do not delete unrelated legacy files as part of this task.
- No timing instrumentation, performance dashboards, benchmarks, or duration-based policies. Existing test timeout safeguards must remain.
- No SUITE/FILTER features, changed-file selection, cached test outcomes, nextest, sccache, distributed caches, or CI redesign.
- No application refactors, profile-version changes, or production Playwright behavior changes.
- Do not install host dependencies, initialize or reconfigure host Incus storage/network/permissions automatically, use Vagrant/libvirt as a fallback, or silently change the guest OS/runtime versions.
- Do not use broad destructive cleanup or discard user changes.

ACCEPTANCE CRITERIA
- Public Rust and Playwright test commands execute their actual workloads only inside the Debian 13 Incus VM.
- make test-all runs the existing validation gate and the full existing Playwright integration test.
- The physical host does not execute Cargo, Node, or Docker for these operations.
- The real installer-to-adapter-to-Compose-to-browser chain remains covered.
- A second invocation reuses Cargo/build/Docker caches but executes tests again.
- Source synchronization includes current changes and does not retain removed source files.
- Host build output is neither copied into the guest nor modified by VM tests.
- Logs and Playwright reports are available on the host after successful and failed runs, without exported test credentials.
- VM shutdown preserves caches.
- Existing local installation commands retain their behavior.
- Documentation matches implemented commands and prerequisites.
- No Vagrant, vagrant-libvirt, or libvirt provider dependency is introduced.

VERIFICATION
After implementation:
1. Run syntax/configuration checks for the new shell scripts and cloud-init configuration (including cloud-init YAML validation where the required tooling is available).
2. Add focused regression checks for orchestration boundaries where practical: missing prerequisites, unknown modes, exit-status propagation through incus exec, non-root guest dispatch, cache protection, instance ownership, and synchronization exclusions. Use lightweight shell-based checks, not a new testing framework.
3. On a virtualization-capable host with Incus configured, run:
   make test-vm-up
   make test
   make test-playwright
   make validate-rust-migration
   make test-all
4. Verify inside the guest: Debian 13, Node.js 24 LTS, Rust 1.85+, Docker/Compose, effective UID/GID 1000 for test work, functional Docker permissions, actual available filesystem size, and cloud-init readiness.
5. Repeat a test invocation and confirm cache directories/images remain present while tests execute again. Do not add timing measurements.
6. Verify stop/start reuse with:
   make test-vm-stop
   make test
7. Exercise an intentional failing run using a disposable test fixture and confirm nonzero status and exported logs. Do not leave failure fixtures or change existing test expectations.
8. Check that source edits and deletions synchronize correctly, symlinks are not followed into unrelated host paths, and guest target cache survives; inspect the host diff for accidental generated files or unrelated changes.
9. Check normal interruption: terminate only the owned guest run and retain available logs, without deleting the VM or affecting unrelated processes.
If real VM execution is blocked by missing tooling or permissions, report exactly what could and could not be verified. Mocked orchestration checks are not evidence that the real Playwright E2E passed.

DELIVERY
Implement the changes directly. Afterward, summarize the files changed, commands delivered, cache/artifact behavior, verification results, and genuine blockers. Do not claim tests passed unless they were actually executed.