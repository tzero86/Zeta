// Cheatsheet tests: verify the context-aware ? overlay opens, shows the right
// section titles for the current focus layer, and closes on Esc.
import { test, expect } from "@microsoft/tui-test";

test("? key opens the cheatsheet overlay", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Send the ? key (Shift+/ on US keyboards, but tui-test accepts the character)
  terminal.keyPress("?");

  // The overlay title for the file manager context is "Quick Reference"
  await expect(terminal.getByText("Quick Reference")).toBeVisible();
});

test("cheatsheet shows Navigation section in file-manager context", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("?");

  await expect(terminal.getByText("Navigation")).toBeVisible();
  // Tab binding is always listed
  await expect(terminal.getByText("switch active pane")).toBeVisible();
});

test("cheatsheet shows File Operations section", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("?");

  await expect(terminal.getByText("File Operations")).toBeVisible();
  await expect(terminal.getByText("copy to other pane")).toBeVisible();
});

test("any action closes the cheatsheet overlay", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("?");
  await expect(terminal.getByText("Quick Reference")).toBeVisible();

  // Any key that produces an action (e.g. arrow key) dismisses the overlay.
  // Esc in pane context returns None so does not trigger a state change.
  terminal.keyDown(1);

  // Overlay should be gone; bottom bar should be visible again
  await expect(terminal.getByText("Quick Reference")).not.toBeVisible();
  await expect(terminal.getByText("F10")).toBeVisible();
});

test("? key is a toggle — second press closes the cheatsheet", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // First press: open
  terminal.keyPress("?");
  await expect(terminal.getByText("Quick Reference")).toBeVisible();

  // Second press: close
  terminal.keyPress("?");
  await expect(terminal.getByText("Quick Reference")).not.toBeVisible();
});
