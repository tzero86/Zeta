// Editor tests: open a file in the embedded editor, verify it renders,
// and check that basic editor keybindings are reachable.
import { test, expect } from "@microsoft/tui-test";
import path from "path";

// Navigate the active pane to the fixtures directory (contains known text
// files) using Alt+G (GoTo prompt), then select README.md via arrow key.
async function navigateToFixtures(terminal: any) {
  // process.cwd() is always the tui-tests/ directory (npm test runs from there)
  const fixturesPath = path.resolve(process.cwd(), "fixtures");
  // Alt+G opens the GoTo path prompt
  terminal.keyPress("g", { alt: true });
  await expect(terminal.getByText("Go to Path")).toBeVisible();
  // Clear pre-filled current directory, then type fixtures path
  terminal.keyPress("u", { ctrl: true });
  terminal.write(fixturesPath);
  terminal.keyPress("Enter");
  // Wait for the pane to show fixture files
  await expect(terminal.getByText("README.md", { strict: false })).toBeVisible();
  // Move cursor past ../, documents/, src/ to README.md
  terminal.keyDown(1);
  terminal.keyDown(1);
  terminal.keyDown(1);
}

test("F4 opens the editor on the selected entry", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await navigateToFixtures(terminal);

  // README.md should now be selected (first text file); open it
  terminal.keyPress("F4");

  await expect(terminal.getByText("Editor")).toBeVisible();
});

test("Esc closes the editor and returns to file manager", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await navigateToFixtures(terminal);

  terminal.keyPress("F4");
  await expect(terminal.getByText("Editor")).toBeVisible();

  terminal.keyPress("Escape");

  await expect(terminal.getByText("F5")).toBeVisible();
});

test("editor cheatsheet shows editing shortcuts", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await navigateToFixtures(terminal);

  terminal.keyPress("F4");
  await expect(terminal.getByText("Editor")).toBeVisible();

  terminal.keyPress("?");

  await expect(terminal.getByText("Quick Reference")).toBeVisible();
  await expect(terminal.getByText("Editing")).toBeVisible();
  await expect(terminal.getByText("save file")).toBeVisible();
});
