---
name: sage-page-sections
description: Create or modify reusable page-bound sections in a Sage 11 theme, such as homepage sliders, banners, cards, contact panels, galleries, and CTA modules. Use for widget-like sections attached to a page, not for the classic WordPress Widget API.
metadata:
  short-description: Build reusable ACF-backed page sections for Sage 11
---

# Sage page sections

Use this skill for widget-like sections that belong to a specific page in the
Sage theme. These are theme sections, not classic WordPress widgets from
`Appearance -> Widgets`.

## Core model

Keep the implementation reusable in code while keeping content assigned to the
page that owns it:

```text
Page + ACF values
  -> Service
  -> View Composer
  -> Blade partial/component
  -> HTML + Tailwind + optional JavaScript
```

ACF stores content and configuration only. Blade owns markup and presentation;
the Service owns WordPress queries, normalization, fallbacks, and page IDs.
Do not put editorial HTML or WordPress queries directly in Blade.

## Placement decision

Classify the requested section before creating files:

1. **Fixed page section** — use when the page has a theme-defined layout, as
   the current homepage does. Add a section partial to the page template and
   attach its ACF fields to the owning page or page type. Template include order
   controls vertical placement.
2. **CMS-reorderable section** — use ACF Flexible Content when administrators
   must add, remove, duplicate, or reorder sections in the panel. Reuse the
   same section partials and services through a controlled layout dispatcher.
3. **Gutenberg block** — use the existing `sage-block-scaffold` skill when the
   user explicitly wants to insert the element inside page content. Do not
   silently convert a fixed homepage section into a Gutenberg block.

For the existing homepage, preserve the fixed-section architecture unless the
user explicitly requests CMS-controlled ordering. Setting a static front page
does not make its editor content render automatically; the theme template still
controls the output.

## Project conventions

- Work under `wp-content/themes/sage/`.
- Store ACF field groups in `resources/acf-json/` and commit the JSON.
- Let the project's ACF hook generate matching `resources/acf-lang/*.php` files;
  never edit generated language files by hand.
- Put page-specific data access in a Service, normally using the front page ID
  from `get_option('page_on_front')` or an explicit page ID supplied by the
  caller.
- Use a View Composer to pass normalized data to Blade.
- Put reusable markup in `resources/views/partials/` or
  `resources/views/components/`; keep fixed homepage wrappers in
  `resources/views/sections/home/`.
- Reuse theme tokens, shared interaction classes, Open Sans, and the existing
  Vite/Tailwind setup. Do not edit `public/build/` manually.
- Use existing dependencies such as Swiper only when the element needs real
  carousel behavior. Load assets conditionally and avoid duplicate enqueue
  hooks.

## Reuse and data ownership

Separate the reusable renderer from page configuration. A news carousel may
share its card partial and query normalization between the fixed homepage
section and an optional `Latest Posts` block, while each page owns its heading,
link, and other ACF values.

When a field such as `hide_on_homepage` describes one placement, pass the
placement or context explicitly to the query. Do not make a homepage-only rule
silently affect archives or unrelated uses of the shared component.

Prefer WordPress-native data sources:

- posts and featured images for news cards;
- ACF fields for page-owned copy, media, links, and switches;
- taxonomies for filtering;
- Flexible Content for administrator-controlled section order.

Do not duplicate existing post content into repeater rows when the section is
supposed to list WordPress posts.

## Implementation workflow

1. Inspect the target page template, comparable section, Service/Composer, ACF
   groups, shared components, tokens, and existing JavaScript behavior.
2. Confirm fixed versus reorderable placement. If ordering in the panel is
   required, choose Flexible Content before writing markup.
3. Define the smallest ACF schema needed for the section, including stable field
   keys, defaults, empty-state behavior, and the correct page location rule.
4. Implement the Service and Composer data flow, then the Blade partial or
   component. Keep the page template responsible for composition and order.
5. Add scoped CSS/JS only where utilities or existing components are
   insufficient. Include preview guards and accessible states for interactions.
6. Verify both frontend output and CMS behavior.

## Required checks

Check:

- empty and partially configured ACF values;
- missing images and long titles or labels;
- correct page-specific data and post filtering;
- desktop, tablet, and mobile layout without horizontal overflow;
- keyboard focus, ARIA labels, semantic headings, and reduced motion;
- editor preview separately from the frontend;
- asset loading only where the section is present;
- section order or Flexible Content ordering, according to the chosen model.

Run the applicable project checks from `AGENTS.md`, including PHP lint, project
lint, PHPStan, tests, and `npm run build` after PHP, Blade, CSS, or JavaScript
changes. Never treat a successful build as proof that CMS fields or page
assignment are configured; verify code and WordPress data separately.
