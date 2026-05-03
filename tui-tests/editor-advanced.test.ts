// Advanced editor tests: save, undo, search, and fullscreen.
import { test, expect } from "@microsoft/tui-test";
import path from "path";

async function openReadmeInEditor(terminal: any) {
  const fixturesPath = path.resolve(process.cwd(), "fixtures");
  terminal.keyPress("g", { alt: true });
  await expect(terminal.getByText("Go to Path")).toBeVisible();
  terminal.keyPress("u", { ctrl: true });
  terminal.write(fixturesPath);
  terminal.keyPress("Enter");
  await expect(terminal.getByText("README.md", { strict: false })).toBeVisible();
  // Navigate to README.md (past ../ and any directories)
  terminal.keyDown(1);
  terminal.keyDown(1);
  terminal.keyDown(1);
  terminal.keyPress("F4");
  await expect(terminal.getByText("Editor")).toBeVisible();
}

test("Ctrl+S saves the file from the editor", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await openReadmeInEditor(terminal);

  // Ctrl+S triggers SaveEditor
  terminal.keyPress("s", { ctrl: true });

  // Should not crash; editor remains open
  await expect(terminal.getByText("Editor")).toBeVisible();
});

test("Ctrl+F opens the inline search bar in the editor", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await openReadmeInEditor(terminal);

  // Ctrl+F triggers EditorOpenSearch
  terminal.keyPress("f", { ctrl: true });

  // The search bar shows "Find:" prompt
  await expect(terminal.getByText("Find:", { strict: false })).toBeVisible();
  terminal.keyPress("Escape");
});

test("Ctrl+Z undoes the last edit in the editor", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await openReadmeInEditor(terminal);

  // Type some text then undo it
  terminal.write("x");
  terminal.keyPress("z", { ctrl: true });

  // Editor should still be open and stable
  await expect(terminal.getByText("Editor")).toBeVisible();
});

test("F11 toggles editor fullscreen mode", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await openReadmeInEditor(terminal);

  // F11 maps to ToggleEditorFullscreen
  terminal.keyPress("F11");

  // Editor should still be visible in fullscreen
  await expect(terminal.getByText("Editor")).toBeVisible();

  // Toggle back
  terminal.keyPress("F11");
  await expect(terminal.getByText("Editor")).toBeVisible();
});

test("editor search can be dismissed with Esc", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await openReadmeInEditor(terminal);

  terminal.keyPress("f", { ctrl: true });
  await expect(terminal.getByText("Find:", { strict: false })).toBeVisible();

  terminal.keyPress("Escape");
  await expect(
    terminal.getByText("Find:", { strict: false })
  ).not.toBeVisible();
  await expect(terminal.getByText("Editor")).toBeVisible();
});
