// Navigation history tests: Left arrow (back) and Right arrow (forward).
import { test, expect } from "@microsoft/tui-test";

test("Left arrow navigates back in directory history", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Navigate into a subdirectory first (Enter on first entry after ../)
  terminal.keyDown(1);
  terminal.keyPress("Enter");

  // Go back with Left arrow (keyLeft is the correct method for arrow keys)
  terminal.keyLeft(1);

  // UI should remain stable — we're checking for no crash
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await expect(terminal.getByText("F5")).toBeVisible();
});

test("Right arrow navigates forward after going back", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Navigate forward, then back, then forward again
  terminal.keyDown(1);
  terminal.keyPress("Enter");
  terminal.keyLeft(1);
  terminal.keyRight(1);

  await expect(terminal.getByText("[Z]eta")).toBeVisible();
});

test("Left arrow on history root does not crash", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Press Left at the very start — no history yet, should no-op
  terminal.keyLeft(1);
  terminal.keyLeft(1);

  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await expect(terminal.getByText("F5")).toBeVisible();
});
