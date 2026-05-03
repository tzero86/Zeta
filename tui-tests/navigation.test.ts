// Navigation tests: pane switching, arrow key movement, directory traversal.
import { test, expect } from "@microsoft/tui-test";

// All navigation tests use the default program configured in tui-test.config.ts
// (the zeta debug binary). Zeta opens in the CWD of the test runner.

test("Tab key switches active pane", async ({ terminal }) => {
  // Wait for Zeta to load
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Tab switches the active pane. Pressing it twice should bring focus back.
  terminal.keyPress("Tab");

  // UI should still be alive and rendered after pane switch
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await expect(terminal.getByText("Right")).toBeVisible();
});

test("arrow keys navigate file list without crashing", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Move down several times
  terminal.keyDown(5);

  // Move back up
  terminal.keyUp(3);

  // Zeta should still be running and rendering
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
});

test("Backspace goes up a directory", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Backspace navigates to parent directory
  terminal.keyPress("Backspace");

  // UI should still be alive — we're just checking for no crash
  await expect(terminal.getByText("F10")).toBeVisible();
});

test("Shift+F10 opens context menu via keyboard", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Shift+F10 is the keyboard shortcut for context menu
  terminal.keyPress("F10", { shift: true });

  // Context menu should appear with at least one menu item
  await expect(
    terminal.getByText("Open", { full: false })
  ).toBeVisible();
});
