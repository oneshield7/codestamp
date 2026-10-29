# Stage 12: Tampilan dasar, Home, dan dialog New collection

Suite [`stage12-e2e.mjs`](../../../e2e/stage12-e2e.mjs) · 41 cek

Memeriksa tampilan dasar (font Geist yang dibundel), hub Home, dialog New collection, emoji picker, validasi ikon dan base URL di inti Rust, tile collection kosong, toolbar ikon dengan tooltip dan keadaan tertekan, warna tombol Send per method, dialog Duplicate, lipat section, dan menambah request dari Home.

## Alur

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

Sebelumnya: [Collaboration lewat Git](stage-11.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Tema, Settings, variabel berwarna, dan klien MCP](stage-13.md)
