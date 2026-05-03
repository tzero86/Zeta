// Marks tests: marking files, clearing all marks, and mark count display.
import { test, expect } from "@microsoft/tui-test";

test("Space marks a file and › * indicator appears", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Navigate past ../ to a real entry
  terminal.keyDown(1);
  terminal.keyPress(" ");

  await expect(terminal.getByText("›*")).toBeVisible();
});

test("marking multiple files shows increasing mark count", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyDown(1);
  terminal.keyPress(" "); // mark first
  terminal.keyDown(1);
  terminal.keyPress(" "); // mark second

  // Status bar renders " ✦ 2 " when two items are marked.
  await expect(terminal.getByText("✦ 2", { strict: false })).toBeVisible();
});

test("Shift+M clears all marks", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Mark a file first
  terminal.keyDown(1);
  terminal.keyPress(" ");
  await expect(terminal.getByText("›*")).toBeVisible();

  // Shift+M maps to ClearMarks
  terminal.keyPress("M", { shift: true });

  // Mark indicator should be gone
  await expect(terminal.getByText("›*")).not.toBeVisible();
});

test("Shift+M on empty marks does not crash", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // No marks, just press Shift+M — should be a no-op
  terminal.keyPress("M", { shift: true });

  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await expect(terminal.getByText("F5")).toBeVisible();
});

test("unmarking a marked file removes the indicator", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyDown(1);
  terminal.keyPress(" "); // mark
  await expect(terminal.getByText("›*")).toBeVisible();

  terminal.keyPress(" "); // unmark same file
  await expect(terminal.getByText("›*")).not.toBeVisible();
});
