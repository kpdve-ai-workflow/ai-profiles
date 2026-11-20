---
name: acf
description: Work with Advanced Custom Fields in this Sage 11 WordPress project, especially when using Novamira MCP to inspect or change local WordPress data. Use for ACF field groups, options pages, option values, repeaters, JSON sync, and ACF-backed theme rendering.
---

# ACF and Novamira MCP workflow

Use this skill for ACF work in WordPress, with Sage 11 architecture guidance.
The data-model, theme paths and footer examples below were inherited from a
specific project: they are not proof of the current project's schema or content.
Inspect the actual repository and site before applying them. Do not create the
example field groups or content unless the user's task calls for them.

## Project baseline

Use `mcp-novamira` to discover the configured connection and verify its host and
blog ID. Do not assume DDEV, multisite, a blog ID or a previous project's domain.
Inspect WordPress/PHP/ACF versions, available ACF APIs and environment type.
Multisite blogs have separate data; select a different site only when explicitly
requested. Ask if the intended target cannot be established safely.
The target-mismatch hard stop in `mcp-novamira` also applies to ACF options,
menus and PHP execution: an unrecognized requested domain or a different
discovered host/blog ID blocks writes until the user explicitly clarifies the
authorized target and it is verified read-only. Never fall back to blog `1`.

## Repository architecture

The theme is `wp-content/themes/sage/`.

ACF field groups are stored as JSON in:

```text
wp-content/themes/sage/resources/acf-json/
```

ACF JSON is the repository source of truth for field-group schema. Commit field
group JSON when changing fields. Do not manually edit generated translation
files in `resources/acf-lang/`.

Use this data flow for theme code:

```text
ACF / WordPress
  -> Service
  -> View Composer
  -> Blade
```

Keep `get_field()` and value normalization in a Service. Pass normalized values
from a Composer to Blade. Do not put complex WordPress queries or ACF data
transformation in Blade.

Theme code belongs in the repository. Do not use Novamira `write-file` or
`edit-file` to modify PHP, Blade, CSS, JavaScript, or ACF JSON in the theme.
Use the local code-editing workflow and commit the resulting code changes.

Novamira is local-only runtime tooling. Its installed files are intentionally
ignored by Git:

```gitignore
/wp-content/plugins/novamira/
/wp-content/novamira-sandbox/
```

Never commit Novamira plugin files, sandbox files, OAuth data, passwords, or
tokens.

## ACF naming and admin language convention

Separate technical identifiers from text displayed to WordPress administrators.
The project uses English for stable code-facing values and Polish for every
custom label, description, instruction, button, tab, menu-location label, and
group title visible in the Polish WordPress admin.

| ACF element | Technical value | Text visible in the admin |
| --- | --- | --- |
| Field group title | stable key stays unchanged | Polish, e.g. `Ustawienia układu strony` |
| Field label | field `name` stays English `snake_case` | Polish, e.g. `Ukryj sidebar` |
| Field name | `hide_sidebar` | not displayed as the editing label |
| Field key | `field_sidebar_hide_sidebar` | never translate or regenerate without a migration |
| Group key | `group_sidebar_layout` | never translate |
| Instructions and messages | not used as PHP identifiers | Polish, clear and actionable |
| Repeater button | not used by code | Polish, e.g. `Dodaj element` |
| Select/radio choice value | stable English value, e.g. `plant` | Polish label, e.g. `Roślina` |

For example, the Contact field group should be presented in the admin as:

| Current technical name | Polish admin label |
| --- | --- |
| `Page: Contact` | `Strona: Kontakt` |
| `contact_page_title` | `Tytuł` |
| `contact_page_map_embed` | `Mapa — kod osadzania` |
| `contact_page_contact_items` | `Dane kontaktowe` |
| `contact_page_accordions` | `Akordeony kontaktowe` |
| `contact_page_table_title` | `Tytuł tabeli kontaktowej` |
| `contact_page_table_rows` | `Wiersze tabeli kontaktowej` |
| `contact_page_table_notes` | `Przypisy pod tabelą kontaktową` |

Nested labels and buttons follow the same rule: `Ikona`, `Treść`, `Tytuł`,
`Dziedzina`, `Biuro we Wrocławiu`, `Biuro w Wałbrzychu`, `Biuro w Jeleniej
Górze`, `Biuro w Legnicy`, `Dodaj dane kontaktowe`, `Dodaj akordeon` and
`Dodaj wiersz`.

The same distinction applies outside ACF. For a registered menu location, keep
the technical key `news_sidebar_navigation`, but expose a Polish label such as
`Nawigacja boczna serwisu`. Do not expose `Site Sidebar Navigation` to editors
when the rest of the WordPress admin is Polish.

Do not translate or rename technical values used by PHP, ACF lookups, JSON
locations, URLs, slugs, CSS classes, JavaScript hooks, field keys, group keys,
or stored choice values. Changing a visible label normally does not require a
data migration; changing a field `name` or `key` does. When localizing an
existing group, preserve its keys and names, update only the visible labels and
instructions, then synchronize the JSON group.

The WordPress core and ACF interface may use their own Polish translation. This
rule concerns custom project UI only. Before handing over a panel change,
inspect the actual Polish admin screen and remove accidental English custom
strings such as `Contact`, `Title`, `Add accordion`, or `Hide sidebar`.

Use Polish admin terminology in user instructions where it is visible in the
installation, for example `ACF → Grupy pól`, `Dostępne do synchronizacji` and
`Synchronizuj`. Technical English names may still be included in parentheses or
code blocks so developers can match them to JSON and PHP.

## Options pages

The main theme options page is registered by:

```text
wp-content/themes/sage/app/Controllers/ConfigController.php
```

Its slug is `sage_config_options`.

The footer options subpage is registered by:

```text
wp-content/themes/sage/app/Controllers/FooterSimpleRowController.php
```

Its slug is `sage_footer_simple_row_options`. A field group targeting this
options page should use this ACF JSON location:

```json
"location": [[
  {
    "param": "options_page",
    "operator": "==",
    "value": "sage_footer_simple_row_options"
  }
]]
```

ACF option values are read with the `option` post ID:

```php
get_field('field_name_or_key', 'option');
update_field('field_key', $value, 'option');
```

Prefer field keys in `update_field()` because they are unambiguous when a new
field has not yet stored a value. Use field names for readable inspection when
the field is already registered.

## Existing footer ACF model

The footer group is:

```text
wp-content/themes/sage/resources/acf-json/group_67594c4e8e4b8.json
```

Its options-page fields include:

| Key | Name | Type | Purpose |
| --- | --- | --- | --- |
| `field_67594c4e8aacf` | `footer_logo` | image, URL | Main footer logo |
| `field_67594c818aad0` | `footer_logo_retina` | image, URL | Retina footer logo |
| `field_67594c908aad2` | `footer_socials` | repeater | Label, URL, and image/icon per social link |
| `field_67594cf48aad3` | `footer_copyright` | text | Copyright template; supports `[since]` |
| `field_67595cae6d513` | `footer_copyright_since` | number | Copyright start/current year |
| `field_footer_accessibility_text` | `footer_accessibility_text` | text | Additional WCAG text below copyright |

The current default content in the repository is:

```text
Copyright © [since] Wojewódzki Fundusz Ochrony Środowiska i Gospodarki Wodnej we Wrocławiu
Wykonanie zgodnie z wymogami WCAG 2.1 AA dla jednostek publicznych.
```

`FooterSimpleRowService::fetch_copyright()` replaces `[since]` with a year
range/current year. If the option is empty, it uses the repository fallback.
`fetch_accessibility_text()` also has a repository fallback. Preserve these
fallbacks so a fresh local database still renders useful content.

The footer controller registers these menu locations:

```text
footer_menu_column_1_navigation
footer_menu_column_2_navigation
footer_menu_column_3_navigation
footer_menu_column_4_navigation
footer_menu_column_5_navigation
footer_bottom_navigation
```

The four bottom links in the current main-site local data are:

```text
Mapa strony
Polityka prywatności
Cookies
Deklaracja dostępności
```

`Realizacja: Ideo` is intentionally not part of the footer implementation.
Bottom links are WordPress menu data, not ACF repeater data, because the theme
already consumes `footer_bottom_navigation` through `wp_get_nav_menu_items()`.

## Novamira MCP workflow

For WordPress data work, use the MCP server matching the target site. The
available adapter tools are:

```text
mcp_adapter_discover_abilities
mcp_adapter_get_ability_info
mcp_adapter_execute_ability
```

The profile configures a server named `novamira`. Discover its client-visible
tool names in the current session; clients may normalize names differently.

Before using Novamira:

1. Select the correct site and MCP server.
2. Call `mcp_adapter_discover_abilities`.
3. Call `mcp_adapter_get_ability_info` for every ability you intend to use.
4. Verify the actual site against the authorized target using read-only abilities, following `mcp-novamira`. A site listing is not authorization to switch targets.
5. Only then execute the requested operation. Stop before any mutation if the host/blog or target object remains uncertain, including when using `execute-php`.

There is no dedicated ACF ability in the verified Novamira installation. Use
the ACF/WordPress APIs through `novamira/execute-php` after inspecting its
schema. Its input is:

```json
{
  "code": "...PHP without <?php tags..."
}
```

The PHP execution ability is temporary and has a 30-second limit. It has the
full WordPress and loaded-plugin environment, including `$wpdb`, ACF functions,
`get_field()`, `update_field()`, menu APIs, and options APIs. Always `return`
structured data for inspection. Never call `exit()` or `die()` and never create
an infinite loop.

Use Novamira for local WordPress data, for example:

```php
return [
    'copyright' => get_field('footer_copyright', 'option'),
    'year' => get_field('footer_copyright_since', 'option'),
    'accessibility' => get_field('footer_accessibility_text', 'option'),
];
```

For field-group inspection, `acf_get_fields('group_key')` is more useful than
counting `acf_get_field_group()['fields']`, because the group metadata may not
contain expanded fields in the returned array:

```php
$group = acf_get_field_group('group_67594c4e8e4b8');
$fields = acf_get_fields('group_67594c4e8e4b8') ?: [];
return [
    'group' => $group ? ['key' => $group['key'], 'title' => $group['title']] : null,
    'fields' => array_map(
        static fn(array $field): array => [
            'key' => $field['key'],
            'name' => $field['name'],
            'label' => $field['label'],
            'type' => $field['type'],
        ],
        $fields,
    ),
];
```

For a write, keep the PHP operation narrow and idempotent. Use `update_field()`
for ACF option values and `wp_create_nav_menu()` / `wp_update_nav_menu_item()`
for menu data. Return the IDs and final values so the next call can verify the
change. Do not modify the Novamira plugin, its OAuth user, or its credentials.

## Schema versus data

Decide which side of the boundary a requested change belongs to:

| Request | Correct place |
| --- | --- |
| Add/remove/rename an ACF field | Theme ACF JSON; update code consumers |
| Change field type, default, location, or repeater subfields | Theme ACF JSON; check ACF sync |
| Set a local option value | Novamira MCP `update_field(..., 'option')` |
| Create/update local menu items | Novamira MCP WordPress menu APIs |
| Change how a field renders | Theme Service/Composer/Blade |
| Create a reusable block | Theme `app/Blocks`, ACF JSON, Service/Composer/Blade |
| Create PHP functionality | Theme repository, not Novamira sandbox |

Do not create a duplicate ACF group when an existing group already models the
requested data. Inspect group keys, locations, and field names first. Reuse
existing option pages, menu locations, Services, and Composers where possible.

## Safe write rules

- Keep Novamira enabled only on local/develop; never configure it on production.
- Before broad or destructive data changes, create a fresh database snapshot.
- Prefer a read-only inspection before every write.
- For bulk changes, report the target site, filters, and record count before
  executing.
- For multisite, include the target host in the operation and verify the blog
  ID afterward.
- Do not store OAuth tokens, Application Passwords, or MCP config in Git.
- Do not edit `public/build/`, `resources/acf-lang/`, WordPress core, or
  `wp-content/plugins/novamira/`.
- After an ACF schema change, verify JSON validity, the ACF group/fields, the
  saved option values, and the first frontend render.

## Verification

For repository changes, run the checks that apply:

```bash
ddev exec php -l wp-content/themes/sage/app/Services/{Service}.php
ddev exec php -l wp-content/themes/sage/app/View/Composers/{Composer}.php
ddev exec php -r 'json_decode(file_get_contents("wp-content/themes/sage/resources/acf-json/{group}.json"), true, 512, JSON_THROW_ON_ERROR);'
git diff --check
```

For local WordPress data, verify through the same site MCP:

```php
return [
    'fields' => [
        'copyright' => get_field('footer_copyright', 'option'),
        'year' => get_field('footer_copyright_since', 'option'),
        'accessibility' => get_field('footer_accessibility_text', 'option'),
    ],
    'locations' => get_nav_menu_locations(),
];
```

When checking a menu, resolve the assigned location and inspect its item titles
and URLs with `wp_get_nav_menu_items()`. Do not assume a menu exists just
because its location is registered.
