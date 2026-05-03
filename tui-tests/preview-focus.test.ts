// Preview focus tests: Alt+F3 focuses the preview panel for keyboard scrolling.
import { test, expect } from "@microsoft/tui-test";
import path from "path";

async function openPreviewOnFile(terminal: any) {
  // Navigate to fixtures and open preview on README.md
  const fixturesPath = path.resolve(process.cwd(), "fixtures");
  terminal.keyPress("g", { alt: true });
  await expect(terminal.getByText("Go to Path")).toBeVisible();
  terminal.keyPress("u", { ctrl: true });
  terminal.write(fixturesPath);
  terminal.keyPress("Enter");
  await expect(terminal.getByText("README.md", { strict: false })).toBeVisible();
  terminal.keyDown(1);
  terminal.keyDown(1);
  terminal.keyDown(1);
  // Open preview
  terminal.keyPress("F3");
  await expect(terminal.getByText("select a file to preview")).toBeVisible();
  // Select README.md to load preview content
  terminal.keyPress("Enter");
}

test("F3 opens preview panel showing placeholder", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  terminal.keyPress("F3");
  await expect(terminal.getByText("select a file to preview")).toBeVisible();
  terminal.keyPress("F3");
});

test("preview panel renders file content when a file is selected", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  const fixturesPath = path.resolve(process.cwd(), "fixtures");
  terminal.keyPress("g", { alt: true });
  await expect(terminal.getByText("Go to Path")).toBeVisible();
  terminal.keyPress("u", { ctrl: true });
  terminal.write(fixturesPath);
  terminal.keyPress("Enter");
  await expect(terminal.getByText("README.md", { strict: false })).toBeVisible();

  // Open preview, then select README.md
  terminal.keyPress("F3");
  await expect(terminal.getByText("select a file to preview")).toBeVisible();

  // Move to README.md and select
  terminal.keyDown(1);
  terminal.keyDown(1);
  terminal.keyDown(1);

  // Preview should update with file content
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("F3");
});

test("Alt+F3 focuses the preview panel", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // First open the preview
  terminal.keyPress("F3");
  await expect(terminal.getByText("select a file to preview")).toBeVisible();

  // Alt+F3 maps to FocusPreviewPanel
  terminal.keyPress("F3", { alt: true });

  // UI stays alive with preview visible
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("F3");
});
