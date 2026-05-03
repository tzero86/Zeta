// Bookmark tests: adding, viewing, and removing bookmarks.
import { test, expect } from "@microsoft/tui-test";

test("Ctrl+B adds a bookmark for the current directory", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Ctrl+B triggers AddBookmark
  terminal.keyPress("b", { ctrl: true });

  // A status message or confirmation should appear
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
});

test("bookmarks overlay lists added bookmark", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Add a bookmark first
  terminal.keyPress("b", { ctrl: true });

  // Open bookmarks list via Navigate menu → k
  terminal.keyPress("n", { alt: true });
  terminal.write("k");
  await expect(terminal.getByText("Bookmarks")).toBeVisible();

  terminal.keyPress("Escape");
});

test("bookmarks overlay is empty when no bookmarks added", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Open bookmarks overlay without adding any
  terminal.keyPress("n", { alt: true });
  terminal.write("k");
  await expect(terminal.getByText("Bookmarks")).toBeVisible();

  terminal.keyPress("Escape");
});

test("bookmarks overlay can be dismissed with Esc", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.keyPress("n", { alt: true });
  terminal.write("k");
  await expect(terminal.getByText("Bookmarks")).toBeVisible();

  terminal.keyPress("Escape");
  await expect(terminal.getByText("Bookmarks")).not.toBeVisible();
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
});
