---
name: sage-cpt-scaffold
description: Create a complete Custom Post Type for the Sage 11 WordPress theme, with optional taxonomy, controller, archive and single Blade views, and ThemeServiceProvider registration. Use for new CPTs, not for changing an existing post type.
---

# Sage CPT scaffold workflow

Generate a project-compatible WordPress Custom Post Type in `wp-content/themes/sage/`, following the established `ProjectsController.php` pattern.

## 1. Parse the CPT name

Use the plural kebab-case slug from the request, for example `events`, `team-members`, or `case-studies`. If it is missing, ask for it before continuing.

Derive and confirm all forms:

| Form | Example (`team-members`) | Usage |
| --- | --- | --- |
| `{PLURAL_SLUG}` | `team-members` | `post_type`, rewrite slug, REST base |
| `{SINGULAR_SLUG}` | `team-member` | singular references |
| `{PLURAL_PASCAL}` | `TeamMembers` | controller class and file |
| `{SINGULAR_PASCAL}` | `TeamMember` | labels |
| `{PLURAL_LABEL}` | `Team Members` | human-readable plural |
| `{SINGULAR_LABEL}` | `Team Member` | human-readable singular |

Convert kebab-case to title case, then to PascalCase. Do not blindly remove a trailing `s` when the singular is irregular; ask the user to confirm cases such as `categories` → `category`.

## 2. Resolve options

Ask only for information not already supplied:

1. Whether a taxonomy is needed; if yes, get its slug and plural/singular labels.
2. The menu icon; default to `dashicons-admin-post`.
3. Supported editor features; default to `['title', 'thumbnail', 'editor']`.

Use a hierarchical taxonomy unless the user explicitly requests a tag-like taxonomy. Keep `show_in_rest` enabled unless the request requires otherwise.

## 3. Inspect the project and check collisions

Read these project files before editing:

- `wp-content/themes/sage/app/Controllers/ProjectsController.php`;
- `wp-content/themes/sage/app/Providers/ThemeServiceProvider.php`;
- `templates/Controller.php.tmpl`;
- `templates/archive.blade.php.tmpl`;
- `templates/single.blade.php.tmpl`;
- `templates/content-single.blade.php.tmpl`.

Before creating any file, check for:

```text
wp-content/themes/sage/app/Controllers/{PLURAL_PASCAL}Controller.php
wp-content/themes/sage/resources/views/archive-{PLURAL_SLUG}.blade.php
wp-content/themes/sage/resources/views/single-{PLURAL_SLUG}.blade.php
wp-content/themes/sage/resources/views/partials/content-single-{PLURAL_SLUG}.blade.php
```

If a path exists, report the collision and stop rather than overwriting it.

## 4. Create the CPT files

From the templates, create:

1. `app/Controllers/{PLURAL_PASCAL}Controller.php` with the post type and, when requested, taxonomy registration;
2. `resources/views/archive-{PLURAL_SLUG}.blade.php`;
3. `resources/views/single-{PLURAL_SLUG}.blade.php`;
4. `resources/views/partials/content-single-{PLURAL_SLUG}.blade.php`.

Fill every placeholder, translate labels with the `sage` text domain, and preserve the project’s strict typing and WordPress registration conventions. Do not include taxonomy code when no taxonomy was requested.

## 5. Register the Controller

Edit `wp-content/themes/sage/app/Providers/ThemeServiceProvider.php`:

- add `use App\\Controllers\\{PLURAL_PASCAL}Controller;`;
- add `{PLURAL_PASCAL}Controller::class` to the `collect([...])` controller list in `boot()`.

Do not manually register the CPT anywhere else.

## 6. Verify

From the repository root, run:

```bash
ddev exec composer run lint:fix
ddev exec php -l wp-content/themes/sage/app/Controllers/{PLURAL_PASCAL}Controller.php
```

After deployment or local registration, visit WordPress **Settings → Permalinks** and save once to flush rewrite rules. Check the admin sidebar, REST endpoint, archive, single view, and taxonomy UI when applicable.

## Completion output

Report the CPT slug and singular/plural labels, taxonomy and options, files created or edited, checks run, and the manual permalink step needed for testing.
