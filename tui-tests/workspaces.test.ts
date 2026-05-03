import { test, expect } from "@microsoft/tui-test";

test("Alt+2 switches to workspace 2", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  terminal.keyPress("2", { alt: true });
  // Status bar shows "ws 2/4" when workspace 2 is active
  await expect(terminal.getByText("ws 2/4")).toBeVisible();
});

test("Alt+1 switches back to workspace 1", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  terminal.keyPress("2", { alt: true });
  await expect(terminal.getByText("ws 2/4")).toBeVisible();
  terminal.keyPress("1", { alt: true });
  await expect(terminal.getByText("ws 1/4")).toBeVisible();
});

test("Alt+3 switches to workspace 3", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  terminal.keyPress("3", { alt: true });
  await expect(terminal.getByText("ws 3/4")).toBeVisible();
});

