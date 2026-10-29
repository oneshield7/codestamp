# DESIGN.md: Codestamp design guide

Visual guide and UI patterns for Codestamp: the reference for adding or changing screens, for both people and AI agents. The visual direction is called **"Postmark"**. The source of truth for tokens is `src/app.css`. If this document differs from the code, the code is correct and this document gets updated.

## 1. Character

- **A daily work tool**, not a marketing page: calm, dense, easy to scan.
- **"Ink" monochrome**: cool grays and white. The near-black ink background is only for primary action buttons in dialogs and panels (Create collection, Duplicate, Save, Share), **never for menus** (tiles, toolbar, sidebar items). **Color only for meaning**: status (ok / warn / err), HTTP methods ("method stamp"), and collection emoji icons.
- **Notebook shape**: a collection is a numbered list of steps. One request is open as a card, the rest are compact rows.
- **Flat**: 1px borders and slightly different backgrounds are preferred over shadows. Shadows are only for raised or floating elements (active sidebar item, open card, popovers, dialogs).
- **No animations or transitions**; state changes show immediately. The only exception is the switch from the launch screen to the app when Codestamp opens (see _Launch screen_).
- **Three themes, one set of tokens**: Light, Dark, and Dark Night redefine the same tokens in `src/app.css` and set `color-scheme` to `light` or `dark` (see _Themes_).

## 2. Color

### Themes

Four options in Settings › Appearance: **Light** (default), **Dark** (neutral graphite), **Dark Night** (bluish black for night use), and **System** (follows Windows, Light or Dark). The theme is set through `data-theme` on `<html>`; any element may use its own `data-theme`, such as the previews in the theme picker.

- The values in the tables below are the Light theme. Dark and Dark Night override the same tokens in `src/app.css`; components never know which theme is active.
- Both dark themes use the same meaning colors (status, method, variable) in lighter versions, with translucent (`rgba`) backgrounds and borders so they fit on both surfaces.
- `--on-accent` becomes dark ink in the dark themes, because `--accent` and the method colors are light there. Text on method colors (Send, Run tool, delete buttons) always uses `--on-accent`.
- The Windows title bar also turns dark for Dark and Dark Night (`set_theme` in Rust).
- All text/background pairs in the three themes pass WCAG: text ≥ 4.5:1, focus ≥ 3:1.

| Token             | Dark      | Dark Night |
| ----------------- | --------- | ---------- |
| `--bg`            | `#15171c` | `#070a12`  |
| `--surface`       | `#1c1f25` | `#0d121d`  |
| `--surface-muted` | `#21252c` | `#111827`  |
| `--border`        | `#2d313a` | `#1c2436`  |
| `--text`          | `#eceef2` | `#e1e7f5`  |
| `--text-muted`    | `#9ba1ac` | `#8a95ae`  |
| `--accent`        | `#eceef2` | `#e1e7f5`  |
| `--focus`         | `#6f9bff` | `#7f9fff`  |

### Surfaces and borders

| Token              | Value     | Used for                                                                                               |
| ------------------ | --------- | ------------------------------------------------------------------------------------------------------ |
| `--bg`             | `#f3f4f6` | App background behind the sidebar and content                                                          |
| `--surface`        | `#ffffff` | Cards, panels, dialogs, inputs, active sidebar item                                                    |
| `--surface-muted`  | `#f9fafb` | Secondary areas inside a surface: code editor frame, values in diffs, option tiles, selected options   |
| `--surface-sunken` | `#edeef1` | Hover on transparent buttons, segmented control container, disabled buttons, neutral chips, icon boxes |
| `--border-soft`    | `#ecedf0` | Dividers inside cards/panels (below the header, table rows, tab rows)                                  |
| `--border`         | `#e2e4e9` | Edges of cards, panels, request rows, quiet buttons                                                    |
| `--border-strong`  | `#c9cdd4` | Edges of inputs, secondary buttons, the open request card, dashed lines                                |

### Text

| Token          | Value     | Used for                                                                     |
| -------------- | --------- | ---------------------------------------------------------------------------- |
| `--text`       | `#111318` | Titles, primary text, active tab                                             |
| `--text-2`     | `#383d47` | Body text and control labels                                                 |
| `--text-muted` | `#5a606c` | Captions, metadata, table headers (contrast ±6.3:1 on white, passes WCAG AA) |

### Ink (accent) and focus

| Token           | Value     | Used for                                                                                          |
| --------------- | --------- | ------------------------------------------------------------------------------------------------- |
| `--accent`      | `#111318` | Primary action buttons in dialogs/panels (Create collection, Duplicate, Save, Share), step badges |
| `--on-accent`   | `#ffffff` | Text on `--accent`                                                                                |
| `--accent-soft` | `#e6e8ec` | Selected option (history version, autocomplete), editor text selection, drop target, info banner  |
| `--accent-text` | `#111318` | Links (ink, weight 500; underline on hover for "Open … →" links)                                  |
| `--focus`       | `#2f6bff` | **Only** the focus indicator: the `:focus-visible` outline and the border of the focused input    |

### Semantic

Always used as a set of three (text, background, line):

| Token                      | Value                             | Meaning                                                                    |
| -------------------------- | --------------------------------- | -------------------------------------------------------------------------- |
| `--ok` / `-bg` / `-line`   | `#0a7a4b` / `#e8f6ef` / `#b6e2cb` | Success: 2xx status, passing tests, synced, "Created just now"             |
| `--warn` / `-bg` / `-line` | `#a44b07` / `#fdf1e6` / `#f1cba6` | Needs attention: unfilled variables, pending conflicts, offline, read-only |
| `--err` / `-bg` / `-line`  | `#c0261b` / `#fdecea` / `#f5c2bd` | Failure or destructive: 4xx/5xx status, errors, delete actions             |

Semantic colors are **always paired with text** ("200", "Tests 3/4", "Team · offline"). Never use color as the only indicator.

### HTTP methods ("method stamp")

Each method has three tokens: `--method-<tone>` (text), `--method-<tone>-bg`, `--method-<tone>-line`.

| Tone     | Text      | Background | Line      | Method                   |
| -------- | --------- | ---------- | --------- | ------------------------ |
| `get`    | `#0a7a4b` | `#e8f6ef`  | `#b6e2cb` | GET                      |
| `post`   | `#a44b07` | `#fdf1e6`  | `#f1cba6` | POST                     |
| `put`    | `#1d5bd8` | `#eaf0fd`  | `#bfd0f7` | PUT                      |
| `patch`  | `#6d3fd0` | `#f1ecfd`  | `#d4c6f7` | PATCH                    |
| `delete` | `#c0261b` | `#fdecea`  | `#f5c2bd` | DELETE                   |
| `other`  | `#383d47` | `#edeef1`  | `#c9cdd4` | HEAD, OPTIONS            |
| `ws`     | `#0f7487` | `#e6f5f8`  | `#b3dfe7` | `WS` label (WebSocket)   |
| `mcp`    | `#7a5a00` | `#fbf4dc`  | `#ecd99b` | `MCP` label (MCP server) |
| `gql`    | `#a8156f` | `#fbe8f3`  | `#f1bfdd` | `GQL` label (GraphQL)    |
| `grpc`   | `#4f6b0a` | `#f1f6e1`  | `#d3e2a8` | `gRPC` label             |
| `soap`   | `#4338ca` | `#eef0fe`  | `#c9cef8` | `SOAP` label             |

Get the tone through `methodTone()` / `methodColor()` (HTTP methods) or `stampTone()` (the stamp for each request kind) in `src/lib/request/methods.ts`, and the stamp label through `stampLabel()` in `src/lib/request/kinds.ts`; never write method colors directly. Every text/background combination above passes 4.5:1, including for 11px labels.

### Variables `{{…}}`

| Token                     | Value (Light)                     | Used for                                                                                 |
| ------------------------- | --------------------------------- | ---------------------------------------------------------------------------------------- |
| `--var` / `-bg` / `-line` | `#a3166b` / `#fbe9f4` / `#efbfdc` | Known `{{name}}` references (Globals, collection, active environment, dynamic variables) |
| `--err` / `-err-bg`       | (see Semantic)                    | Undefined references, plus a wavy underline and an "… is not defined" tooltip            |

- Used in the URL field, params/headers tables, auth fields, the body/message/MCP argument editors, and the URL line of closed requests.
- Single source: `splitVariables()` / `variablePattern()` in `src/lib/format/variables.ts` (same naming rules as Rust), and the `VariableText` component for colored text.
- An input field cannot color part of its text, so a colored copy is drawn over the input (no background, `pointer-events: none`) and the input text is made transparent; the caret and selection still belong to the input.

### Icon colors (`--hue-*`)

Eight colors for title icons: `blue`, `green`, `teal`, `violet`, `amber`, `orange`, `rose`, `slate`. Each has `--hue-<name>` (icon stroke) and `--hue-<name>-bg` (tile background); the dark themes use lighter versions with translucent backgrounds. All pass 4.5:1 against their tile and the surface in all three themes.

Usage: set `data-hue="blue"` on the icon or its tile. The attribute fills `--hue` and `--hue-bg`, and `color` follows `--hue`, so `stroke: currentColor` picks up the color too. Tiles use `background: var(--hue-bg)`.

| Place                                                                                     | Color  |
| ----------------------------------------------------------------------------------------- | ------ |
| Home, "New collection" action, Requests feature, HTTP request tile/adder                  | blue   |
| Environments, Scripts and tests feature                                                   | green  |
| Import, Docs, Import action, Environments feature, WebSocket tile/adder, Settings › Proxy | teal   |
| History, Catalog, Join action, Team and Appearance features, Settings › Appearance        | violet |
| Cookies, MCP server tile/adder, Settings › Certificates                                   | amber  |
| Orchestration, History feature, section adder, Settings › About & shortcuts               | orange |
| Settings (sidebar and title), Capture feature, Settings › Data & history                  | rose   |
| Copy as code feature, "No account needed" panel                                           | slate  |

One place always has one color: the sidebar icon and its page title icon are the same (`NavIcon`). Icon colors carry no status meaning; status still uses the semantic tokens and text.

### Supporting tokens

- Status dots: `--dot-ok` (active environment, passed Run segment), `--dot-err` (failed segment), `--dot-off` (Globals). In the environment picker only the active environment is green; other environments and "No environment" are gray.
- Shadows: `--shadow-card`, `--shadow-raise`, `--shadow-segment` (selected segment), `--shadow-pop` (popovers, tooltips, pickers), `--shadow-dialog`; dialog backdrop `--backdrop`.
- Syntax highlight: `--code-property`, `--code-keyword`, `--code-function`, `--code-string`, `--code-number`, `--code-atom`, `--code-comment`, `--code-punctuation`, `--code-active-line`.
- The only exception: Capture images (`src/lib/capture/render.ts`) have their own light and dark palettes, because the images are used outside the app.

New colors are added first as tokens in `src/app.css` for all three themes, not as hex values in components. Run a contrast check after changing a value.

## 3. Typography

**Fonts**

- UI: `--font-ui`, default **Geist Variable** (`@fontsource-variable/geist`, bundled), fallback Segoe UI.
- Code: `--font-code`, default **Geist Mono Variable** (`@fontsource-variable/geist-mono`, bundled), fallback Cascadia Code and Consolas.
- Emoji: `--font-emoji` (Segoe UI Emoji, then Apple/Noto Color Emoji); applied through the `.emoji` utility.
- Fonts are bundled with the app; no fonts are loaded from the internet (strict CSP).
- Users can change the fonts (Settings › Appearance), including importing their own, so do not depend on the metrics of any one font.

**User-adjustable sizes**

- The UI size (11-20, default 14) is applied as **window zoom** (`uiFontSize / 14`). Components write sizes in px on a 14px base, and everything scales along with it.
- Code is set separately: `--font-size-code` (default 13, range 10-24), `--code-line-height` (1.6, range 1.2-2.2), `--code-ligatures`.

**Scale**

| Size / weight                       | Used for                                                                             |
| ----------------------------------- | ------------------------------------------------------------------------------------ |
| 34px / 600, letter-spacing −0.025em | Home title                                                                           |
| 26px / 600, letter-spacing −0.01em  | View titles and collection name (h1)                                                 |
| 20px / 600                          | "Add your first request", request name in focus mode                                 |
| 18px / 600                          | New collection dialog title                                                          |
| 17px / 600                          | Request name on the card, ConfirmDialog title, "What you can do"                     |
| 16px / 600                          | Dialog and page panel titles (Import)                                                |
| 15px / 600                          | Collection panel titles (Variables, Auth, Team, History, Export), brand, tile titles |
| 14px                                | Base text and inputs                                                                 |
| 13px                                | The most common working size: buttons, tabs, tables, panel content, banners          |
| 12px                                | Metadata, chips, hints, sidebar group titles, URLs and numbers in rows               |
| 11px                                | Method stamps in request rows, `kbd` in the shortcut list                            |

**Weight and style**

- 400 for body text, 500 for labels and active states, 600 for titles, primary buttons, chips, and method stamps. 700 is practically unused.
- Monospace (`.mono` or `--font-code`) for URLs, repository addresses, paths, values in diffs, code, **step numbers** (`01`, `02`), counts in the sidebar, durations, and status codes.
- Text that does not fit: one line with an ellipsis (names, URLs); `overflow-wrap: anywhere` for paths or addresses that must be readable in full.

## 4. Spacing, sizing, and layout

**Spacing scale (px):** 2 · 4 · 6 · 8 · 10 · 12 · 14 · 16 · 18, plus 20 / 24 / 28 / 40 at page level. The most used gaps: 10, 12, and 8.

**Shell**

- Main content: padding `28px 24px 48px` (`20px 16px 40px` below 900px), 14px gap between sections. Max width 1080px, 1400px from a 1800px window, and 1720px from 2400px (focus mode: 1180 / 1560 / 1880px), so large monitors are used for request and response side by side.
- Sidebar: fixed width 264px, padding `20px 14px`, always beside the content (never wraps below it). The sidebar sticks (`position: sticky`, full window height) and has its own scroll.
- Card/panel header: padding `14px 16px`; panel content 16px; dialog header and footer `14px 18px` (New collection dialog `16px 22px`).

**Control heights**

| Height  | Used for                                                                    |
| ------- | --------------------------------------------------------------------------- |
| 46px    | Adder buttons (dashed border), URL field and Send in focus mode             |
| 44px    | Collection icon tile in the header                                          |
| 42px    | URL field and Send button                                                   |
| 38-40px | Form inputs, icon buttons in the New collection dialog, emoji picker button |
| 36px    | Buttons in dialogs and page panels                                          |
| 34px    | Icon buttons in the collection toolbar (34×34), request name input          |
| 32px    | Buttons in collection panels, icon buttons                                  |
| 28-30px | Table cells, small selects, sidebar "+" button, small picker buttons        |
| 22px    | Collection emoji box in the sidebar                                         |

**Responsive**

- Windows are supported from the app's minimum size (720×520) up to 4K. Inside the content, prefer `flex-wrap` with `flex-basis` (e.g. `flex: 1 1 280px`) so elements wrap when the window is narrow.
- Media queries are only for the shell: below **900px** the sidebar becomes a **64px icon rail** (item names stay for screen readers and show as tooltips), and the content width grows at 1800px and 2400px.
- The request card uses a **container query** (`container-type: inline-size`): when the card is narrower than 640px, the request name gets its own row and the header buttons move to a second row.
- Dialogs are flex columns with `max-height: calc(100vh - 32px)`: the header and footer (action buttons) are always visible, and only the content in the middle scrolls or shrinks. Never put action buttons inside the scrolling area.
- Flex children that hold long text (`nowrap` + ellipsis) must get `min-width: 0`, including panels inside dialogs that are `display: flex`; without it the text widens its container.
- Dialogs: `width: min(<N>px, calc(100vw - 32px))` and `max-height: calc(100vh - 48px)` with scrollable content. Widths in use: 420 (confirmation), 580 (New collection), 760 (Copy as code), 860 (conflict), 920 (history), 1180 (Capture). Popovers: `min(372px, calc(100vw - 32px))`.
- Wide tables are wrapped in `overflow-x: auto` and given a `min-width`.

## 5. Radius and elevation

| Radius               | Used for                                                                  |
| -------------------- | ------------------------------------------------------------------------- |
| `--radius-sm` (6px)  | Icon buttons, segments, method stamps, sidebar emoji box, `kbd`           |
| `--radius-md` (8px)  | Buttons, inputs, URL field, banners, status box, sidebar items            |
| `--radius-lg` (12px) | Cards, request rows, panels, dialogs, tiles, adders, icon tiles, popovers |
| 999px                | Status chips, focus mode bar                                              |
| 50%                  | Status dots                                                               |
| 18px                 | Card in focus mode                                                        |

| Shadow                                               | Used for                       |
| ---------------------------------------------------- | ------------------------------ |
| `--shadow-raise` (`0 1px 2px` α 0.06)                | Active sidebar item ("raised") |
| `--shadow-card` (`0 1px 2px` + `0 12px 32px` α 0.08) | Open request card              |
| `0 24px 64px` α 0.28                                 | Dialogs                        |
| `0 16px 40px` α 0.18                                 | Emoji picker                   |
| `0 12px 32px` α 0.14                                 | Autocomplete popover           |
| `0 10px 40px` α 0.1                                  | Focus mode card                |
| `0 1px 2px` α 0.1                                    | Selected segment               |

## 6. Icons

- **Colored title icons**: sidebar icons, page titles, Settings navigation, Home tiles, empty collection tiles, and "Add …" buttons use the `--hue-*` colors (section 2). Small action icons (toolbar, card icons) stay neutral so colors do not compete for attention.
- **Interface icons**: inline SVG, no icon library. `viewBox="0 0 24 24"`, usually 16px (14px in small buttons, 18-20px in tiles, 28px in the dropzone). `fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round`. Always `aria-hidden="true"`; icon-only buttons must have an `aria-label` (+ `title` when it needs an explanation).
- **Logo**: `branding/codestamp-logo.svg` (logo 3a), shown at 28px as `<img alt="">` next to the name "Codestamp". A rounded `#111318` ink square holding white curly braces, a `#4a505c` divider line, a red `#d1432b` postage stamp, and three white address lines. Its colors stay the same in every theme because the same logo is the app icon (`src-tauri/icons`, regenerated with `npx tauri icon branding/codestamp-logo.svg`).
- **README banner**: `branding/banner-light.svg` and `branding/banner-dark.svg` (1280×360), shown through `<picture>` so GitHub picks the one for its theme. Logo, name, tagline, and a collection section with four request rows drawn with the stamp and status chip tokens of the matching theme. Geist and Geist Mono are embedded as data URIs; their license is in `branding/OFL-Geist.txt`. The version badge under it is static, so update it with every release.
- **README reason images**: `branding/why/<name>-light.svg` and `-dark.svg` (760×400), one per card in the two-column "Why Codestamp" table, shown at 380px. Text in them is at least 21px so it stays readable at half size. They draw real app parts (request rows, stamps, the `codestamp-cli` output format) with the tokens of the matching theme, and embed a Geist subset as WOFF.
- **Collection icons**: emoji, see section 7. Emoji are not used as interface icons anywhere else.

## 7. Components

### Buttons

| Variant                     | Style                                                                                                             | Used for                                                                                    |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Primary (`.primary`)        | Background `--accent` (ink), text `--on-accent`, 600, no border                                                   | One main action in a dialog or panel: Save, Share, Create, Duplicate                        |
| Send (`.send`)              | Background in the selected method's color (`--method-<tone>`, WebSocket `--method-ws`), white text, 600           | Sending a request from the card; every combination passes 4.5:1 contrast                    |
| Secondary (`.secondary`)    | Surface background, border `--border-strong`                                                                      | Companion actions in dialogs/pages: Cancel, Later, Choose another file                      |
| Quiet (`.quiet`)            | Surface background, border `--border`, text `--text-2`, 13px                                                      | Text-labeled actions in panels and pages                                                    |
| Toolbar icon (`IconButton`) | 34×34, border `--border`, 16px icon, tooltip; optional status dot in the corner; `round` variant is a 40px circle | Collection header toolbar, section header (Add …, Duplicate, Remove), extra buttons on Home |
| Danger                      | Confirmation: background `--err`, white text. Trigger: `.quiet.danger`, red on hover                              | Delete, Stop syncing                                                                        |
| Link (`.link`, `.go`)       | Transparent, ink 500; `.go` ("Open … →") is underlined on hover                                                   | Light inline actions: Dismiss, Resolve, Copy, Open … →                                      |
| Icon (`.icon`)              | 32×32, transparent, hover `--surface-sunken`                                                                      | Card actions: Copy as code, History, Duplicate, Capture, Focus, Delete, Collapse            |
| Tool (`.tool`)              | Icon + text, border `--border-strong`                                                                             | Actions in focus mode (Capture)                                                             |
| Adder (`.adder`)            | Height 46px, 1.5px dashed border `--border-strong`, ink on hover                                                  | Add request, Add WebSocket, Add section                                                     |
| Tile (`.tile`)              | Large card on a surface background: icon, 15px title, 13px description; no ink background                         | Quick actions in an empty collection                                                        |
| Home main pill (`.primary`) | 40px pill, background `--text`, text `--surface`, 600; hover `--text-2`                                           | The one main action of the selected Home tile                                               |

- Disabled: background `--surface-sunken` with `--text-muted` text/icon (in older dialogs: opacity 0.6, Send: 0.7). Explain the reason in the tooltip or `title` (e.g. "Run all · add an HTTP request first").
- **"Already clicked" marker**: every button that opens a panel or dialog uses `aria-expanded` (plus `aria-haspopup="dialog"` for dialogs) and appears **pressed** while what it opened is still open: `--text` border/line and `--accent-soft` background. This applies to Variables/Auth/Team/Duplicate/Delete in the collection toolbar, the sidebar "+" button, the Home main button that opens New collection, the Duplicate section button, and the Copy as code/History/Duplicate/Capture/Delete icons on the card. Do not use an ink background for this marker.
- Every button gives feedback when pressed (`:active`), including one-shot actions such as Duplicate and Export.
- While busy, the label changes to a verb + "…": "Saving…", "Syncing…", "Creating…", "Joining…".
- At most one primary per area. Destructive actions are never primary and always go through ConfirmDialog.

### Inputs

- **Form field**: height 38-40px, padding `0 12px`, border `--border-strong`, radius md. Label above the input (13px, `--text-2`, gap 6px), 12px muted hint below. On focus, border `--focus`. Invalid value: `aria-invalid="true"`, border and hint in `--err`.
- **URL field**: a 42px group containing the method dropdown (in its method stamp's color and background, with a border on the right) and the input. `:focus-within` changes the group border to `--focus`.
- **Dropdown** (`Dropdown`): replaces every native `<select>`, whose list cannot be styled and does not follow the theme. The button shows the current choice, a chevron, and, when present, a status dot or color (method, MCP transport). When open or focused by keyboard, its edge is blue: `--focus` border plus a 1px ring (in the URL field, a 2px inset ring). The list is a popover (surface, border, radius md, `--shadow-pop`, max 320px): active item `--accent-soft`, selected item bold with a blue check mark. It opens upward when it does not fit below, and closes on an outside click, scroll, or resize. When showing the active option, only the list scrolls (not `scrollIntoView`), so the dialog that contains it does not shift and close the list. Keyboard: arrows/Enter/Space open it, arrows/Home/End move, Enter selects, typing a letter jumps, Escape closes without selecting (and does not also close the dialog or focus mode). Variants: `field` (40px, full width), `compact` (34px), `cell` (30px, edge appears on hover), `chip` (30px, 12px), `method` (in the URL field).
- **Edit in place** (`EditableText`, request name, table cells): transparent border, `--border` appears on hover, then `--focus` with a white background on focus.
- **Variable input** (`VariableInput`): autocomplete after `{{` in a popover (surface, radius md, popover shadow, active item `--accent-soft`). A 12px "Resolves to …" line below the URL, in the warn color when a variable is empty.
- **Secret** (`SecretInput`): the value shows as `••••••••` until the Show button is pressed.
- **Code textarea**: `--surface-muted` background with the code font.

### Segmented control, choices, and tabs

- Segmented: `--surface-sunken` container, 3px padding, radius md; the active segment has a white background, shadow `0 1px 2px`, weight 500. Use `aria-pressed`.
- Body types: None, JSON, XML, HTML, Text, URL-encoded, Form data, Binary. JSON, XML, and HTML have highlighting and Beautify. Broken XML shows a `--warn` hint "Line N, column M: …" below the editor.
- **Choice cards** (radio, e.g. "How do you want to start?"): `<fieldset>` + `<legend>`; each option is a `<label>` containing a native radio, a 32px icon box, a 500 title, and a 13px description. Selected: border `--text`, background `--surface-muted`.
- Long option lists (more than about 5) use `Dropdown`, not radio cards, so the panel does not grow too tall. Example: **Auth type** in the Auth tab, with a 12px muted note about the option (e.g. the header that gets added) beside it.
- Tabs: 13px muted text, padding `10px 2px`, 16px spacing; the active tab has `--text` text, weight 500, and a 2px `--text` underline (not a color). The tab row has a `--border-soft` bottom border and can scroll horizontally. Counts go in the label: "Cookies · 1".

### Chips, method stamps, and status dots

- **Chip**: pill, padding `2px 8px`, 12px 600, `--surface-sunken` background. `.ok` and `.err` use the semantic pairs. Examples: the status code in a request row (mono), "Tests 3/4", "Created just now".
- **Method stamp** (`.stamp`): `padding: 2px 7px`, 1px border, radius sm, 11px 600 code font, `letter-spacing: 0.04em`, colors from its tone's three tokens. Used in request rows; the method select on the card uses the same color + background.
- **Dot**: 8px circle. The active environment is green, Globals is gray, and sync status follows the tone (ok / warn / err).

### Icon toolbar and tooltips (`IconButton`)

- The collection header uses one row of icon buttons in a `role="group"` ("Collection actions"), with a vertical line between groups: **Run all** | **Variables, Auth, Collaboration, Mock server, History** | **Duplicate, Export, Delete**. Variables through History, and Export, open their panel as a dialog above the page (see Collection panel).
- Each button: 34×34, radius md, border `--border`, 16px icon; hover border `--border-strong`; pressed `--surface-sunken`; open (`aria-expanded`) border `--text` + `--accent-soft` background; Delete turns red on hover.
- **Tooltip** = the button label (also its `aria-label`): a small white box below the button (border `--border`, radius sm, shadow `0 4px 12px` α 0.12, 12px 500), shown on hover and on keyboard focus (not after a mouse click). The rightmost button uses `align="end"` and the leftmost button uses `align="start"` so the tooltip is not cut off by the window edge.
- `placement="above"` opens the tooltip above the button, for buttons at the bottom edge of the window (status bar). `current` sets `aria-current="page"` and a pressed look (`--accent-soft` background) on the button that opens the page currently shown.
- **Shortcuts never appear on screen**: not in tooltips, button labels, placeholders, hints, menus, or `title`. Shortcuts still work; the only list of them is in Settings › About & shortcuts (`ShortcutList`). For screen readers, shortcuts go in `aria-keyshortcuts`, not in `aria-label`.
- Short status moves into the label and a **7px dot** in the button corner: Auth set (`--text-2` dot), Team sync status (ok/busy/warn/err). The full label is in the tooltip ("Team · synced", "Auth · on").
- The **section header** uses the same pattern (`role="group"`): one icon button per request type with a `data-hue` colored icon (Add request, Add WebSocket, Add GraphQL, Add gRPC, Add MCP server, Add SOAP), a divider line, then Duplicate section and Remove section (red on hover). The list of types comes from `ADDABLE_KINDS` (`lib/request/kind-icons.ts`), the same source as the adder, the empty collection tiles, and Home.

### Collection icon (emoji)

- Every collection has an emoji. It is **picked automatically from the name** until the user chooses one: keywords in the name are matched against rules (payment/billing → 💳, shop/cart/order → 🛒, user/account → 👤, auth/login/token → 🔐, tool/admin/internal → 🛠️, test/qa → 🧪, and about 20 other rules). When nothing matches, a hash of the name picks from a fixed set (📮 🚀 🧭 🧩 🌱 🔭 🪐 🦊 🐙 🌊 🎯 🧰 📦 🛰️ 🍀 🔮), so the same name always gets the same emoji.
- The user's choice is stored in the database (`collections.icon`; `NULL` = automatic) and **wins** over the automatic one. Renaming only changes an automatic emoji.
- In a team collection, that choice also syncs through `collection.json` (key `icon`, not written while automatic; repository format 2), and its conflicts show as "Collection icon".
- The catalog (58 emoji), the rules, and the fallback set exist only in **Rust** (`src-tauri/src/storage/icons.rs`): the catalog is also the whitelist; any other emoji is rejected. The UI gets the catalog through `collection_icons` and suggestions through `suggest_collection_icon`.
- Display: a 22px box (`--surface-sunken`, radius sm, 14px emoji) in the sidebar; a 44px tile (border `--border`, radius lg, 24px emoji) next to the collection title; a 40px button next to the name field in the New collection dialog.

### Emoji picker (`EmojiPicker`)

- A popover below its trigger (`role="dialog"`, `aria-label="Collection icon"`), 372px wide, radius lg, picker shadow.
- Contents: a search field (focused automatically; matches emoji keywords), a **Suggested for “name”** row (6 emoji from the matching rules), an **All** grid with 8 columns (38px buttons, `aria-label` = emoji name, selected: border `--text` + sunken background), then the footer: status ("Automatic: picked from the name" / "Chosen by you"), **Shuffle**, and **Use automatic** (disabled when already automatic).
- Picking an emoji in the grid saves and closes right away; Shuffle saves without closing. Escape and an outside click close only the picker (not the dialog or focus mode behind it).

### Launch screen (`index.html`)

- The window opens in the center of the screen (`center()` in Rust).
- Written directly in `index.html`, so it shows before the app script runs. Its styles are in `src/launch.css`, **not** an inline `<style>`: inline styles get a hash in the CSP at build time, and the browser then rejects every style injected later (CodeMirror, `style` attributes). Its background is the `--bg` of the saved theme: Rust opens the window hidden, gives the window and the webview that `--bg` color (`paint_background`), sets `data-theme` through an _initialization script_ (`theme_script`), then shows the window. There is no white flash.
- An 84px logo in the center, surrounded by a 104px rounded square (3px line, radius 25px): a `--border` track and one `--text` segment (22% of the perimeter) that spins at 1.4 seconds per turn while the app loads.
- The app signals that it is ready after the collection list has loaded (`finishLaunch` in `lib/dom/launch.ts`). The launch screen shows for at least 0.5 seconds from when the window opens, so a fast start does not flicker.
- Transition: the launch screen fades out over 320ms and its logo grows to 108% (260ms), while the app enters from 96.5% scale with a 6px blur and opacity 0 (460ms, `cubic-bezier(0.16, 1, 0.3, 1)`). Only the start of the animation is held, so afterwards `#app` keeps no transform or filter that could shift `position: fixed` popovers. The launch screen is removed from the DOM afterwards.
- `prefers-reduced-motion: reduce`: the segment does not spin (the track pulses in opacity), and the transition is only a 150ms fade.
- Reloading the window (Ctrl+R) uses the theme picked in that session (`sessionStorage` `codestamp.theme`), not the theme from when the app opened.
- Its review template is in `branding/templates/launch-and-access.html`.

### Title bar (`TitleBar`)

- Replaces the Windows title bar (`decorations: false`). Height `--titlebar-height` (36px), background `--bg`, bottom border `--border`. The sidebar sits directly below it.
- The window itself never scrolls (`html, body { overflow: hidden }`), because a window scrollbar would run up beside the Minimize, Maximize, and Close buttons.
  - `#app` is a flex column as tall as the window: the title bar (`flex: none`), then `.scroller` (`flex: 1; overflow-y: auto; position: relative`), which scrolls all the content. Its scrollbar starts right below the title bar. `position: relative` is required: without it, `position: absolute` elements inside (`visually-hidden` labels) stretch the window, and `scrollIntoView` also pushes the title bar up.
  - `.shell` inside it fills that area (`min-height: 100%`). Sticky elements inside the content use `top` relative to `.scroller` (sidebar `top: 0`, Docs index `top: 16px`), and scroll targets use `scroll-margin-top: 16px`.
- Left: two `IconButton`s with the `quiet` variant (32×28, no border, hover `--accent-soft`): **Menu** (`aria-haspopup="menu"`, pressed while the menu is open) and **Collapse/Expand sidebar** (`align="start"`, so its tooltip starts at the button edge and is not cut off by the left edge of the window). Both use `keepFocus`: a mouse click does not move keyboard focus, so Edit commands from the menu apply to the field being edited.
- Center: an empty `data-tauri-drag-region` area for moving the window; double-click to maximize/restore.
- Right: Minimize, Maximize/Restore, and Close, drawn like Windows 11 caption buttons (46px, 1px line icons at 10×10, hover `--accent-soft`). Close on hover uses the Windows red `--window-close-bg` with white text (5.66:1).
- The menu uses the native Windows menu (`@tauri-apps/api/menu`), so Windows handles the keyboard, accessibility, and Edit commands. The menu module loads the first time the menu opens. Only items that actually work are shown. Menu items do not list accelerators; their shortcuts still work.
- Keyboard focus: a `--focus` ring like other buttons. On window buttons that touch the edge, the ring offset is inward (`-2px`).

### Sidebar

- Brand (logo + "Codestamp"), the **Home** item, the **Collections** group with a "+" button (`aria-label="New collection"`), the collection list (emoji, name, mono count), then the **Tools** group (Orchestration, Catalog, Docs, Git changes), pushed to the bottom of the sidebar (`margin-top: auto`; when the collection list is long, Tools sits right below it). Each group is a `role="group"` with its title as the `aria-label`. Settings, Recently deleted, Cookies, and Environments are in the status bar; Import opens from "+" (New collection), Home, Menu › File, and Quick search; History is in each collection.
- The **Tools** title can be clicked to fold its group (`FoldButton` with text, `aria-expanded`, chevron pointing right when folded). The folded state is remembered per device (`localStorage` `codestamp.foldedNavGroups`). The icon rail always shows every item.
- The 64px icon rail (class `rail`): used below 900px, and when the user collapses the sidebar. It shows the logo, the search button, an icon for each item, and divider lines between groups; group titles and counts are hidden, and item names are only for screen readers and the tooltip.
- **Rail tooltip**: the item name appears to the right of the icon on hover or keyboard focus (`:focus-visible`), with the `IconButton` tooltip style (surface, border, radius sm, `--shadow-pop`, 12px 500), its center aligned with the center of the item. This tooltip is `position: fixed` outside `nav`, because the scrollable sidebar would clip it; it disappears when the pointer leaves, focus moves, or the sidebar scrolls. Long collection names wrap (max 280px) instead of being truncated. The "+" button uses the same tooltip in the full sidebar. The tooltip only shows in the layout where it appeared, so expanding or collapsing the sidebar while focus is still on the item removes it. Rail items do not use `title`, so there are never two tooltips.
- The active item is "raised": background `--surface`, border `--border`, `--shadow-raise`, weight 500. Hover: background `--surface-sunken`.
- No collections: muted text "No collections yet."

### Status bar (`StatusBar`)

- A strip across the full width of the window at the very bottom, below the sidebar and the content (like the Bruno status bar). Height `--statusbar-height` (30px), background `--bg`, top border `--border`. The heights of the sidebar, the Docs index, Git changes, and Home subtract this height.
- Left: four `quiet` `IconButton`s with single-color 15px icons (`currentColor`), in order **Settings** (gear), **Recently deleted**, **Cookies**, **Environments**. Tooltips sit above the button (`placement="above"`; the first one uses `align="start"`). The button for the open page uses `current` (pressed, `aria-current="page"`).
- Right: the running Codestamp version ("v0.3.3", 12px `--text-muted`, tabular figures) from `getVersion()`.
- `<footer aria-label="Status bar">` with a `role="group"` group "Settings and data". Hidden during focus mode, like the sidebar.
- Can be collapsed to the icon rail (button in the title bar, View › Collapse sidebar, or Ctrl+B) rather than hidden, so navigation is always visible. The choice is remembered in this device's `localStorage`; without storage, the toggle still works while the app is open. Below 900px the button is disabled with the tooltip "Widen the window to expand the sidebar".

### Home

- Shown when picked from the sidebar, and automatically when there is no collection yet. It is a **hub** that fills the window height, with content from `lib/home/hub.ts`.
- **Top row**: pill tabs (`role="tab"`) Start, Requests, Testing, Workspace in a bordered surface container; the active tab is `--accent-soft`. On the right, the target collection ("Collection **name**" or "No collection yet").
- **Tiles**: 72px buttons, radius lg, background `--hue-bg`, 30px `--hue` icon, name in the tooltip. Behind them are native radios (`aria-label` = tile name), so the keyboard arrows work. Selected: `--hue` border plus a 2px `--hue` ring at a 3px offset, slightly enlarged.
- **Panel (stage)**: surface, border, radius lg, min 340px, content aligned to the bottom left. The tile icon is drawn large (220px, line width 1, opacity 0.16) at the top right, and hidden when the panel is below 900px (container query) so it does not sit behind the text. Contents: a short 13px fact in `--hue`, a 34px/600 title, a 15px `--text-2` description, example pills (when there is more than one command), a command box (muted `$`, mono code, Copy button), then the main pill and extra round icon buttons. Copy status is 12px (`role="status"`).
- **Tile tooltip**: the tile name appears only on hover over a tile that is **not** selected. The selected tile already shows as the panel title, and because that tile is slightly enlarged, its tooltip would be covered by the panel.
- **Motion** (all off with `prefers-reduced-motion`):
  - one `--accent-soft` pill slides to the selected tab (280ms, following the tab position through `ResizeObserver`);
  - the tiles of a new section rise in by 10px one at a time (45ms delay);
  - the panel text rises 8px line by line (fact, title, description; 50ms delay), then the example pills and the command box;
  - the large icon appears from scale 0.82 and rotation −8° over 480ms.
  - The animations use the separate `translate` / `scale` / `rotate` properties so they do not clash with the selected tile's `transform`.
- **Keyboard**: ←/→ selects a tile (on the tabs: changes section), Enter runs the main button, 1-4 selects a section. Only active when focus is on the page or inside Home, and not while a dialog is open. There is no key hint line below the panel.
- The main button follows the state: "Add GraphQL to Shop API", "Run Shop API", or "Create a collection first" when there is no collection yet.
- Only features that actually exist may be shown. Codestamp has no login or accounts; teamwork comes in through Git.

### Duplicate dialog

- Every duplicate (collection, section, request) goes through a dialog first instead of copying immediately. Width 460px; title "Duplicate collection/section/request", subtitle "Make a copy of “name”".
- The **Name of the copy** field is prefilled with "name copy" (shortened to fit 120 characters) and selected right away. Live validation: required, at most 120 characters, and must not match (case-insensitively) the name of a sibling: another collection, another section in the same collection, or another request in the same collection. The error message shows below the field, and the Duplicate button stays disabled until it is fixed. Rust checks the name again (`clean_name`).
- An info box explains what gets copied. Enter = Duplicate, Escape = cancel.

### New collection dialog

- Opened from the "+" in the sidebar and from the button on Home (with the initial choice matching the tile: New collection, Import, Join).
- "How do you want to start?" choice cards: **Empty collection** (Name required, with an emoji icon button next to it, and an optional Base URL saved as the collection variable `{{baseUrl}}`), **Import from Postman** ("Choose a file…" button, then a preview on the Import page), **Join a team collection** (repository address + Git name/email).
- The Base URL is validated in the UI (must be `http://` or `https://`) and checked again in Rust; the name and icon are also checked in Rust.

### Empty collection (`CollectionStart`)

- Replaces the adder when the collection has no requests yet. The header meta reads "No requests yet · saved automatically", plus a "Created just now" chip for a newly created collection.
- An **Add your first request** panel with one tile per request type (HTTP, WebSocket, GraphQL, gRPC, MCP server, SOAP) and Paste a cURL command (→ the Import page with this collection already selected), then an **Also ready in this collection** card: the `{{baseUrl}}` variable (when present), Sections, and Work with your team.

### Request card (open)

- 1px `--border-strong` border, radius lg, `--shadow-card`.
- Header: step badge (26px, round, ink background), editable 17px name, 12px muted save status ("Saved"; red when saving fails), then icon buttons.
- Target row: the URL field and a Send button in the selected method's color (it changes when the method changes). The request and response panels can stack (separated by a dashed line) or sit side by side (`flex-wrap`, 420px basis).
- A card opened from outside the list (Quick search, History panel, Catalog, Docs, Orchestration, Run all, Import) is scrolled to the top of the content area with `revealElement` (`scroll-margin-top: 16px`) and receives focus (`tabindex="-1"`). A card opened from its own row does not scroll the page.

### Request row (closed)

- Full-width button, grid `24px 64px 1fr auto`, padding `12px 16px`, border `--border`, radius lg.
- Contents: muted mono step number (`01`), method stamp, name (500) and URL (mono 12px muted), then muted mono duration + status code chip (or "Not run yet").
- The drop target is marked with `box-shadow: 0 -3px 0 var(--accent)`. Rows can be dragged, or moved with Alt+↑/↓.

### Section header

Fold button (chevron, `aria-expanded`), dotted drag handle, `EditableText` name at 14px 700 `--text` (stronger than the request rows below it), 12px muted request count, then action links on the right.

- **Folding a section** hides its requests to keep the list compact; the count stays visible and the step numbers of the other requests do not change. The chevron points right when folded.
- The fold state is stored per device (`localStorage`, section ids only) and is not part of team sync. The fold chevrons in collections, Docs, and the sidebar are one component (`FoldButton`), and all fold states are stored through `RememberedIds`.
- A section opens on its own when one of its requests is opened (from History, a duplicate, Run) or when a request is moved into it with Alt+↑/↓. While a section is folded, requests can be dragged onto its section header.

### Collection panel (Variables, Auth, Collaboration, Mock server, History, Export)

- Opened as a modal dialog above the page (`PanelDialog`), like Quick search: `<dialog>` with `showModal()`, width `min(880px, 100vw - 32px)`, top margin 10vh, max height `90vh - 16px`, radius 14px, `--shadow-dialog`, `--backdrop` background with no blur. It rises 10px from scale 0.98 while fading in (240 ms, the same curve as Quick search and the Home tiles; turned off by `prefers-reduced-motion`). The page behind it does not shift.
- The button in the collection header still uses `aria-expanded` and appears pressed while its dialog is open. Escape, a click on the backdrop, or Close closes it; editable panels ask first when there are unsaved changes.
- The dialog content scrolls inside it, and the panel head (title, actions, Close) stays at the top (`position: sticky`). Confirmation dialogs and other dialogs opened from the panel show above it.
- The `14px 16px` header holds a 15px title, a 13px muted description, and action buttons, separated by a `--border-soft` line. The panel's own frame (border, radius) is removed inside the dialog because the dialog itself acts as the card.
- Ctrl+S in Variables and Auth saves while keyboard focus is inside that panel's dialog, not in another dialog open above it.
- Editable panels use `CollectionPanel`: a Save/Saved button, a discard-changes confirmation, and a _leave guard_ that prevents switching collections while there are still edits.
- Collaboration: the account is shown as `dl` rows (_Signs in as_ for https only, _Your changes as_), with a secondary **Change account** button below them. The account form replaces those two rows inside a box bordered with `--border-soft` (radius md, padding 14px, 13px h3 title "Account for “name”"), then Cancel and Save on the right.
- Settings › Git accounts: one `.panel` with a title, a hint, and an Add account button on the right. The add form sits in a box bordered with `--border-soft`. Each account is a grid row (mono name + host, chip, buttons) separated by `--border-soft` lines; below 560px (container query) its content stacks. Chips use the semantic pairs and always carry text: collection count (accent-soft), _Signed in_ (ok), _Sign-in needed_ (warn), _Check failed_ (err), _Offline_ (sunken).
- Git changes: a top bar (collection Dropdown, warn chips for local changes and unpushed commits, muted sync time text, secondary button), then underlined tabs (`role="tablist"`, left/right arrows switch tabs, the active tab has a 2px `--accent` underline). The tab panel is a surface box with radius lg. When there is a shared collection, the page is as tall as the window (`height` from `--titlebar-height` and the `main` padding `--main-pad-top`/`--main-pad-bottom`, at least 480px): the title, bar, and tabs stay in place, and the panel scrolls. Above 760px (container query) the commit list and its details sit side by side at 2:3, each with its own scroll area (4px padding so the focus ring is not clipped); below that, the panel scrolls as a whole. Commit dot 10px `--accent`; a merge is a 2px circle. File change chips: _Added_ (ok), _Updated_ (accent-soft), _Removed_ (err).
- `FieldChanges` is a three-column table (Field, before, after) with mono values in `--surface-muted` boxes, used by the request history dialog and the commit details.
- Sharing to a repository that already has content: a `--warn-bg` box bordered with `--warn-line` (radius md, padding 14px) with a 13px h3 title, an explanation, a 12px mono file list in a `--surface` box (max height 180px, scrollable), then Cancel and a red "Replace files and share" button (`--err` with `--on-accent` text, the same as the ConfirmDialog action button).
- The _sign in_ status (err) in the panel carries a secondary **Sign in** button inside the status box (the same position as Resolve), only for `https://` addresses. While Git waits for sign-in, the button is disabled and reads "Signing in…". A rejection shows in the panel's `.error`.
- The _not synced_ status (warn) shows in the header and the panel while there are saved changes that have not been sent with Sync now. There is no automatic sync.
- **Access groups** (`AccessSection`) sit below the invitation text, separated by a `--border-soft` line: a 13px h3 title, a 13px muted hint, then a `dl` row _This device_ with a 12px mono fingerprint (`XXXX-XXXX-XXXX-XXXX`) and a Copy link. That row only appears once the collection has a group, because reading it creates the device key.
  - Each group is a box bordered with `--border-soft` (radius md, padding 14px): a 13px 600 h4 name and a text chip _You are a member_ (ok), _No access_ (sunken), or _Locked, needs a check_ (warn). Below that, 12px muted text "Holds token, apiKey (Staging)." and the member list (name, muted email, mono fingerprint, small Remove or Leave button).
  - _Asking for access_ (h5) lists requests with their fingerprints. Members get a 12px hint to match the fingerprint over chat or phone, then a Decline (small) and a Grant (secondary) button. This device's own request has a Withdraw button.
  - A group that fails its check shows a `--warn-bg` box with the reason, and a secondary **Repair group** button if this device has checked an earlier version as a member.
  - Every action runs Sync now right away. Remove asks for confirmation (ConfirmDialog), explaining that values already seen stay known to that person, then an `--ok-bg` box lists the variables that need to be changed at their source.
  - The **New group** form (label, 32px input, secondary Create group button) sits below the group list.
- `AccountFields` is used in Share, the account form, and both Join forms. The _Git account (optional)_ field only appears for `https://` addresses; the word "(optional)" uses `--text-muted`. A 12px hint below it explains what an empty field means.

### Import page

- Title **Import** (`import` icon, teal) and an explanation that mentions Export is in each collection. Panels: cards with `flex: 1 1 400px`, padding 18px, gap 14px, 16px h2 title, 12px muted hint.
- Contents: file (Codestamp, Collection v2.0/v2.1, environment, OpenAPI, Swagger, WSDL), Paste cURL, and Join a shared collection. The preview of a Codestamp file adds an _environments_ stat when present.

### Collection panel: Export and History (`ViewPanel`)

- The shared `ViewPanel` shell inside `PanelDialog`: a head with a 15px/600 title, a 13px muted description, optional actions, and a **Close** button; a body with 16px padding and a 12px gap.
- **Export** (`CollectionExport`): a **Format** `Dropdown` (`field` variant, max 520px) with _Codestamp_, _Postman Collection v2.1_, and _Docs as Markdown_, with a 12px muted note about the option below it (like Auth type). The main **Export…** button opens the save dialog. Result (`ExportOutcome`): an `--ok-bg` box "Exported to" followed by the path in mono with a `--text-2` note (secret values are emptied, access groups are not included), or an `--err` box on failure. Environments are exported from the Environments page.
- **History** (`CollectionHistory`): only requests sent from that collection, newest first. The **Clear history** action (quiet, red on hover, ConfirmDialog; disabled while a filter is active or the list is empty) only deletes that collection's history. A search field (36px, blue border on focus) and status pills (All, 2xx, 3xx, 4xx, 5xx, Failed). The list is grouped by day (13px h3) in a scroll area of max 420px: time, colored mono method, name + mono URL, status chip, duration · size, **Open** (opens the request card and scrolls to it) or **Save to collection** when the request no longer exists, and ✕. Empty: "Nothing sent from this collection yet." / "No requests match." Settings › Data & history still deletes all history.
- The History tile on Home opens the History panel of the last opened collection ("Open History of …"), or "Create a collection first".
- Codestamp file: `{"codestamp": "collection", "files": {…}}`, containing the Collaboration repository files under their paths. Secrets and restricted values are emptied, local file paths are not included, and access groups are not included. On import, every part gets a new id, so the file can be imported next to its original collection.

### Dialog

- A native `<dialog>` opened with `showModal()`, with `aria-labelledby` pointing to the title. Escape fires `oncancel`, which calls the component's close function.
- Structure: header (16-18px title, 13px muted description, 32px ✕ button), a scrollable body, then the footer (status/notes on the left; actions on the right, secondary then primary). Sections are separated by `--border-soft` lines.
- ConfirmDialog: 420px wide, a title phrased as a question ("Delete?", "Stop syncing?"), a message explaining the consequence, then Cancel and a red button labeled with the action.

### Copy as code (`CodeDialog`, `lib/request/codegen/`)

- A 760px dialog: header (title "Copy “name” as code", ✕), an options row, a code viewer (read-only `CodeEditor`, resizable height), and a footer (note on the left, _Copy_ on the right).
- Options row: two columns with 12px/500 muted labels, **Language** and **Values**, each a `Dropdown` with the `field` variant. Grid `repeat(auto-fit, minmax(200px, 1fr))`, so in a narrow window they stack instead of stretching to the right.
- _Language_ is remembered per device (`localStorage` `codestamp.codeLanguage`). _Values_ always starts at **Environment variables**; _Filled in (shows secrets)_ is never remembered.
- The footer note follows _Values_: "Reads BASE_URL, TOKEN from environment variables.", "Variables stay as {{name}}.", or "Values are filled in as they would be sent now.", always ending with "Pre-request scripts are not run."
- The generated code always has the same layout, separated by blank lines: imports, values from the environment (only the ones used), URL and query, headers, body, send, then print the status and body. One query pair, header, or field per line. Indentation follows the language (Go uses tabs; Python, PHP, Java, Kotlin, C#, Swift, Rust, and PowerShell use 4 spaces; JavaScript, Dart, Ruby, cURL, and HTTPie use 2 spaces).
- Names: `{{baseUrl}}` becomes the environment variable `BASE_URL` and the local variable `baseUrl` (or `base_url` in Python, Ruby, and Rust). Names that clash with keywords or the generator's local names (`url`, `body`, `headers`, …) get the suffix `Value`. Environment variables the system already uses (`USER`, `PATH`, `HOME`, …) get the suffix `_VALUE`.
- _Language_ order: cURL, HTTPie, PowerShell, JavaScript fetch, JavaScript axios, Python requests, PHP Guzzle, Go net/http, Java HttpClient, Kotlin OkHttp, C# HttpClient, Dart http, Swift URLSession, Ruby Net::HTTP, Rust reqwest. Labels are always "language + library", so it is clear what needs to be installed.

### Banner

- Shown at the top of the content through `notices`. One message at a time, with a Dismiss button. An info message may carry one action button before Dismiss, for example _Open variables_, which opens the related collection's Variables panel, _Undo_ after deleting a collection or request, and _Open_ after restoring one from Recently deleted.

### Recently deleted

- Deleted collections and requests are kept for 30 days (`deleted_items`, schema 26) as unchanged copies of their database rows: sections, requests, examples, environments owned by the collection, variables (secrets stay vault-encrypted), Catalog info, Run results, Collaboration links, access groups, and sign-in tokens. Orchestration steps and History entries that point to the item are reconnected when it is restored.
- The delete dialog says "You can restore it from Recently deleted for 30 days." (not "cannot be undone"). After the delete, an info banner reads "“Name” was deleted. It stays in Recently deleted for 30 days." with an **Undo** button, which restores the item and opens it.
- **Recently deleted** page (trash icon, hue `slate`, in the status bar, Menu › Go, Quick search, and Home): title + explanation, a quiet **Delete all** button (red on hover, with ConfirmDialog). The list works like History: _Collection_/_Request_ chip, name 500, a muted line (request count, "In “Collection”", or a note that its collection was deleted too, so Restore creates a new collection with the same name), "Deleted … · N days left", a **Restore** button (small secondary), and a ✕ button "Delete … for good" with ConfirmDialog. Empty: "Nothing deleted in the last 30 days."
- A request goes back to its section if that section still exists; if not, it goes outside any section. A collection that was shared reconnects to its repository, and its clone is rebuilt from the last synced commit.
- Error: `--err-bg` / `--err`, border `--err-line`, `role="alert"`.
- Info: `--accent-soft` / `--accent-text`, `role="status"`. Examples: a teammate deletes the request that is open, or Sync now brings access to a group this device asked for ("Mate gave this device access to Ops. Its restricted values are ready to use.").
- The update banner (`UpdateBanner`) stands on its own above the `notices` banner, with the same info colors: version text, then a _What's new_ text button (when there are notes), _Update and restart_, and _Later_. While downloading, the buttons are replaced by a percentage and a 6px `<progress>` (fill `--accent` over `--surface`). On failure: error colors, `role="alert"`, _Try again_ and _Later_ buttons.

### Inline status box

- A box with radius md, padding `10px 14px`, 13px, with a semantic background/text pair. Examples: export result (ok), pending conflicts (warn), sync status in the Collaboration panel (ok/warn/err with a dot).
- Form error: `--err-bg` background, padding `10px 12px`, `role="alert"`.

### Key-value table

- 13px, header 500 muted, rows separated by `--border-soft`, cells 30px tall.
- Disabled row: muted, struck-through text.
- File row: a dashed-border button holding the file name.
- A collection variable replaced by the active environment: a 12px `--warn` note under the name ("Replaced by “DEV”"), not color alone.
- The Type column in the variables table (150px wide) holds Text, Secret, and _Restricted: group name_ for each group this device can use. Restricted values stay secret on the device and are sent encrypted for their group.
- A row restricted to a group this device does not belong to is locked: read-only name, the value cell holds 12px muted text "Restricted to Ops. Ask a member for access in Collaboration.", the Type column reads _Restricted: Ops_, and there is no delete button. The URL preview writes such a name as `{{opsKey}} (restricted to Ops)`, not simply as undefined.

### Environments page

- The list on the left is grouped under 12px muted group titles: _On this device_, then one group per collection (emoji + name).
- Under the environment name is a **Belongs to** Dropdown (_This device only_ or a collection) with a 12px muted hint that explains the effect: only on this device, shared through Git on Sync now (secrets stay on each device), or carried along when the collection is shared.

### Dropzone and drop target

- Border 1.5px dashed `--border-strong`. On hover or when active: ink border and `--accent-soft` background.
- Drop target in a list: `--accent-soft` background with a 1.5px dashed ink outline.

### Code editor

CodeMirror with a transparent background inside a `--surface-muted` frame (border and radius md). The gutter has no border, active line `--code-active-line`, selection `--accent-soft`, syntax colors from the `--code-*` tokens.

- Follows Settings › Editor: indentation (None by default, 2 spaces, 4 spaces, Tab; also used by Beautify), auto-closing brackets/quotes, and wrapping of long lines. Changes apply at once without reopening the card. With None, Enter starts at column 0 and Tab moves focus to the next control.
- The body editor, WebSocket messages, and MCP arguments color `{{variable}}` the same way input fields do.
- **Find** (`editor/search.ts`, Ctrl+F): a panel above the editor, `--surface-muted` background with a `--border-soft` bottom border. It holds a 28px mono field (border `--border-strong`), a 12px muted match count with tabular numbers ("3 of 12", "No matches", or "Invalid pattern" in `--err`), 28px Previous/Next icon buttons, the mono toggles `Aa` `ab` `.*` (`aria-pressed`; lit with `--accent-soft` and a `--border-strong` ring), then Close.
  - Clicking a button does not move focus out of the field.
  - Every match has a `--warn-bg` background with a `--warn-line` border, and the selected match gets a 2px `--warn` frame, so it stays visible without telling colors apart.
  - While a card is open, Ctrl+F outside the editor (from anywhere on the collection page) opens find in the response body (`data-find`), instead of bringing up the browser's search field in the top corner of the window.

### Settings

- Section navigation on the left (200px, sticky): Appearance, Editor, Requests, Proxy, SSL/TLS, Data & history, About & shortcuts. The active section is raised like a sidebar item.
- Section content is a panel (surface, border, radius lg). Shared classes (`.panel`, `.field`, `.input`, `.check`, `.button`, `.primary`, `.segmented`, `.error`, `.status`) are defined once in `SettingsView` with `:global`.
- Editor, Requests, and Data & history save automatically 400 ms after the last change; Rust checks the values, and error messages show in the panel. Proxy has a Save button because the address and password are typed in gradually.
- Server certificate verification cannot be turned off; the Requests panel explains this and points to SSL/TLS (minimum TLS version, CA certificate, client certificate).
- Actions that cannot be undone (Clear history, Restore from backup, Remove certificate) go through a confirmation dialog.
- About › Updates: a status sentence (a new version, or a green `.status` "You have the latest version"), release notes as plain text in a `--surface-muted` box (max height 240px, scrollable), then _Update and restart_ (`.primary`) and _Check for updates_ (`.button`).

### Theme picker (`ThemePicker`)

A grid of cards (min 150px). Each card holds a mini preview that uses that theme's own `data-theme` (sidebar, a GET row with Send, a POST row with `{{baseUrl}}`), a name, and a caption. System shows Light and Dark side by side with the caption "Now Dark/Light". Selected card: `--text` border and ring. Plain radio buttons sit underneath, so the keyboard arrows work.

### MCP card

- Target: a transport choice (STDIO/HTTP, `mcp` stamp color, locked while connected), then the command (`npx -y …`) or server URL, then a Connect/Disconnect button colored `--method-mcp`.
- Left panel: server info (name, version, MCP version, instructions), Tools/Prompts/Resources tabs with their counts, a Headers tab (HTTP) or Environment tab (STDIO), and a Paste config tab for pasting a `{"mcpServers": …}` config from another client.
- The selected item uses a `--method-mcp-bg` background and a `--method-mcp-line` border. Selecting an item fills the arguments from the schema (`default`, then an empty value that matches the type).
- JSON arguments go in the code editor; the Run tool / Get prompt / Read resource button stays disabled until the arguments are a JSON object.
- Right panel: a Result tab (a Result, Tool error, or Error code chip; a readable view per content block, or Raw JSON) and a Messages tab (JSON-RPC log: ↑ sent, ↓ received, server stderr and logging, info, error).
- The selected tool, prompt, or resource is saved with the request (and syncs with the team), with its arguments in the body.

### Quick search (`QuickSearch`)

- `Ctrl+Space` or the search button in the sidebar (the button reads only "Search"). A modal `<dialog>` in the top third of the screen that drops 10px from scale 0.98 while fading in (240 ms, `cubic-bezier(0.2, 0.7, 0.2, 1)`, the same curve as the Home tiles), with a `--backdrop` background that fades in over 200 ms. Only opacity and transform move, so the animation runs on the compositor. The backdrop is **not** blurred: a window-wide blur repaints the whole app on every frame and makes opening feel heavy. `prefers-reduced-motion` turns the animation off.
- No `kbd` hints (Tab, Esc, arrows, Enter) in the field or the footer; the footer holds only a sample path or an error message.
- One large field (16px). When the typed text is the start of an API path, the rest of the matching path shows faintly behind the text and `Tab` completes it.
- Results are grouped (requests, collections, Settings pages) with a method stamp, name, mono URL, and muted location; the matching part is bold in `--accent-text`. Up/down arrows select, `Enter` opens, `Esc` closes.
- **Docs** group: collections (emoji), sections (Docs icon), and requests (method stamp) whose name or description matches. The detail is a description excerpt, or "Request docs" when the name is what matched. When the Docs page is open, this group comes first.

### GraphQL card

- `GQL` stamp (tone `gql`) in front of the endpoint; Send is colored `--method-gql`.
- The Query tab holds the query editor (highlighting + autocomplete from the schema) and a JSON variables editor below it. A _Fetch schema_ / _Fetch again_ button with the number of loaded types; the autocomplete popup uses the editor tooltip tokens.

### SOAP card (`SoapPanel`)

- `SOAP` stamp (tone `soap`) in front of the endpoint; a regular **Send** button (always POST). The first tab is named **Envelope**.
- Envelope tab contents:
  - WSDL row: a URL field (or the file name when it came from _Choose file…_, without the full path, with a _Use a URL_ link), the _Choose file…_ and _Load WSDL_ / _Reload WSDL_ buttons. Enter in the URL field also loads.
  - A load error shows as an `--err` box. Warnings (imports that could not be read) show as a `--warn` list, at most 3 lines plus "…and N more".
  - An **Operation** Dropdown (`Operation · Binding · SOAP 1.1`), a **SOAP version** segmented control (1.1 / 1.2), and a **SOAPAction** field.
  - Below them, the operation's documentation (or a short hint) and a _Reset the envelope to the sample_ link. The XML editor uses `BodyEditor` with `fixedMode` (no body type choice).
- Selecting an operation fills the version, action, endpoint (when empty), name (when it is still the default or the previous operation's name), and envelope (when empty or still the same as the previous sample). The user's text is never overwritten silently.
- The XML response is pretty-printed. A SOAP Fault adds a red _SOAP Fault_ chip and a `code: reason` error box.

### gRPC card (`GrpcPanel`, `GrpcReplyPanel`)

- `gRPC` stamp (tone `grpc`) in front of the `grpc://` / `grpcs://` address; the main button is labeled **Invoke** and stays disabled until a method is selected.
- Source row: a _Server reflection_ / _.proto file_ segmented control, a file button (file name only, the full path is not shown), then _Load services_ on the right.
- The method Dropdown (mono) lists `package.Service/Method · unary / server stream / client stream / bidirectional stream`.
- Message tab (JSON editor with a hint that matches the stream type), Metadata (key-value table), and Auth (without Digest and AWS, which do not apply to gRPC).
- Reply panel: a gRPC status chip (`0 OK` green, others red), duration and message count, the status message as an error box, then the Messages tab (one message in a read-only editor; streams as a numbered list), Headers, and Trailers.

### Examples and mock server

- _Examples_ tab on the request card: a list of examples (`EditableText` name, status chip, body editor), with a _Save as example_ button in the response panel.
- _Mock server_ panel (collection toolbar, status dot while running): a port field (empty = automatic), Start/Stop, a copyable address, and a request log (time, status chip, method and path, the name of the example that answered or "no matching example").

### Event stream (SSE)

- The body tab becomes _Events · n_. Stream header: a pulsing green dot + "Streaming · n events" + a Stop button; when finished, "Stream ended", or a red error message if the connection drops.
- Each event: mono time, event name (when it is not `message`), id, then the data (JSON pretty-printed) in a mono block.

### Orchestration (`FlowsView`, `FlowEditor`, `FlowMap`, `FlowStepCard`)

- A two-column page: the flow list (200px, active item like a sidebar item) and the editor. Editor header: `EditableText` name (title), save status, a **Run flow** button (ink background; turns into a red outlined Stop while running), and Delete flow.
- **Map card** (`section` "Flow map", surface, border, radius lg): the top row holds the _Environment_ label and the environment picker on the left, and the run summary on the right (`role="status"`, plus a `--err` stop note). Below it is the step map (`FlowMap`).
- **Step map**: a grid with Start and End columns `--edge` wide (80px), an Add step column `--add` wide (52px), and step columns that share the remaining width equally (`minmax(120px, 1fr)`). Four steps (692px) fit without scrolling in a 1280px window, also when the page has a scrollbar; at that minimum width a DELETE stamp still fits in a node. When they do not fit, only the map scrolls sideways, and the selected node is scrolled into view without scrolling the page.
  - `<ol aria-label="Steps from start to end">` uses subgrid. A 1.5px `--border-strong` line runs from the center of the Start column to the center of the End column, with a chevron in front of every point. Each point sits in the center of its column.
  - **Add step**: a round `IconButton` with "+" (tooltip and name "Add step", `aria-haspopup="dialog"`), between the last step and End.
  - A step node is a button (radius lg, surface, border `--border`): a top row with a 14px status mark, an 11px mono number, and a `MethodStamp` at size `sm` (5px gap); then the name at 13px/500 (truncated with an ellipsis) and a short 11px mono result (code · time; `--ok`/`--err`). A failed node has an `--err-line` border, a skipped one a dashed border. Selected node: `--text` border, `--shadow-raise`, `aria-current="step"`. Left/right arrows move between nodes and move focus.
  - End: a pill with a flag; green `--ok` when everything passes, red `--err` when something fails. Its tint background is layered over `--surface` so the line does not show through in dark themes.
- **Value lines** (above the nodes, only for the selected step, `aria-hidden`): one line per value, from the center of the source column to the center of the target column, a 1.5px line with top corners rounded at 10px and a chevron at every step that uses that value. A `FlowValueChip` label (`{{name}}`, mono, pill) sits at the top center, on a surface background. Colors: `--text-muted` before a run, `--ok` after the value is passed on, `--err` dashed and struck through when it is not found.
  - Values received: every `{{name}}` that the step's request sends (`StepRequest.variables`), from the last earlier step that passes that name on, the same as during a run. Values passed on: to every later step that sends that name, until another step passes on the same name. Single source: `lib/flow/lineage.ts`.
- **Stop line**: after a run that stopped because a step failed, a dashed `--err` line below the nodes from the center of that step to the center of End, with the label "Stopped at step N". Because the end columns have a fixed width, its left margin is computed from the step column width only: `(100% - --add - --edge) / (2 × number of step columns)`.
- **Step detail** (`FlowStepCard`, always open, only for the selected step): the header holds the status mark (`FlowStepMark`: empty circle; dashed = skipped; spinning `--focus` = running; `--ok` check; `--err` cross), mono number, `MethodStamp`, the name and "collection · URL", the full result, then Move left / Move right and Remove. A failed card has an `--err-line` border.
  - Content: a _Uses_ row (value chips + "from step N", only when there are any), a settings row (_If this step fails_, Change request, Open request), Checks / Pass on / Response tabs, rules as rows of a dropdown + mono input, and a _Last run_ box (`--surface-muted`) with ✓/✗ per check and test.
  - A removed step is replaced by its neighbor; a flow with no steps shows only the map (Start, Add step, End).
- Request picker (`FlowStepPicker`): a dialog with a search field and a list of HTTP/GraphQL requests from all collections (method stamp, name, URL, collection).

### Catalog (`CatalogView`, `components/catalog/`)

- **List** (`CatalogList`): a title with the catalog `NavIcon`, description, Refresh button. Four `StatTile`s (13px muted label, 26px/600 number, 12px note; the note is colored `--err`/`--warn` only when the number is bad, for example rising errors). Then a search field, a tag `Dropdown` (only when there are tags), and health status pills like the History filters.
- A table inside a card (surface, border, radius lg) with `container-type: inline-size`: below 760px the p95, Owner, and Last activity columns are hidden and the owner moves to a small line under the name. Sortable column headers are buttons with an arrow; `th` uses `aria-sort`. The API name is a button that opens its page.
- **`HealthChip`**: a pill with a mono score and a label (Healthy / Needs attention / Critical / No data) using the semantic ok / warn / err pairs; No data is neutral. Color always comes with text.
- **API page** (`CatalogApi`): an "All APIs" link, 44px emoji tile, h1 name, health chip, owner, tags (outlined pills), then Edit owner and tags (`aria-expanded`, opens an inline `InfoForm`), Run all, Open collection. Tabs styled like the request card tabs: Overview, Runtime, Tests, Quality · number of failing rules.
- **Overview**: a 48px score, then three rows, one per score part (name + weight, an 8px meter with a `--surface-sunken` track and an ok/warn/err fill, value + label). An explanation of the weights and thresholds sits below them. "What it holds" (`<dl>`) and "Used in Orchestration" cards.
- **Chart** (`TrafficChart`): stacked columns per day (max width 24px, 4px radius on the top end, 2px gap between segments), gray `--text-muted` for answered requests and `--err` for errors, always in the same position in the stack. A legend on top, 1px `--border-soft` gridlines, a tooltip on hover, and a **Show as table** button as the alternative for keyboard and screen reader users. In the Dark theme the color separation for color blindness is only 7.5, so the gaps, positions, and legend must stay.
- **Quality** (`RuleList`): a card per rule with a round ✓/✗ mark (ok/err), title, Error/Warning chip, description, "N of M fail"; findings as a list (method stamp, request name as a button, muted detail), the first five, then "Show all N".
- Loading, empty, and failed states always have text: "Loading the catalog…", "No APIs yet" with a Create a collection button, "No API matches these filters." with Clear filters, and an `--err` box with Try again.

### Docs (`DocsView`, `components/docs/`)

- **Index + one document**: the index on the left (280px), only one document on the right. The **Save as Markdown** button sits at the top right, and its result shows as an `--ok` box with the path, or `--err`.
  - Only one document is rendered, so a collection with hundreds of requests stays light.
- **Index** (`nav` "Docs index"):
  - Contents from the top: a collection `Dropdown` (`field` variant, emoji + name), a **Search docs** field (Catalog search field style: magnifier icon, `--border` border, blue on focus; Ctrl+F on the Docs page), and a 12px muted count line ("321 requests" + a `Ctrl` `F` hint, or "3 of 321 requests").
  - Below that: **Overview**, requests outside any section, then each section (`FoldButton` chevron, name 600 + mono count) with its requests indented 18px (`IndexEntry`: small method stamp + name).
  - Sections **start folded**, so the index of a large collection stays short. The chevron (Enter/Space) opens or folds a section; selecting a section, opening a request through Previous/Next, or a Quick search result opens its section. Open sections are remembered per device (`localStorage` `codestamp.openDocsSections`). While searching, every section shows its results and the chevron is disabled.
  - The selected item looks like an active sidebar item (`--surface` background, 1px `--border` ring, `aria-current="page"`). Up/down arrows move between items and show them right away.
  - From 700px wide (container query), the index sticks below the title bar and has its own window-height scroll. Below 700px, the index sits above the document with a list of at most 260px.
  - No results: a dashed box with text and a Clear search button.
- **Overview**: the collection card (44px emoji tile, 22px/600 name, request and section counts) and its description, then the Auth row.
  - **Variables** are folded in a `<details>` ("Variables" + count) that holds a mono Name/Value table. A secret is written "Secret, set on each device", and values whose names look like credentials (token, password, key) are masked through `documentedValue`, the same as in Save as Markdown.
  - The **Sections** list (name + request count) opens the chosen section.
- **Section**: a card with a 22px title, request count, description, and a list of requests (method stamp, name, muted mono URL) that opens the request document.
- **Request** (`RequestDoc`): method stamp, 16px/600 name, an **Open request** button, mono URL, description, facts (Auth, gRPC method, SOAP operation, MCP transport), `DocTable` tables (Path variables, Query parameters, Headers/Metadata, Body form) with written credential values masked through `documentedValue`, a `<pre>` body block (max 320px, scroll), and examples in `<details>` with a status chip.
  - Below it are **← Previous** and **Next →** buttons (request name under a 12px muted hint) that follow the index order.
- A Docs result from Quick search selects that item, then scrolls to its document and focuses it (`scroll-margin-top: 16px`).
- **Description** (`DescriptionBlock`): when empty it shows "No description for … yet." + an "Add a description" link; when filled it shows the Markdown + an "Edit description" link. Edit mode: a Write/Preview segmented control, a mono character count (red when over 20,000), a mono textarea (`--surface-muted`, blue border on focus), an error message with `role="alert"`, a `Ctrl Enter`/`Esc` hint, then Cancel (secondary) and Save (ink primary). Only one description is edited at a time.
- **Markdown** (`Markdown`, `MarkdownNodes`): rendered node by node from mdast, with no raw HTML. Markdown headings are shifted below the page heading (`headingOffset`), inline code `--surface-sunken`, code blocks `--surface-muted` with a border, tables with a `--border` border and a `--surface-muted` header, quotes with a neutral left border. Links using http/https are underlined and open in the browser; other links are plain text with a dotted underline; images are written "Image: alt"; raw HTML shows as code.

### Method stamp (`MethodStamp`)

One component for the method/request type stamp in the request row, Quick search, and Orchestration: label from `stampLabel()`, tone from `stampTone()`, size `md` (11px) or `sm` (10px).

### Path variables (`PathVariablesTable`)

A table above Query parameters whose rows follow the `:name` segments in the URL (names cannot be edited in the table, values support `{{variable}}`). It hides itself when the URL has no path variables.

### Program approval dialog (MCP)

Appears before an STDIO server runs for the first time, and every time its program, arguments, or environment change (including through team sync or a changed variable). Shows the program, the argument list, and the environment names (not their values). Main button "Start program", Escape = cancel.

### Focus mode

- The sidebar and other cards are hidden; the content background becomes `--surface-sunken`.
- A pill-shaped bar at the top holds the title, a "Hide secrets on screen" option, and an Exit `Esc` button.
- A larger card: radius 18px, URL field and Send 46px tall.

## 8. Interaction states

| State           | Treatment                                                                                                                                                                               |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Hover           | Outlined button: `--border-strong` border (tile/icon: `--text`). Transparent button: `--surface-sunken` background                                                                      |
| Keyboard focus  | Global `:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px }`. Inputs switch their border to `--focus`; the outline may be removed only when that border replaces it |
| Selected/active | Sidebar item: raised (surface + border + `--shadow-raise`). A button that opens a panel/dialog, while it is open: `--text` border and `--accent-soft` background                        |
| Pressed         | `:active`: `--surface-sunken` background (outlined buttons, tiles) or `--accent-soft` (transparent buttons, sidebar items)                                                              |
| Disabled        | `--surface-sunken` with `--text-muted`; `cursor: not-allowed` (global)                                                                                                                  |
| Busy            | Verb label + "…"; `aria-busy` on the response area                                                                                                                                      |
| Failed          | `--err` text or background with a message the user can act on                                                                                                                           |

## 9. UI text

- The UI language is **English**, and so is the project documentation. `README.id.md` is the only document in Indonesian.
- **Sentence case** in all labels and titles: "Add request", "Delete collection", "Sync now".
- Buttons are verbs (+ an object when needed). Confirmation buttons name the action ("Delete", "Stop syncing", "Discard"), not "OK".
- Names that belong to the user go in curly quotes: “Login”. Running processes use the single ellipsis character "…".
- Confirmation messages explain the consequence: "… will be deleted. This cannot be undone."
- Error messages say what happened and what the user can do, without internal details (paths, queries, stack traces), following the project's security rules.
- Singular and plural forms must be correct: "1 request", "2 requests".
- A short hint (12-13px) under a panel or field title explains what happens to the data, for example what is shared with the team, or that the Base URL becomes `{{baseUrl}}`.
- Do not promise features that do not exist (e.g. accounts/login); describe how it really works (sign-in through Git).
- No em dash (—) and en dash (–), in the UI or in documents. Separators use a colon or a period ("status equals 200: got 201"). Ranges in the UI are written "1 to 4"; in documents a hyphen is enough (11-20 px).
- Titles say what the screen does ("Requests you sent", "Run requests in order"), not a slogan ("Make it yours") or an overclaim ("Find anything", "Everything you sent").
- Name the actor: "Codestamp keeps only what you typed", not "only what you typed is kept". Avoid empty words such as _journey_, _seamless_, _powerful_, _effortless_.

## 10. Accessibility

- Prefer native elements: `<button>`, `<dialog>`, a `<label>` that wraps or points to its input, `<fieldset>`/`<legend>` for radio groups, `<table>`, `<dl>` for the shortcut list.
- ARIA attributes:
  - icon buttons: `aria-label` (emoji button: "Change icon", or a description of the current icon);
  - toggles: `aria-pressed`; popover triggers: `aria-haspopup="dialog"` + `aria-expanded`;
  - panels that open and close: `aria-expanded`;
  - areas or groups: `aria-label`, for example `section[aria-label="Team"]`.
- Announcements: `role="alert"` for errors, `role="status"` for updates and status.
- Keyboard:
  - every control can be reached with Tab;
  - Escape closes only the topmost layer: the emoji picker, then the dialog, then focus mode (except when the cursor is in a code editor);
  - Alt+↑/↓ moves a request or section, then focus returns to the moved item.
- `.visually-hidden` is used for hidden file inputs and screen-reader-only text. Svelte strips leading and trailing whitespace from element content, so the separating space is written outside the span: `Open request <span class="visually-hidden">{name}</span>`. Decorative emojis in the sidebar get `aria-hidden="true"` because the collection name is already read out.
- Color is never the only indicator.

## 11. On-screen privacy

- Secret values always show as `SECRET_MASK` (`••••••••`, from `src/lib/privacy/secrets.ts`) until the user presses Show.
- Focus mode and Capture mask tokens and cookies, while the auth scheme stays visible (`Bearer ••••••••`).
- New components that show sensitive values must use that helper, not their own masking logic.

## 12. New component checklist

1. Colors, radius, and shadows come from tokens. No new hex values in components; method colors go through `methodTone()`.
2. Text sizes come from the scale in section 3, written in px on a 14px base. Code text uses `--font-size-code`.
3. Buttons follow the variants in section 7: one primary (ink) per area, destructive actions through ConfirmDialog.
4. Layout uses flex-wrap and flex-basis. Test in a narrow window (±600px) and at UI size 20.
5. Dialogs use the native `<dialog>` with width `min(N, calc(100vw - 32px))`, and Escape closes them.
6. Interface icons are inline SVG as described in section 6, with labels. Emojis only for collection icons, and only from the catalog in Rust.
7. UI text is in English, sentence case, with generic error messages.
8. Styles are written in the component's `<style>` block (scoped by Svelte). Only tokens and global utilities (`.mono`, `.emoji`, `.visually-hidden`) live in `src/app.css`.
9. `npm run check` and `npm run lint` (svelte-check, ESLint, Prettier) pass without warnings.

## 13. File locations

| File                                            | Contents                                                 |
| ----------------------------------------------- | -------------------------------------------------------- |
| `src/app.css`                                   | Tokens, reset, global utilities                          |
| `src/lib/appearance/presets.ts`                 | Font presets and size ranges                             |
| `src/lib/request/methods.ts`                    | HTTP method tones and colors                             |
| `src/lib/request/codegen/`                      | Copy as code generators, one file per language           |
| `src/lib/components/CodeDialog.svelte`          | Copy as code dialog (Language, Values, code)             |
| `src/lib/format/status.ts`                      | HTTP status tones                                        |
| `src/lib/team/status.ts`                        | Team sync status tones and text                          |
| `src/lib/components/AccountFields.svelte`       | Git account, name, and email fields for Collaboration    |
| `src/lib/editor/setup.ts`                       | Code editor theme and syntax highlighting                |
| `src/lib/capture/render.ts`                     | Capture image palette (light and dark)                   |
| `src/lib/privacy/secrets.ts`                    | Masking of sensitive values                              |
| `src-tauri/src/storage/icons.rs`                | Emoji catalog, automatic rules, icon whitelist           |
| `src/lib/state/icons.ts`                        | Emoji catalog loaded once from Rust                      |
| `src/lib/components/EmojiPicker.svelte`         | Collection emoji picker                                  |
| `src/lib/components/IconButton.svelte`          | Icon button with tooltip and status dot                  |
| `src/lib/components/HomeView.svelte`            | Home page                                                |
| `src/lib/components/NewCollectionDialog.svelte` | New collection dialog                                    |
| `src/lib/components/ChoiceCard.svelte`          | Radio choice card (32px icon, title, description)        |
| `src/lib/components/ImportView.svelte`          | Import page                                              |
| `src/lib/components/StatusBar.svelte`           | Status bar: Settings, data, and version                  |
| `src/lib/components/ViewPanel.svelte`           | Collection panel shell (title, actions, Close)           |
| `src/lib/components/PanelDialog.svelte`         | Modal dialog for every collection panel                  |
| `src/lib/components/CollectionExport.svelte`    | Export collection panel with a format dropdown           |
| `src/lib/components/CollectionHistory.svelte`   | Per-collection History panel                             |
| `src/lib/components/ExportOutcome.svelte`       | Export result: path and notes, or an error               |
| `src/lib/components/DeletedView.svelte`         | Recently deleted page                                    |
| `src/lib/state/deleted.svelte.ts`               | Recently deleted list, Undo, and Restore                 |
| `src/lib/components/CollectionStart.svelte`     | Empty collection view                                    |
| `src/lib/components/ThemePicker.svelte`         | Theme picker with previews                               |
| `src/lib/components/Dropdown.svelte`            | Styled dropdown that replaces `<select>`                 |
| `src/lib/components/NavIcon.svelte`             | Colored icons for the sidebar and page titles            |
| `src/lib/components/SettingsView.svelte`        | Settings frame and shared panel styles                   |
| `src/lib/components/settings/`                  | A panel for each Settings section                        |
| `src/lib/components/VariableText.svelte`        | Text with colored `{{variable}}`                         |
| `src/lib/format/variables.ts`                   | Variable reference pattern and parser                    |
| `src/lib/components/McpPanel.svelte`            | MCP card content (server, arguments, result, log)        |
| `src/lib/components/McpApprovalDialog.svelte`   | Approval before an MCP program runs                      |
| `src/lib/components/QuickSearch.svelte`         | Quick search `Ctrl+Space`                                |
| `src/lib/dom/reveal.ts`                         | Scroll to an element and move focus to it                |
| `src/lib/search/`                               | Search result matching and grouping                      |
| `src/lib/editor/search.ts`                      | Find (Ctrl+F) in the code editor                         |
| `src/lib/docs/filter.ts`                        | Filtering of documents and the collection list in Docs   |
| `src/lib/components/GraphqlEditor.svelte`       | GraphQL query, variables, and schema                     |
| `src/lib/components/GrpcPanel.svelte`           | gRPC card source, method, message, metadata              |
| `src/lib/components/GrpcReplyPanel.svelte`      | gRPC reply (status, messages, headers, trailers)         |
| `src/lib/components/SoapPanel.svelte`           | SOAP card WSDL, operation, version, and SOAPAction       |
| `src/lib/components/ExamplesPanel.svelte`       | Request card Examples tab                                |
| `src/lib/components/MockPanel.svelte`           | Collection mock server panel                             |
| `src/lib/components/PathVariablesTable.svelte`  | `:name` path variable table                              |
| `src/lib/components/OAuthFields.svelte`         | OAuth 2.0 settings and tokens                            |
| `src/lib/components/FlowsView.svelte`           | Orchestration page (flow list and editor)                |
| `src/lib/components/FlowEditor.svelte`          | Map card and the selected step's detail                  |
| `src/lib/components/FlowMap.svelte`             | Step map, value lines, and stop line                     |
| `src/lib/components/FlowStepCard.svelte`        | Step detail (Uses, Checks, Pass on, Response)            |
| `src/lib/components/FlowStepMark.svelte`        | Step status mark                                         |
| `src/lib/components/FlowValueChip.svelte`       | `{{name}}` chip for values that are passed on            |
| `src/lib/components/FlowStepPicker.svelte`      | Request picker dialog for steps                          |
| `src/lib/components/MethodStamp.svelte`         | Method/request type stamp                                |
| `src/lib/components/CatalogView.svelte`         | Catalog page (list or a single API)                      |
| `src/lib/components/catalog/`                   | List, API page, chart, rules, tiles                      |
| `src/lib/catalog/present.ts`                    | API labels, colors, formatting, filters, and sort order  |
| `src/lib/components/DocsView.svelte`            | Docs page (collection list and document)                 |
| `src/lib/components/docs/`                      | Description, Markdown, request block, tables             |
| `src/lib/components/settings/TlsPanel.svelte`   | Settings › SSL/TLS (TLS version and certificates)        |
| `branding/codestamp-logo.svg`                   | App logo: source for the icon and the sidebar logo       |
| `branding/logo-samples/`                        | Logo samples compared before choosing 3a                 |
| `branding/banner-light.svg`, `banner-dark.svg`  | README banner for GitHub light and dark mode             |
| `branding/why/`                                 | README "Why Codestamp" images, light and dark            |
| `branding/OFL-Geist.txt`                        | License of the Geist fonts embedded in the README images |
