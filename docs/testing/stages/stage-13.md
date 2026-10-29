# Stage 13: Themes, Settings, colored variables, and the MCP client

Suite [`stage13-e2e.mjs`](../../../e2e/stage13-e2e.mjs) · 28 checks

Switches themes and opens every Settings section, checks colored `{{variables}}`, then runs MCP requests against a test server over STDIO (Codestamp starts the program after approval) and over Streamable HTTP.

## Requirements

- The MCP test server `mcp-server.mjs` (Node).

## Flow

### 1. Themes and Settings

- Settings has eight sections
- Dark Night paints the window
- the theme is saved and back after a reload
- Settings refuse a bad User-Agent and save good values
- the proxy refuses an address with a path and saves the system proxy

### 2. An MCP request

- the MCP tile opens an MCP card
- variables in a field are coloured
- a program asks for approval before it starts
- the STDIO server connects and lists what it offers
- connecting shows the tools list
- a tool runs with variables filled in
- the program got its environment with variables filled in
- templates follow the schema and structured content shows
- a tool error is shown as one
- arguments must be a JSON object
- a prompt is fetched with its arguments
- a resource is read
- messages show traffic, stderr and logging
- the row shows an MCP stamp
- the chosen resource is saved with the request
- an approved program starts without asking again
- a changed environment asks again, and Cancel starts nothing

### 3. The same server over Streamable HTTP

- the HTTP server connects without asking to start a program
- a tool answered as an event stream shows its result
- disconnecting ends the HTTP session on the server
- a refused connection explains why
- the request is stored as MCP over HTTP with its operation
- test data cleaned up

---

Previous: [Basic look, Home, and the New collection dialog](stage-12.md) · [Stage list](../README.md#stage-list) · Next: [Dropdowns and colored icons](stage-14.md)
