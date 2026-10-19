---
name: mcp-figma
description: Retrieve and organize Figma design context through the official Figma MCP for implementation work. Use when a task needs exact Figma nodes, geometry, styles, states, screenshots, or design assets; pair with a project-specific implementation skill such as styling.
---

# Figma MCP context

Use this skill to obtain reliable, implementation-ready information from
Figma. It is a discovery and handoff skill: collect the design context through
the official Figma MCP and pass it to the implementation workflow. Do not
guess values that the Figma file can provide, and do not paste generated Figma
code into the repository without adapting it to the target project.

## Identify the exact design scope

Before reading the design, establish:

- the Figma file key and exact node ID from a node-specific URL;
- the page, component, variant, or section being implemented;
- whether the node is a reusable component, instance, or one-off frame;
- the desktop and mobile nodes or viewport states, when both exist.

If the URL does not contain a node-specific ID, ask for the precise node
instead of selecting or inventing one.

## Read the design through MCP

Use the `figma-design-to-code` guidance before calling `get_design_context`
when that guidance is available. Use `get_design_context` as the primary read.
Use `get_metadata` when the context does not expose enough of the node tree or
geometry, and use `get_screenshot` for visual comparison at the relevant
viewport sizes.

Inspect the target node together with the ancestors and children needed to
understand its implementation. Capture:

1. **Structure and geometry**
   - child hierarchy and visible semantic roles;
   - layout direction, wrapping, alignment, sizing mode, constraints, and
     positioning;
   - container width, margins, padding, gaps, order changes, and overlap;
   - min/max dimensions and aspect ratios affecting media or controls.

2. **Design values**
   - fills, text colours, borders, opacity, gradients, radii, shadows, and
     other effects;
   - font family, size, weight, line height, letter spacing, alignment,
     casing, and truncation;
   - spacing and breakpoint-specific differences;
   - named variables, styles, and library components exposed by Figma.

3. **States and behaviour**
   - default, hover, focus, pressed, selected, disabled, expanded, and error
     states when present;
   - motion intent, such as image scaling, veil changes, arrow movement, or
     panel opening;
   - keyboard and touch expectations;
   - loading, empty, long-content, and overflow behaviour shown by the design
     or required by the feature.

4. **Visual verification and assets**
   - screenshots of the target at relevant desktop and mobile sizes;
   - asset identity, crop, focal point, export format, and whether an icon or
     illustration already exists in the repository.

Use design context for structure and values, metadata for missing node-tree or
geometry details, and screenshots for composition and comparison. Do not use a
screenshot as the sole source for measurements that node data already exposes.

## Handoff

Pass the implementation workflow a concise summary of the file/node scope,
responsive states, relevant values and variables, interaction states, assets,
and any unresolved design assumptions. Keep material behaviour or responsive
ambiguities explicit so the implementer can ask for clarification instead of
silently creating a new convention.

