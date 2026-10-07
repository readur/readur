# Tokens

Source of truth: `frontend/src/styles/tokens.css`. This table is generated from it — if they disagree, the CSS wins.

## Colour

| Token | Light | Dark | Use |
|---|---|---|---|
| `--bg` | `#EEF3F1` | `#0F1716` | page background |
| `--surface` | `#FFFFFF` | `#16211F` | cards, sidebar, top bar |
| `--surface-2` | `#FFFFFF` | `#1C2826` | raised: popovers, dialogs, toasts, menus |
| `--surface-sunken` | `#E3EBE8` | `#0B1110` | wells, tracks, search bar, table head |
| `--line` | `#D5E0DC` | `#26332F` | hairlines |
| `--line-strong` | `#74867F` | `#5F746E` | control borders (3:1) |
| `--fg` | `#13211E` | `#E6F0ED` | text |
| `--fg-2` | `#45564F` | `#B4C4BF` | secondary text |
| `--fg-meta` | `#56665F` | `#93A6A1` | meta text |
| `--accent` | `#0F6E63` | `#4FD1BD` | actions, focus, selection |
| `--accent-hover` | `#0B5B52` | `#6EDCCB` | primary hover |
| `--accent-soft` | `#D3ECE7` | `#173A35` | tonal buttons, selected rows, active nav |
| `--accent-fg` | `#FFFFFF` | `#062420` | text on accent |
| `--new` | `#B23F18` | `#FF9A76` | "changed since you looked" (coral) |
| `--new-fill` | `#DC5229` | `#FF9A76` | decorative new marks |
| `--new-soft` | `#FCE9E2` | `#3A2119` | new tint |
| `--new-fg` | `#FFFFFF` | `#1A0B05` | text on new |
| `--ok` | `#1C7247` | `#6FD69F` | success |
| `--ok-soft` | `#D8F0E2` | `#15301F` | success tint |
| `--warn` | `#8A5B06` | `#F0C062` | warning |
| `--warn-fill` | `#F0B44C` | `#F0C062` | decorative warning |
| `--warn-soft` | `#F7EACB` | `#30270F` | warning tint |
| `--danger` | `#B4313A` | `#F28B93` | errors, destructive |
| `--danger-soft` | `#F8DDE0` | `#3A1A1E` | danger tint / tonal danger button |
| `--danger-fg` | `#FFFFFF` | `#2A0A0E` | text on danger |
| `--selection` | `#BFE3DC` | `#1F4A43` | text selection |
| `--scrollbar` | `#B9C9C4` | `#374743` | scrollbar thumb |
| `--shadow-1` | `0 1px 2px rgba(19, 33, 30, 0.06), 0 4px 14px rgba(19, 33, 30, 0.06)` | `0 1px 2px rgba(0, 0, 0, 0.3), 0 6px 18px rgba(0, 0, 0, 0.25)` | resting cards |
| `--shadow-overlay` | `0 2px 6px rgba(19, 33, 30, 0.08), 0 16px 40px rgba(19, 33, 30, 0.14)` | `0 2px 8px rgba(0, 0, 0, 0.4), 0 18px 48px rgba(0, 0, 0, 0.45)` | anything floating |
| `--scrim` | `rgba(15, 40, 36, 0.32)` | `rgba(3, 8, 7, 0.6)` | modal backdrop |
| `--accent-soft-hover` | `#C2E4DD` | `#1E4842` | tonal hover |
| `--focus-halo` | `rgba(15, 110, 99, 0.32)` | `rgba(79, 209, 189, 0.35)` | 3px input focus halo |
| `--hover` | `rgba(19, 33, 30, 0.05)` | `rgba(230, 240, 237, 0.06)` | neutral hover overlay |
| `--press` | `rgba(19, 33, 30, 0.09)` | `rgba(230, 240, 237, 0.1)` | neutral press overlay |
| `--bevel` | `inset 0 1px 0 rgba(255, 255, 255, 0.18)` | `inset 0 1px 0 rgba(255, 255, 255, 0.06)` | raised-button inner highlight |
| `--src-1` … `--src-8` (+ `-soft`) | | | source hues; pick through `sourceHue()` / `SourceBadge`, never by number |

## Shape, type, space, layers, motion

| Group | Tokens |
|---|---|
| Radius | `--radius-sm` 8px (small buttons, menu rows) · `--radius` 10px (buttons, inputs) · `--radius-lg` 14px (cards, tables, notices) · `--radius-xl` 20px (dialogs, slide-over, palette) · `--radius-pill` |
| Font | `--font-ui` (Plus Jakarta Sans) · `--font-data` (JetBrains Mono, tabular — sizes, counts, dates, fractions) · `--font-label` |
| Size | `--fs-xs` 11 · `--fs-sm` 12 · `--fs-md` 14 · `--fs-lg` 16 · `--fs-xl` 20 · `--fs-title` 24 · `--fs-2xl` 28 |
| Space | `--s-1` 4 · `--s-2` 8 · `--s-3` 12 · `--s-4` 16 · `--s-5` 24 · `--s-6` 32 · `--s-7` 48 |
| Layers | `--z-local` (inside one component) < `--z-shell` < `--z-dock` < `--z-overlay` < `--z-toast` < `--z-palette` |
| Motion | `--dur-1` 120ms · `--dur-2` 200ms · `--dur-3` 480ms · `--ease` (all 0ms under reduced motion; gate keyframes yourself) |

## Changing the accent

1. Edit `--accent`, `--accent-hover`, `--accent-soft`, `--accent-soft-hover`, `--accent-fg`, `--focus-halo`, `--selection` in **three** blocks of `tokens.css`: light, dark, and the `@media (prefers-color-scheme: dark)` pre-theme block (it must equal dark).
2. Run `node scripts/contrast-check.mjs`. Text pairs need 4.5:1, marks 3:1.
3. If a `--src-n` now sits within 25° of the new accent hue, move that slot's hue (light, dark and media blocks) and re-run.
4. Update `DESIGN.md` frontmatter colours.
