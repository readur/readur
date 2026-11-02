# Tokens

Source of truth: `frontend/src/styles/tokens.css`. This table is generated from it — if they disagree, the CSS wins.

## Colour

| Token | Light | Dark | Use |
|---|---|---|---|
| `--bg` | `#F0F1F7` | `#10121C` | page background |
| `--surface` | `#FFFFFF` | `#171A28` | cards, sidebar, top bar |
| `--surface-2` | `#FFFFFF` | `#1D2132` | raised: popovers, dialogs, toasts, menus |
| `--surface-sunken` | `#E4E6F0` | `#0B0D15` | wells, tracks, search bar, table head |
| `--line` | `#D8DBE8` | `#272B3D` | hairlines |
| `--line-strong` | `#737896` | `#646A88` | control borders (3:1) |
| `--fg` | `#161A2E` | `#E7E9F5` | text |
| `--fg-2` | `#474C66` | `#B3B8CF` | secondary text |
| `--fg-meta` | `#585D78` | `#959BB6` | meta text |
| `--accent` | `#4338CA` | `#A5B4FC` | actions, focus, selection |
| `--accent-hover` | `#3730A3` | `#C0CBFD` | primary hover |
| `--accent-soft` | `#E2E3F9` | `#262B52` | tonal buttons, selected rows, active nav |
| `--accent-fg` | `#FFFFFF` | `#12153A` | text on accent |
| `--new` | `#8F5300` | `#FBC15E` | "changed since you looked" (amber) |
| `--new-fill` | `#BF7A0A` | `#FBC15E` | decorative new marks |
| `--new-soft` | `#FCEFD6` | `#3A2C10` | new tint |
| `--new-fg` | `#FFFFFF` | `#1F1400` | text on new |
| `--ok` | `#1C7247` | `#6FD69F` | success |
| `--ok-soft` | `#D8F0E2` | `#15301F` | success tint |
| `--warn` | `#A3440C` | `#FFA066` | warning (orange, kept apart from the amber "new") |
| `--warn-fill` | `#E8782E` | `#FFA066` | decorative warning |
| `--warn-soft` | `#FCE5D5` | `#3A2312` | warning tint |
| `--danger` | `#B4313A` | `#F28B93` | errors, destructive |
| `--danger-soft` | `#F8DDE0` | `#3A1A1E` | danger tint / tonal danger button |
| `--danger-fg` | `#FFFFFF` | `#2A0A0E` | text on danger |
| `--selection` | `#CDD0F7` | `#2E3570` | text selection |
| `--scrollbar` | `#BDC1D6` | `#383D55` | scrollbar thumb |
| `--shadow-1` | `0 1px 2px rgba(22, 26, 46, 0.06), 0 4px 14px rgba(22, 26, 46, 0.06)` | `0 1px 2px rgba(0, 0, 0, 0.3), 0 6px 18px rgba(0, 0, 0, 0.25)` | resting cards |
| `--shadow-overlay` | `0 2px 6px rgba(22, 26, 46, 0.08), 0 16px 40px rgba(22, 26, 46, 0.14)` | `0 2px 8px rgba(0, 0, 0, 0.4), 0 18px 48px rgba(0, 0, 0, 0.45)` | anything floating |
| `--scrim` | `rgba(16, 18, 40, 0.32)` | `rgba(4, 5, 12, 0.6)` | modal backdrop |
| `--accent-soft-hover` | `#D3D5F5` | `#2F3563` | tonal hover |
| `--focus-halo` | `rgba(67, 56, 202, 0.32)` | `rgba(165, 180, 252, 0.35)` | 3px input focus halo |
| `--hover` | `rgba(22, 26, 46, 0.05)` | `rgba(231, 233, 245, 0.06)` | neutral hover overlay |
| `--press` | `rgba(22, 26, 46, 0.09)` | `rgba(231, 233, 245, 0.1)` | neutral press overlay |
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
