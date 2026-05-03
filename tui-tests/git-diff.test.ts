// Git diff panel tests: Ctrl+D toggles the diff panel.
import { test, expect } from "@microsoft/tui-test";

test("Ctrl+D opens the git diff panel", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("d", { ctrl: true });

  // Git diff panel header shows "Changed Files"; allow extra time under parallel load
  await expect(
    terminal.getByText("Changed Files", { strict: false })
  ).toBeVisible({ timeout: 10000 });
});

test("Ctrl+D twice closes the git diff panel", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("d", { ctrl: true }); // open
  await expect(
    terminal.getByText("Changed Files", { strict: false })
  ).toBeVisible({ timeout: 10000 });

  terminal.keyPress("d", { ctrl: true }); // close
  await expect(
    terminal.getByText("Changed Files", { strict: false })
  ).not.toBeVisible();
});

test("git diff panel does not crash when repo has no staged changes", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("d", { ctrl: true });
  await expect(
    terminal.getByText("Changed Files", { strict: false })
  ).toBeVisible({ timeout: 10000 });

  // UI stays alive — file manager still renders
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  terminal.keyPress("d", { ctrl: true });
});
