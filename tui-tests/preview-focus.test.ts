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

  // Open preview panel first, then navigate to README.md.
  terminal.keyPress("F3");
  await expect(terminal.getByText("select a file to preview")).toBeVisible();

  // Navigate to README.md: past ../, documents/, src/ → keyDown(3).
  terminal.keyDown(1);
  terminal.keyDown(1);
  terminal.keyDown(1);

  // Preview updates as cursor moves; fixtures/README.md contains "Test Fixture".
  await expect(
    terminal.getByText("Test Fixture", { strict: false })
  ).toBeVisible({ timeout: 5000 });

  terminal.keyPress("F3");
});

test("Alt+F3 focuses the preview panel", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Open preview and navigate to a file so the panel has content to focus.
  const fixturesPath = path.resolve(process.cwd(), "fixtures");
  terminal.keyPress("g", { alt: true });
  await expect(terminal.getByText("Go to Path")).toBeVisible();
  terminal.keyPress("u", { ctrl: true });
  terminal.write(fixturesPath);
  terminal.keyPress("Enter");
  await expect(terminal.getByText("README.md", { strict: false })).toBeVisible();

  terminal.keyPress("F3");
  await expect(terminal.getByText("select a file to preview")).toBeVisible();
  terminal.keyDown(1);
  terminal.keyDown(1);
  terminal.keyDown(1);
  await expect(
    terminal.getByText("Test Fixture", { strict: false })
  ).toBeVisible({ timeout: 5000 });

  // Alt+P is the testable secondary binding for FocusPreviewPanel (Alt+F3 sends a CSI
  // cursor-position sequence that crossterm 0.28 cannot distinguish from F3+Alt).
  terminal.keyPress("p", { alt: true });
  await expect(
    terminal.getByText("preview panel focused", { strict: false })
  ).toBeVisible();

  terminal.keyPress("F3");
});
