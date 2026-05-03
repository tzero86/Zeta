import { test, expect } from "@microsoft/tui-test";

test("F3 opens preview pane", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  // Preview is off by default; F3 toggles it on
  terminal.keyPress("F3");
  await expect(terminal.getByText("select a file to preview")).toBeVisible();
});

test("F3 twice closes preview pane again", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  terminal.keyPress("F3"); // open
  await expect(terminal.getByText("select a file to preview")).toBeVisible();
  terminal.keyPress("F3"); // close
  await expect(
    terminal.getByText("select a file to preview")
  ).not.toBeVisible();
});




