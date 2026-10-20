# Sage Block Conventions

## Naming

**Block names must be universal** — describe the layout/component pattern, not the content or use-case. A name that only makes sense for one project or one audience is wrong.

| ❌ Too specific | ✅ Universal |
|---|---|
| `AudienceCards` | `ImageTiles` |
| `TeamMembers` | `PersonCards` |
| `NewsHighlights` | `ArticleGrid` |
| `ServicesStrip` | `IconCards` |

| Form | Pattern | Example |
|---|---|---|
| PHP class / folder | `PascalCase` | `ImageTiles` |
| Block slug | `sage/{kebab-case}` | `sage/image-tiles` |
| Blade view | `kebab-case` | `image-tiles.blade.php` |
| ACF group key | `group_{snake}_main` | `group_image_tiles_main` |
| ACF field key (top-level) | `field_{snake_block}_{field}` | `field_image_tiles_heading` |
| ACF field key (sub-field) | `field_{snake_block}_{parent}_{field}` | `field_image_tiles_link_label` |
| Alpine component | `camelCase` | `imageTiles` |
| CSS/JS file | `kebab-case` | `image-tiles.js` |

---

## File locations (relative to `wp-content/themes/sage/`)

| File | Path |
|---|---|
| Block metadata | `app/Blocks/{PascalName}/block.json` |
| Service | `app/Services/{PascalName}Service.php` |
| Composer | `app/View/Composers/{PascalName}.php` |
| Blade template | `resources/views/blocks/{kebab-name}.blade.php` |
| ACF field group | `resources/acf-json/group_{snake_name}.json` |
| JS (optional) | `resources/js/blocks/{kebab-name}.js` |
| CSS (optional) | `resources/css/blocks/{kebab-name}.css` |

---

## Auto-registration — nothing to touch

- `ThemeServiceProvider` auto-discovers `app/Blocks/`
- Vite auto-discovers `resources/js/blocks/*.js` and `resources/css/blocks/*.css`
- `composer.json` PSR-4: `App\Services`, `App\View\Composers` already registered
- ACF sync: commit `resources/acf-json/` JSON; ACF reads automatically
- Block JS self-registers via `alpine:init` — no `app.js` import needed
- Block CSS enqueued via Composer — no `@import` in `app.css`

---

## ACF field types

| Content | ACF type | Notes |
|---|---|---|
| Short text | `text` | — |
| Long text | `textarea` | — |
| Rich text | `wysiwyg` | Returns HTML string |
| Single image | `image` + `return_format: "array"` | `{ url, alt, width, height }` |
| Link / button | `group` | Sub-fields: `label`, `url`, `open_in_new_tab` |
| List of items | `repeater` | Array of arrays |
| Boolean | `true_false` | — |
| Options | `select` | — |
| Color override | `color_picker` | Hex string |

---

## Service patterns

- All `get_field()` calls live in the Service — never in Blade.
- `getData()` returns a flat array keyed to Blade variable names.
- Sanitize: strings → `trim()`, URLs → `esc_url()`, booleans → `filter_var(..., FILTER_VALIDATE_BOOLEAN)`, arrays → `is_array()` check.

| Helper | Use when |
|---|---|
| `getString(string $field)` | `text` / `textarea` |
| `getImage(string $field)` | `image` (returns `{ url, alt }`) |
| `normalizeButton(mixed $btn)` | `group` link (returns `{ label, url, open_in_new_tab }`) |
| `getRepeater(string $field, callable $normalize)` | `repeater` |

---

## Alpine vs plain JS

| Situation | Use |
|---|---|
| `x-show`, `x-text`, `x-bind` bound to JS state in Blade | Alpine component |
| Swiper init, scroll reveals, DOM work without reactive template variables | `DOMContentLoaded` + query selectors |

**`x-cloak` rule**: always guard `@if (!$is_preview) x-cloak @endif` — bare `x-cloak` hides the block in the editor.

**Scroll reveals**: the theme drives the page with Locomotive Scroll. Use `onEnterView` from `@utils/scroll-observer.js` (hooks LS's scroll event, falls back to native scroll) to add an `.is-inview` class to `[data-animate]` elements; define the hidden->visible transition in the block CSS. Do not add GSAP — it is not a project dependency.

**Scroll reveal / Swiper in editor**: `DOMContentLoaded` fires before Gutenberg injects block previews — neither runs in the editor. This is intentional. Guard the reveal explicitly:
```js
if (el.closest('.block-editor-block-list__block, [data-is-preview]')) return;
```

---

## Swiper CSS — shared handle

`@utils/swiper.js` is side-effect free (no CSS imports). CSS lives in `resources/css/common/swiper.css`, loaded via a shared handle WordPress deduplicates:

```php
wp_enqueue_style('sage-swiper', Vite::asset('resources/css/common/swiper.css'));
```

If the block also has a block-specific CSS file with Swiper overrides, declare the dependency:

```php
wp_enqueue_style('{KEBAB_NAME}-style', Vite::asset('resources/css/blocks/{KEBAB_NAME}.css'), ['sage-swiper']);
```

---

## Blade rules

**Wrapper (always)**:
```blade
<?php
$is_preview = $is_preview ?? false;
$atts = ($is_preview) ? '' : ' ' . get_block_wrapper_attributes();
?>
<div<?php echo $atts; ?>>
```

**Preview guard for links**:
```blade
<a href="{{ esc_url($link['url']) }}"
   @if ($link['open_in_new_tab']) target="_blank" rel="noopener noreferrer" @endif
   @if ($is_preview) aria-disabled="true" tabindex="-1" class="pointer-events-none" @endif>
```

**Repeater with empty state**:
```blade
@forelse ($items as $item)
  {{-- item markup --}}
@empty
  @if ($is_preview)<p class="text-gray-400 italic">Add items to display here.</p>@endif
@endforelse
```

**Inline SVG / raster**:
```blade
{!! Vite::content('resources/images/icon.svg') !!}
<img src="{{ $image['url'] }}" alt="{{ $image['alt'] }}" class="w-full object-cover">
```

---

## Tailwind — font sizes

Map Figma px to nearest Tailwind step (≤ 8px diff). Add `@theme` token to `resources/css/common/fonts.css` only for outliers. Never use `text-[Npx]`.

`xs`=12 · `sm`=14 · `base`=16 · `lg`=18 · `xl`=20 · `2xl`=24 · `3xl`=30 · `4xl`=36 · `5xl`=48 · `6xl`=60 · `7xl`=72 · `8xl`=96 · `9xl`=128

---

## Tailwind — color tokens

Defined in `resources/css/common/colors.css`:

| Token | Hex | Usage |
|---|---|---|
| `primary` | `#016dc6` | `text-primary`, `bg-primary`, `border-primary` |
| `dark` | `#0f1215` | `text-dark`, `bg-dark` |

Check `colors.css` first. Add `--color-{name}: #hex` under `@theme {}` if no match. Never use `text-[#hex]` / `bg-[#hex]`.

---

## Responsiveness (mobile-first)

`sm:`=640px · `md:`=768px · `lg:`=1024px · `xl:`=1280px

Never use fixed pixel widths (`w-[448px]`) — prefer `flex-1`, `w-full`, fractional grid columns.

---

## CSS files

**Tailwind first.** Prefer Tailwind utilities for all layout, spacing, color, hover, and transition styles. Use an explicit `<div>` in Blade for overlays instead of a CSS `::after` pseudo-element.

Create a separate `.css` file only for:
- Keyframe animations or transitions not expressible in Tailwind
- Complex pseudo-element content (`::before`/`::after` with generated content)
- Third-party library overrides (e.g. Swiper pagination dot colours)
