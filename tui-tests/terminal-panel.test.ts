// Terminal panel tests: F2 toggle, fullscreen, and basic interaction.
import { test, expect } from "@microsoft/tui-test";

test("F2 opens the embedded terminal panel", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("F2");

  // Terminal panel shows a " Shell " badge when open
  await expect(terminal.getByText("Shell", { strict: false })).toBeVisible();
});

test("F2 twice closes the terminal panel", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("F2"); // open
  await expect(terminal.getByText("Shell", { strict: false })).toBeVisible();

  terminal.keyPress("F2"); // close
  await expect(
    terminal.getByText("Shell", { strict: false })
  ).not.toBeVisible();
});

test("terminal panel renders waiting message on first open", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("F2");
  await expect(terminal.getByText("Shell", { strict: false })).toBeVisible();

  // Before any shell output, the panel shows a waiting indicator
  await expect(
    terminal.getByText("Waiting", { strict: false })
  ).toBeVisible();

  terminal.keyPress("F2");
});

test("terminal panel is distinct from file manager panes", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("F2");
  await expect(terminal.getByText("Shell", { strict: false })).toBeVisible();

  // File manager UI still renders alongside terminal
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("F2");
});
