import { test, expect } from "@microsoft/tui-test";

test("sort cycles: pressing s changes pane title to name ↓", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  // Default sort is name ↑; one press goes to name ↓
  terminal.keyPress("s");
  await expect(terminal.getByText("name ↓")).toBeVisible();
});

test("sort cycles multiple: s again shows size ↑", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  terminal.keyPress("s"); // name ↓
  terminal.keyPress("s"); // size ↑
  await expect(terminal.getByText("size ↑")).toBeVisible();
});

test("pane filter: / opens inline filter bar with Esc clear hint", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  terminal.keyPress("/");
  await expect(terminal.getByText("Esc clear")).toBeVisible();
});

test("pane filter: typing text shows match count", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  terminal.keyPress("/");
  await expect(terminal.getByText("Esc clear")).toBeVisible();
  terminal.write("a");
  // Any directory listing will have entries matching 'a'; a count appears
  await expect(terminal.getByText("matches")).toBeVisible();
});

test("marks: Space marks a file and shows › * indicator", async ({
  terminal,
}) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  // Navigate past the ../ entry to a real file/dir then mark it
  terminal.keyDown(1);
  terminal.keyPress(" ");
  await expect(terminal.getByText("›*")).toBeVisible();
});
