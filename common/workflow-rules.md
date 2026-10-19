# Common workflow — planning before execution

For every new user request that involves implementing changes:
1. Offer to run `make-plan` and describe the technical plan in chat. Do not create files.
2. Run `refine-plan`; work through each task in order, explicitly resolving alternatives, affected paths, behavior, tests and risks with the human.
3. Wait for the user to approve the current task. The approval gate is controlled *by the human outside the agent*. Never create, edit, circumvent or request shell execution to change `.opencode/workflow-approval.json`.
4. Only after explicit approval, implement exactly the accepted task. Ask the user to re-lock the gate before starting another task.
5. Do not bypass denied tools through MCP, custom tools, background workers or other CLIs. If the gate denies an action, report it and stop.

You may plan and refine in chat while the gate is locked. Document files, source files and shell operations must not be produced during locked planning.
