---
name: sage-block-scaffold
description: Create a new Sage 11 WordPress ACF block from a Figma design, screenshot, or written specification, including ACF JSON, PHP data flow, Blade, and optional CSS/JavaScript. Use for new blocks, not for modifying existing blocks.
---

# Sage block scaffold workflow

Create a complete, project-compatible ACF block in `wp-content/themes/sage/`.

## 1. Define the block

Use the first available input:

- Figma URL: extract `fileKey` from the path after `/design/` and `nodeId` from the `node-id` query parameter, replacing `-` with `:`. Use the design-context integration to inspect layout, fields, assets, fonts, colors, and interaction signals.
- Screenshot: derive the layout, editable fields, likely ACF types, and interaction signals. List content/decorative assets that require manual placement.
- Written specification: identify the block name, content fields, and behavior from the request.

If no input is available, ask whether the user wants to provide a Figma URL, screenshot, or description. Ask for the block name and resolve ambiguities before creating files.

Read [reference/conventions.md](reference/conventions.md) before designing the schema or markup. Read the base templates before creating files:

- `templates/block.json.tmpl`
- `templates/acf-group.json.tmpl`
- `templates/Service.php.tmpl`
- `templates/Composer.php.tmpl`
- `templates/blade.blade.php.tmpl`

Read the JavaScript and Controller templates from `templates/` just in time, only if the block needs them.

## 2. Derive names and dependencies

Derive these consistently using the conventions reference:

- `{PASCAL_NAME}` — PHP class and block directory;
- `{KEBAB_NAME}` — block slug, Blade, CSS, and JavaScript;
- `{SNAKE_NAME}` — ACF group and field keys;
- `{TITLE}` — editor-facing title.

Choose only the libraries the design requires:

- Swiper for carousels and sliders;
- Locomotive Scroll for reveal-on-scroll, parallax, or scroll-driven effects;
- Alpine.js only when Blade binds to reactive state with `x-show`, `x-text`, or `x-bind`;
- no library for static markup and Tailwind utilities.

Do not add GSAP. Swiper and Locomotive Scroll are already dependencies of the theme.

## 3. Check for collisions

Before writing anything, check whether any of these already exist:

```text
wp-content/themes/sage/app/Blocks/{PASCAL_NAME}/
wp-content/themes/sage/app/Services/{PASCAL_NAME}Service.php
wp-content/themes/sage/app/View/Composers/{PASCAL_NAME}.php
wp-content/themes/sage/resources/views/blocks/{KEBAB_NAME}.blade.php
wp-content/themes/sage/resources/acf-json/group_{SNAKE_NAME}.json
```

If a path exists, report the collision and stop rather than overwriting it.

## 4. Plan fields and assets

Map editable content to ACF types using the conventions reference. Keep `get_field()` calls in the Service; the Composer passes normalized data to Blade.

Download only small UI icons from Figma when needed. Do not download decorative or content assets such as photos, masks, or background shapes; use CSS or leave them for the user to provide through ACF. Use `Vite::content()` for inline SVG and `Vite::asset()` for raster or built assets.

If the schema is complex or the design leaves important behavior ambiguous, confirm the field model with the user before creating files.

## 5. Create the block

From the templates, create:

1. `app/Blocks/{PASCAL_NAME}/block.json`;
2. `resources/acf-json/group_{SNAKE_NAME}.json`;
3. `app/Services/{PASCAL_NAME}Service.php`;
4. `app/View/Composers/{PASCAL_NAME}.php`;
5. `resources/views/blocks/{KEBAB_NAME}.blade.php`.

Set `jsx` to `true` only when the block uses InnerBlocks. Keep the Blade wrapper and preview/link guards from the conventions reference. Use an empty state for repeaters in the editor.

When required, also create:

- `resources/js/blocks/{KEBAB_NAME}.js` from the matching template: Swiper, scroll, combined, or Alpine;
- `resources/css/blocks/{KEBAB_NAME}.css` only for keyframes, complex pseudo-elements, transitions, or third-party overrides;
- `app/Controllers/{PASCAL_NAME}Controller.php` only for global WordPress hooks.

Block directories are auto-discovered by `ThemeServiceProvider`; do not manually register the block there. A Controller used for global hooks does need to be added to its controller collection with a `use` import.

For Swiper, use the shared `sage-swiper` style handle and declare it as a dependency for block-specific Swiper overrides. Vite discovers block CSS and JavaScript files; enqueue them through the Composer pattern shown in `templates/Composer.php.tmpl`.

## 6. Verify

From the repository root, run the checks that apply:

```bash
ddev exec composer run lint:fix
ddev exec php -l wp-content/themes/sage/app/Services/{PASCAL_NAME}Service.php
ddev exec php -l wp-content/themes/sage/app/View/Composers/{PASCAL_NAME}.php
ddev exec php -l wp-content/themes/sage/app/Controllers/{PASCAL_NAME}Controller.php  # if created
cd wp-content/themes/sage && npm run build
```

Check the block in the editor preview and on the frontend. If it uses interaction, check initialization, responsive behavior, and the editor preview separately.

## Completion output

Report the block slug and title, ACF fields, selected libraries, asset handling, files created, checks run, and how to test it locally.
