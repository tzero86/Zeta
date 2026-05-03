import { defineConfig } from "@microsoft/tui-test";
import path from "path";

// tui-test compiles this config into `.tui-test/cache/` before running.
// Use process.cwd() (the tui-tests/ directory) to build absolute paths.
const repoRoot = path.resolve(process.cwd(), "..");

// Resolve the zeta binary path.
// Override with ZETA_BIN env var, e.g.: ZETA_BIN=/abs/path/to/zeta npx tui-test
const binaryPath =
  process.env.ZETA_BIN ?? path.join(repoRoot, "target", "debug", "zeta");

// Wrapper script gives each test a unique ZETA_CONFIG path so config state
// (e.g. preview_panel_open) never leaks between test runs.
const wrapperPath = path.join(repoRoot, "tui-tests", "zeta-wrapper.sh");

export default defineConfig({
  // Terminal dimensions — wide enough for dual-pane layout
  use: {
    rows: 40,
    columns: 220,
    program: {
      file: wrapperPath,
      // Pass the binary path as env so the wrapper can locate it regardless
      // of where tests are run from.
    },
    env: {
      ZETA_BIN: binaryPath,
    },
  },

  // Retry flaky tests up to 2 times in CI
  retries: process.env.CI ? 2 : 0,

  // Capture traces on failure for debugging
  trace: process.env.CI ? true : false,
  traceFolder: path.join(process.cwd(), "tui-traces"),

  // Per-test timeout (ms)
  timeout: 20_000,

  // Async expect timeout (ms) — give TUI renders time to settle
  expect: { timeout: 8_000 },
});
