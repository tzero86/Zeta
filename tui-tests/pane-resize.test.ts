// Pane resize tests: + grows the left pane, _ shrinks it.
import { test, expect } from "@microsoft/tui-test";

test("+ key grows the left pane without crashing", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // + maps to GrowLeftPane
  terminal.write("+");

  // UI should still be alive and rendering
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
});

test("_ key shrinks the left pane without crashing", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // _ maps to ShrinkLeftPane
  terminal.write("_");

  await expect(terminal.getByText("[Z]eta")).toBeVisible();
});

test("grow then shrink returns to stable layout", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  terminal.write("+");
  terminal.write("+");
  terminal.write("_");
  terminal.write("_");

  // Dual-pane layout still visible after resize cycle
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
  await expect(terminal.getByText("F5")).toBeVisible();
});

test("multiple grows do not crash at boundary", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Grow 6 times from 50% → clamps at 80%.  State emits "pane split: 80%".
  for (let i = 0; i < 8; i++) {
    terminal.write("+");
  }

  await expect(
    terminal.getByText("pane split: 80%", { strict: false })
  ).toBeVisible();
});

test("multiple shrinks do not crash at boundary", async ({ terminal }) => {
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Shrink 6 times from 50% → clamps at 20%.  State emits "pane split: 20%".
  for (let i = 0; i < 8; i++) {
    terminal.write("_");
  }

  await expect(
    terminal.getByText("pane split: 20%", { strict: false })
  ).toBeVisible();
});
