// Bookmark tests: adding, viewing, and removing bookmarks.
import { test, expect } from "@microsoft/tui-test";

test("Ctrl+B adds a bookmark for the current directory", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Ctrl+B triggers AddBookmark; status bar shows "bookmark added: <path>".
  terminal.keyPress("b", { ctrl: true });

  await expect(
    terminal.getByText("bookmark added", { strict: false })
  ).toBeVisible();
});

test("bookmarks overlay lists added bookmark", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Add a bookmark first; status confirms "bookmark added: <path>".
  terminal.keyPress("b", { ctrl: true });
  await expect(
    terminal.getByText("bookmark added", { strict: false })
  ).toBeVisible();

  // Open bookmarks list via Navigate menu → k
  terminal.keyPress("n", { alt: true });
  terminal.write("k");
  await expect(terminal.getByText("Bookmarks")).toBeVisible();

  // The saved path should appear in the modal listing.
  await expect(
    terminal.getByText("tui-tests", { strict: false })
  ).toBeVisible();

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

  // Empty state row rendered by render_bookmarks_modal when paths is empty.
  await expect(
    terminal.getByText("no bookmarks yet", { strict: false })
  ).toBeVisible();

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
