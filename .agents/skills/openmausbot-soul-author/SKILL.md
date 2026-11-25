---
name: openmausbot-soul-author
description: Create or improve concise, precise OpenMausBot SOUL.md standing instructions from verified bot context. Use when drafting or reviewing a bot's durable role, responsibilities, behavior, boundaries, or delegation guidance.
---

# OpenMausBot SOUL author

Help the user create a useful, reviewable `SOUL.md` that describes how a bot should behave across tasks. Gather only the information needed for the requested change, distinguish evidence from assumptions, and do not invent runtime capabilities.

## 1. Establish scope and context

1. Determine whether the user wants a new SOUL, a revision, or a review. Keep the requested bot and scope fixed; do not redesign its team or access model without a reason.
2. Read relevant supplied context and, when available and authorized, inspect the bot's current description, SOUL, skills, tools, working folder, and profile settings. Prefer inspecting these over asking the user to repeat them. Treat existing text as context, not proof that a capability is actually available.
3. Build a short fact list covering, as relevant: purpose and durable responsibilities; audience; preferred language and style; success criteria; uncertainty and escalation; decision authority and approval boundaries; delegation; actual tools, connected services, and work environment.
4. Ask one concise question at a time only when a missing fact materially changes the SOUL or its safety. Otherwise state a conservative assumption and continue. Do not turn temporary project instructions into standing rules.

## 2. Put each instruction in the right place

- Put stable identity-in-action, recurring responsibilities, communication preferences, and durable operating boundaries in `SOUL.md`.
- Keep a short bot description separate. Use memory for remembered facts, a skill for a reusable procedure, and a task prompt for temporary work.
- Access, approval level, connected apps, tools, and computer choice are runtime/profile settings. SOUL text cannot grant access, create a sandbox, or override the user's current instructions or the bot's governing instructions.
- Include delegation guidance only when delegation is part of the bot's real role. State the bounded outcome and relevant handoff boundary; do not imply a teammate or tool exists unless verified.
- Never include secrets, passwords, access tokens, or one-time codes. Where credentials are needed, point to the platform's secure credential flow without reproducing them.

## 3. Draft the SOUL

Write in the user's preferred language. Use direct, durable instructions with concrete observable behavior. Prefer a few specific rules over repeated slogans. Avoid vague commands, contradictions, impossible guarantees, temporary task details, and unsupported claims about tools, access, expertise, or enforcement.

When facts are unavailable, either omit the detail or mark the assumption for review; do not turn a guess into a capability claim. Make uncertainty actionable (for example, identify when to ask, state an assumption, or escalate) and keep autonomy within the actual approval and access settings.

Keep the draft concise and below 24,000 UTF-8 bytes. That ceiling is stated in the cited OpenMausBot design document; treat it as a design constraint, not independently verified behavior of every current runtime. Leave practical room for future edits.

## 4. Review and present

Before presenting, check that the draft:

- matches the bot's actual role, audience, and durable responsibilities;
- uses clear, checkable behavior and has no conflicting or repeated rules;
- separates standing instructions from description, memory, skills, task prompts, and runtime permissions;
- makes no unsupported capability, access, security, or outcome guarantees;
- handles uncertainty, escalation, delegation, approvals, and secrets appropriately;
- uses the requested language and stays within the documented byte limit.

Show the complete proposed SOUL and briefly call out material assumptions or unresolved points. If the user asked only for a draft or review, do not apply it. If they explicitly requested a profile change, use only the profile proposal/apply capability actually available in the current context and follow its approval flow; report whether it applied, is pending, or failed. Never claim a change or permission update that did not occur.

## Sources

See [references/sources.md](references/sources.md) for the attributed source list and the scope of each source.
