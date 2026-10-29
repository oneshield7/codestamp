# Stage 12: Basic look, Home, and the New collection dialog

Suite [`stage12-e2e.mjs`](../../../e2e/stage12-e2e.mjs) · 41 checks

Checks the basic look (the bundled Geist font), the Home hub, the New collection dialog, the emoji picker, icon and base URL validation in the Rust core, the empty collection tile, the icon toolbar with tooltips and a pressed state, the Send button color per method, the Duplicate dialog, folding sections, and adding a request from Home.

## Flow

### 1. Postmark look

- Geist and Geist Mono are bundled and used
- Postmark tokens are in place
- every collection in the sidebar has an emoji

### 2. Home

- Home shows sections, tiles and a stage
- a number key picks a section and the arrows a tile
- Enter runs the main action
- the command line tile switches its example commands
- a Home button opens its page

### 3. New collection dialog

- the dialog starts on the chosen option and switches
- Cancel closes the dialog
- the name field is focused when the dialog opens
- the icon follows the typed name
- Escape closes the picker but keeps the dialog
- a base URL that is not http(s) is refused before creating
- the new collection opens with its icon and empty state
- the header says it has no requests and was just created
- the base URL is saved as the {{baseUrl}} collection variable

### 4. Emoji picker

- the picker suggests icons for the name
- search finds emoji by keyword
- a chosen emoji is shown everywhere and stored
- a chosen icon stays when the collection is renamed
- Use automatic goes back to the icon from the name
- Escape closes the picker
- an automatic icon follows a new name

### 5. The Rust core checks icons and base URLs

- icons outside the catalogue are refused
- a non-http base URL is refused and nothing is created

### 6. Empty collection tiles

- Paste a cURL command opens Import with this collection chosen
- rows show a numbered step and a coloured method stamp
- the start panel gives way to the adders once a request exists

### 7. Icon toolbar, tooltips and pressed states

- the collection menu is icon buttons with labels
- hovering shows a tooltip, which goes away after
- an open panel's button looks pressed
- the Home stage is light with a dark main button
- the tile and the + button stay pressed while the dialog is open

### 8. Send colour, Duplicate dialog, folding sections

- Send takes the colour of the method
- Duplicate asks for a name and refuses taken or empty ones
- the copy gets the chosen name and opens
- folding a section hides its requests and keeps its count
- a fold is remembered after a reload and opens again

### 9. Adding a request from Home

- Home adds a request of the chosen kind to the open collection
- test data cleaned up

---

Previous: [Collaboration through Git](stage-11.md) · [Stage list](../README.md#stage-list) · Next: [Themes, Settings, colored variables, and the MCP client](stage-13.md)
