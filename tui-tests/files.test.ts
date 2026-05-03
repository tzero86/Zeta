import { test, expect } from "@microsoft/tui-test";
import path from "path";

// Helper: navigate the active pane to the fixtures directory.
// Uses Alt+G (goto), clears the pre-filled path, then types the path.
async function navigateToFixturesDir(terminal: any) {
  const fixturesPath = path.resolve(process.cwd(), "fixtures");
  terminal.keyPress("g", { alt: true });
  terminal.keyPress("u", { ctrl: true });
  terminal.write(fixturesPath);
  terminal.keyPress("Enter");
  // Move down past ../ to land on the first real entry
  terminal.keyDown(1);
}

test("F5 opens copy dialog", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await navigateToFixturesDir(terminal);
  terminal.keyPress("F5");
  await expect(terminal.getByText("Copy")).toBeVisible();
  terminal.keyPress("Escape");
});

test("F7 opens new directory dialog", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await navigateToFixturesDir(terminal);
  terminal.keyPress("F7");
  await expect(terminal.getByText("New Directory")).toBeVisible();
  terminal.keyPress("Escape");
});

test("F8 opens delete dialog", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await navigateToFixturesDir(terminal);
  terminal.keyPress("F8");
  await expect(terminal.getByText("Delete")).toBeVisible();
  terminal.keyPress("Escape");
});

test("r key opens rename dialog", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await navigateToFixturesDir(terminal);
  terminal.keyPress("r");
  await expect(terminal.getByText("Rename")).toBeVisible();
  terminal.keyPress("Escape");
});
