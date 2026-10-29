# Stage 26: The Tools group and Catalog

Suite [`stage26-e2e.mjs`](../../../e2e/stage26-e2e.mjs) · 30 checks

The sidebar holds Collections and Tools; other pages are reached through the status bar and Quick search. Catalog scores collections from History, Run all, and quality rules: the list, owner and tags, overview, runtime, quality, Run all results in Tests, flow links, and the columns left in a narrow window. Creates "E2E Catalog" and one flow and deletes both afterwards.

## Requirements

- A local HTTP server started by the suite itself (Node `http`).

## Flow

### 1. The sidebar's Tools group

- the sidebar has Tools; the other pages sit in the status bar
- quick search still lists every page, Catalog included
- Environments, Recently deleted and Settings still open from quick search
- Home › Testing has an Catalog tile that opens the catalog

### 2. Fixture: one API with history, rules to break and a flow

Prepares data or state for the next steps, with no checks of its own.

### 3. The list

- the catalog lists the collection as an API with its error rate and no run yet
- the totals sit above the list
- search narrows the list and an empty result offers to clear the filters
- sorting by error rate puts the highest first and says so

### 4. One API: owner and tags

- the API page has the health chip, no owner yet and four tabs
- a tag with characters that are not allowed is refused with the reason
- owner and tags are saved and shown

### 5. Overview

- the overview splits the score into Runtime, Tests and Quality
- it lists what the API holds and the flows that use it

### 6. Runtime

- Runtime counts 6 sends, 2 errors (33%) and 1 client error from History
- the daily chart stacks errors on answered sends with a legend
- requests are ranked by error rate
- the chart can be read as a table instead

### 7. Quality

- a token typed into a header breaks the secrets rule
- a plain ws:// address and requests without tests are found
- a finding opens its request

### 8. A Run all shows up under Tests

- the catalog comes back to the API that was open
- before any run the Tests tab says so
- after Run all the catalog comes back to Tests with the run kept
- coverage, pass rate and tests come from the run
- each request of the run has its result

### 9. Back to the list: the run, the owner and the tag filter

- the list shows the last run and owner, and filters by tag

### 10. The flow link

- a flow in "Used in Orchestration" opens in Orchestration

### 11. The columns a narrow window keeps

- a narrow list keeps the ranking columns and moves the owner under the name
- the suite ran to the end
- test data cleaned up

---

Previous: [Title bar and the collapsed sidebar](stage-25.md) · [Stage list](../README.md#stage-list) · Next: [None indentation, SSL/TLS, and Docs](stage-27.md)
