# Stage 14: Dropdowns and colored icons

Suite [`stage14-e2e.mjs`](../../../e2e/stage14-e2e.mjs) · 11 checks

Checks the styled dropdowns (method colors, blue focus ring, keyboard navigation) and colored icons in the Light and Dark Night themes. Creates the collection "E2E Dropdowns" and deletes it afterwards.

## Flow

### 1. Setup

- the method list opens with each method in its colour
- an open list has a blue edge
- arrow keys and Enter pick the next method
- typing jumps to an option and Escape closes without choosing
- clicking elsewhere closes the list
- the environment picker shows its status dot
- sidebar icons are coloured in Light
- page titles and Settings sections have coloured icons
- Settings uses the same dropdown
- icons and the list follow Dark Night
- test data cleaned up

---

Previous: [Themes, Settings, colored variables, and the MCP client](stage-13.md) · [Stage list](../README.md#stage-list) · Next: [Wording, Quick search, and saving the response body](stage-15.md)
