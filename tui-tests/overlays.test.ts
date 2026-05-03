import { test, expect } from "@microsoft/tui-test";

test("Ctrl+P opens file finder overlay", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  terminal.keyPress("p", { ctrl: true });
  await expect(terminal.getByText("File Finder")).toBeVisible();
  terminal.keyPress("Escape");
});

test("Shift+P opens command palette overlay", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  terminal.keyPress("p", { shift: true });
  await expect(terminal.getByText("Command Palette")).toBeVisible();
  terminal.keyPress("Escape");
});

test("F1 opens help dialog", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  terminal.keyPress("F1");
  await expect(terminal.getByText("Help")).toBeVisible();
  terminal.keyPress("Escape");
});

test("Ctrl+O opens settings panel", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  terminal.keyPress("o", { ctrl: true });
  await expect(terminal.getByText("Settings")).toBeVisible();
  terminal.keyPress("Escape");
});

test("bookmarks overlay opens via Navigate menu", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  // Navigate menu → mnemonic 'k' for "Show Bookmarks"
  terminal.keyPress("n", { alt: true });
  terminal.write("k");
  await expect(terminal.getByText("Bookmarks")).toBeVisible();
  terminal.keyPress("Escape");
});
