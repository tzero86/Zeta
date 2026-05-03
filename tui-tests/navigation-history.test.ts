// Navigation history tests: Alt+Left (back) and Alt+Right (forward).
// Plain Left maps to NavigateToParent; history navigation requires Alt+Left/Right.
import { test, expect } from "@microsoft/tui-test";
import path from "path";

// ESC-prefixed arrow sequences: the standard terminal encoding for Alt+arrow.
const ALT_LEFT = "\x1b\x1b[D";
const ALT_RIGHT = "\x1b\x1b[C";

async function navigateToFixtures(terminal: any) {
  const fixturesPath = path.resolve(process.cwd(), "fixtures");
  terminal.keyPress("g", { alt: true });
  await expect(terminal.getByText("Go to Path")).toBeVisible();
  terminal.keyPress("u", { ctrl: true });
  terminal.write(fixturesPath);
  terminal.keyPress("Enter");
  await expect(terminal.getByText("README.md", { strict: false })).toBeVisible();
}

test("Alt+Left navigates back in directory history", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Navigate to fixtures dir (has README.md and documents/ subdirectory).
  await navigateToFixtures(terminal);

  // Move into documents/ (first entry after ../ in fixtures).
  terminal.keyDown(1);
  terminal.keyPress("Enter");
  await expect(terminal.getByText("note1.txt", { strict: false })).toBeVisible();

  // Alt+Left → NavigateBack → should return to fixtures showing README.md.
  terminal.write(ALT_LEFT);
  await expect(terminal.getByText("README.md", { strict: false })).toBeVisible();
});

test("Alt+Right navigates forward after going back", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Navigate to fixtures, then into documents/, then back.
  await navigateToFixtures(terminal);
  terminal.keyDown(1);
  terminal.keyPress("Enter");
  await expect(terminal.getByText("note1.txt", { strict: false })).toBeVisible();

  terminal.write(ALT_LEFT);
  await expect(terminal.getByText("README.md", { strict: false })).toBeVisible();

  // Alt+Right → NavigateForward → back to documents/, showing note1.txt again.
  terminal.write(ALT_RIGHT);
  await expect(terminal.getByText("note1.txt", { strict: false })).toBeVisible();
});

test("Alt+Left with no history does not crash", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // At startup history stack is empty; Alt+Left should be a no-op.
  terminal.write(ALT_LEFT);
  terminal.write(ALT_LEFT);

  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await expect(terminal.getByText("F5")).toBeVisible();
});
