---
name: styling
description: Implement or refine frontend styling in a Sage 11 theme. Use whenever the user asks to style, restyle, or visually refine a page, section, component, or block; verify the project's Tailwind 4 tokens, semantic component conventions, responsive rules, and accessibility requirements. Pair with sage-block-scaffold when creating a completely new block.
---

# Sage styling

Use this skill when the target is the Sage 11 theme in
`wp-content/themes/sage/` and the visual requirements are available from a
design specification or from `mcp-figma`. Fit the implementation to the
existing design system and codebase rather than creating a one-off styling
system.

## Inspect the repository before styling

Before writing markup or CSS, inspect the target Blade view, its Composer or
Service when relevant, and any block JavaScript. Search surrounding views and
stylesheets for the closest existing visual pattern. Confirm whether the
component can reuse an existing semantic style, shared interaction pattern, or
theme token.

For the styling baseline, read the applicable files before making decisions:

- `resources/css/app.css` — Tailwind entry point, import order, and scan sources;
- `resources/css/common/colors.css` — design colour tokens and aliases;
- `resources/css/common/fonts.css` — font face, typography scale, and element defaults;
- `resources/css/common/layout.css` — containers, WordPress layout rules, and content defaults;
- `resources/css/common/interactions.css` — shared hover, media, link, and button behaviour;
- `resources/css/elements/buttons.css` and `forms.css` when controls are involved;
- the relevant section, page, or block stylesheet and nearby Blade templates.

Treat the current repository implementation as the source of truth for naming,
cascade, and reusable patterns. Do not create a selector inventory in the
skill, comments, or task response; report only patterns relevant to the
component being changed.

## Translate the design into theme conventions

Apply the following order of preference:

1. existing semantic component styles;
2. existing theme tokens and named typography values;
3. Tailwind utilities composed in Blade markup;
4. a narrowly scoped stylesheet rule for behaviour or geometry that utilities
   cannot express cleanly.

### Tokens first

Map design values to existing theme tokens before adding new values. Use the
theme's colour variables for brand, action, surface, ink, border, and muted
states. Use the configured Open Sans family and named typography scale. Do not
put repeated hexadecimal values, arbitrary font declarations, or copied design
variable names directly into Blade.

Add a new theme token only when the value is a reusable design-system value or
clearly belongs to the global palette. A one-off adjustment belongs to the
local component and should be justified by the design.

### Utility-first composition

Tailwind CSS 4 is the default styling mechanism. Compose layout, spacing,
visibility, sizing, typography, colour, and responsive changes with utilities
when they are clear and local to the markup.

Use arbitrary values only when all of the following are true:

- the design provides an intentional value;
- no existing token or scale expresses it;
- the value is local rather than a new global convention.

Do not use arbitrary values to compensate for an incorrect container, font,
line height, or token mapping. Fix the underlying convention first.

### Semantic CSS and cascade

Keep reusable component defaults in the appropriate element or shared
stylesheet and place them in the same cascade layer as comparable code. Keep
block-specific rules in the block stylesheet and section/page rules in their
corresponding stylesheet. Avoid broad global selectors for a single block.

Follow the project's established cascade: component defaults should remain
overridable by local utilities, while shared interactive states must retain
the layer ordering that lets them work alongside text-colour and spacing
utilities. Do not solve a cascade conflict with `!important` unless the
surrounding file has a documented WordPress or third-party reason for it.

Use the existing component naming shape for new semantic selectors: a stable
component root, consistent element and modifier relationships, and no names
describing a temporary colour or pixel measurement. Keep JavaScript hooks
separate from visual selectors when the component has behaviour.

### Responsive implementation

Implement mobile-first. Treat the smallest design state as the base and add
only the changes required at the project's established breakpoints. Reuse the
theme's container and layout conventions. Check:

- column count and stacking;
- order and alignment;
- readable type and line length;
- image crop and aspect ratio;
- control size and tap target;
- overflow, wrapping, and long labels.

Do not force a desktop frame to scale down with fixed heights when the design
shows a different mobile composition. Prefer content-driven sizing unless the
design clearly requires a fixed or minimum dimension.

### Interactions and accessibility

Reuse the shared interaction language for buttons, links, arrows, cards, and
media. Add a component-specific interaction only when the design shows a
genuinely different state. Preserve visible keyboard focus, correct disabled
behaviour, and sufficient contrast. Do not communicate state through colour
alone.

Keep colour choices compatible with the project's high-contrast mode by using
theme tokens instead of hard-coded fills and text colours. Check
focus-visible styles, hover equivalents for touch devices, reduced-motion
expectations, and the semantics of buttons versus links.

## Implement in a controlled order

After the design context and repository inspection:

1. preserve the existing data flow and semantic HTML unless the target design
   requires a structural correction;
2. map the design to existing tokens and reusable component styles;
3. establish the responsive container and layout;
4. add typography, surfaces, borders, media treatment, and spacing;
5. implement interaction states and motion;
6. add only the smallest local CSS needed for the remaining mismatch;
7. keep asset loading compatible with Vite and the existing theme helpers.

Do not edit `public/build/` manually. Do not add a new dependency for static
styling. Use existing project libraries only when the interaction actually
requires them.

## Verify the result

Run the relevant checks from the repository after implementation:

```bash
cd wp-content/themes/sage && npm run build
```

For PHP or Blade changes, also run the applicable PHP lint, project lint, and
tests described in `AGENTS.md`. Inspect the rendered result at the reference
sizes and at an intermediate width. Compare structure and visual hierarchy,
not just isolated pixels.

Verify at minimum:

- desktop and mobile composition;
- container edges, spacing rhythm, typography, and image crops;
- all provided interactive states;
- keyboard focus and touch-sized controls;
- long text, empty data, and overflow cases;
- high-contrast mode when the component uses colour or media;
- absence of horizontal overflow and unintended global side effects.

If the design leaves a material behaviour or responsive state undefined,
record the assumption and ask for clarification rather than silently creating
a new convention.

## Completion report

Summarize the existing project conventions reused, files changed, deliberate
deviations or assumptions, and checks that passed. Do not paste a full class
list; mention only the relevant styling patterns for the component.
