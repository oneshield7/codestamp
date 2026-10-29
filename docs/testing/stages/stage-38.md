# Stage 38: Launch screen

Suite [`stage38-e2e.mjs`](../../../e2e/stage38-e2e.mjs) · 9 checks

Every reload runs the launch screen again. For Light, Dark, and Dark Night: the theme is on the page before the app code runs, the screen uses the theme background, the logo is centered, the screen shows for at least 0.5 seconds, the transition is smooth, and no transform is left behind. Styles injected at runtime (CodeMirror, `style` attributes) still apply under the CSP, and reduced motion only fades the app in.

## Flow

### 1. Setup

- styles injected at runtime still apply (CodeMirror, style attributes)
- after launch the screen is gone and the app is left with no transform
- [theme]: the theme is on the page before the app's code runs
- [theme]: the launch screen is drawn in the theme's background
- [theme]: the logo sits in the middle of the window
- [theme]: the app keeps the theme for the next reload
- [theme]: the screen stays at least 0.5 s
- [theme]: the hand-over draws smoothly
- reduced motion: the trace stands still and the app only fades in

---

Previous: [Access groups](stage-37.md) · [Stage list](../README.md#stage-list) · Next: [Window position, the active environment, and folding](stage-39.md)
