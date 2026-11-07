# Design

How the interface looks, and the rules that keep it consistent. The tokens
live in `frontend/src/index.css`; the shared pieces in
`frontend/src/shared/components/ui.tsx`.

## Palette

The colors live in one file, `design/tokens.css`, which the application and the
website both import; the extension's popup copies the values, because a store
package holds only its own folder. `tools/test/design.test.mjs` fails if a
pairing below drops under its contrast, or if the popup or a page drifts.

Green is the brand. It has a full scale, with the two greens of the logo at
300 and 500:

| Step | Value | Step | Value |
| --- | --- | --- | --- |
| `green-50` | `#E9FFF6` | `green-600` | `#12A06C` |
| `green-100` | `#C8FDE8` | `green-700` | `#0E7D54` |
| `green-200` | `#94FBD3` | `green-800` | `#0B5E40` |
| `green-300` | `#4BF6B6` (logo) | `green-900` | `#08402C` |
| `green-400` | `#3DDC9F` | `green-950` | `#04140E` |
| `green-500` | `#19C283` (logo) | | |

Pages use roles, never a scale step. Contrast is the WCAG ratio on the page
background; normal text needs 4.5.

| Role | Value | Use | Contrast |
| --- | --- | --- | --- |
| `surface-page` | `gray-950` `#08090A` | Page | |
| `surface-panel` | `gray-900` `#0F1011` | Panels, sidebar | |
| `surface-raised` | `gray-850` `#17181A` | Toasts, menus over a panel | |
| `border` | `gray-800` `#222326` | Borders, dividers | |
| `border-strong` | `gray-700` `#34363B` | Controls, scroll bars | |
| `text-primary` | `gray-100` `#F7F8F8` | Main text | 18.7 |
| `text-secondary` | `gray-400` `#A1A5AD` | Labels, lists | 8.1 |
| `text-muted` | `gray-500` `#8A8F98` | Descriptions | 6.1 |
| `text-hint` | `gray-600` `#787C83` | Hints, versions | 4.75 |
| `accent` | `green-500` | The main action, "on", what is showing | 8.6 |
| `accent-hover` | `green-400` | Hover of `accent` | 11.3 |
| `accent-strong` | `green-300` | Text and icons that stand out in green | 14.4 |
| `accent-ink` | `green-950` | Text on `accent` | 8.2 |
| `accent-tint` | green at 15 % | Chips and rows that are on | |
| `accent-line` | `green-800` | A border around something that is on | |
| `focus` | green at 60 % | Focus ring | |
| `warning` | `#F5B83D` | Waiting, paused publishing | 11.2 |
| `danger` | `#F2555A` | Errors, stopped presences | 5.9 |

Rules:

- Text on `accent` is `accent-ink`. White on it reads at 2.3 and fails.
- Green means brand, "on" or "showing". One primary button in view.
- No other hues. A state that is neither on, waiting nor failing is grey.
- `text-hint` is the dimmest text allowed; on a panel it is still 4.5.
- A presence's own color appears only on its logo.

## Type

Inter (bundled, `@fontsource-variable/inter`), 13 px for interface text,
22 px semibold for page titles, 12–11 px for hints. Section titles are
13 px medium grey, in sentence case, never upper case.

## Pieces

| Piece | Where | For |
| --- | --- | --- |
| `Section` | `ui.tsx` | A titled group on a page, with an optional action. |
| `Panel` | `ui.tsx` | A bordered box whose rows are split by thin lines. |
| `SettingRow` | `ui.tsx` | A setting: text on the left, control on the right (or below with `stacked`). |
| `Chip` | `ui.tsx` | Kind, origin and other short labels. |
| `fieldClass`, `selectClass`, `areaClass` | `ui.tsx` | Text boxes, lists and multi-line boxes. |
| `StatusIcon`, `PriorityBars` | `StatusIcon.tsx` | A state's shape and color; priority as one to three bars. |
| `Accordion` | `Accordion.tsx`, `design/accordion.css` | A section that opens and closes. The website uses the same styles; a group with `name` keeps one open. |
| `Button` | `Button.tsx` | `primary` for the one main action of a view; `ghost` for the rest. |

Lists are one `Panel` with a row per item, not a card per item. A page has at
most one primary button in view.

## Tray

The tray icon gets a dot in its lower right corner: `accent` while something
is showing, `warning` while paused, grey while Discord is closed or the user
is away, and none when there is simply nothing to show
(`src-tauri/src/tray.rs`).
