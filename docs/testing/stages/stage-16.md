# Stage 16: Path variables and data-driven runs

Suite [`stage16-e2e.mjs`](../../../e2e/stage16-e2e.mjs) · 15 checks

Tests path variables (`/users/:id`) on the request card, when sending, in the URL preview, and on export and import, then runs a collection with a CSV data file. Creates "E2E Paths" and "E2E Paths Import" and deletes them afterwards.

## Requirements

- A local HTTP server started by the suite itself (Node `http`).
- The Windows save or open file dialog, answered by `answer-dialog.ps1`.

## Flow

### 1. The card

- the Params tab lists each :name of the path once
- an empty path variable stops the send with a clear message
- the URL preview shows the filled path
- values go into their segments, encoded, and the query keeps its text
- the tab count includes path variables
- renaming a segment updates the table
- a value comes back with its name and only names in the URL are saved

### 2. Data-driven run

- a CSV data file can be chosen
- one pass per row, each titled with its data
- each pass sends its row as variables and path values
- test scripts read pm.iterationData and pm.info
- without a data file, Iterations repeats the run

### 3. Export and import

- export writes url.variable
- import fills path variables from url.variable and adds the missing ones empty
- test data cleaned up

---

Previous: [Wording, Quick search, and saving the response body](stage-15.md) · [Stage list](../README.md#stage-list) · Next: [Auth: Digest, AWS SigV4, and OAuth 2.0](stage-17.md)
