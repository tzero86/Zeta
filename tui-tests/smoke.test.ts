// Smoke tests: verify Zeta starts, renders the dual-pane UI, and quits cleanly.
import { test, expect } from "@microsoft/tui-test";

test("zeta starts and renders the title bar", async ({ terminal }) => {
  // The title bar always shows "[Z]eta" in the menu bar area.
  await expect(terminal.getByText("[Z]eta")).toBeVisible();
});

test("zeta renders dual panes", async ({ terminal }) => {
  // Both pane header labels should be visible after startup.
  await expect(terminal.getByText("Left")).toBeVisible();
  await expect(terminal.getByText("Right")).toBeVisible();
});

test("zeta shows function-key bar at the bottom", async ({ terminal }) => {
  // The bottom bar always shows F-key hints (F1=Help, F5=Copy, etc.)
  await expect(terminal.getByText("F5")).toBeVisible();
  await expect(terminal.getByText("F8")).toBeVisible();
  await expect(terminal.getByText("F10")).toBeVisible();
});

test("zeta quits on Ctrl+Q", async ({ terminal }) => {
  // Wait for UI to load first
  await expect(terminal.getByText("[Z]eta")).toBeVisible();

  // Send Ctrl+Q — the standard quit binding
  terminal.keyPress("q", { ctrl: true });

  // After quit the alternate screen is dismissed — the title bar is gone
  await expect(terminal.getByText("[Z]eta")).not.toBeVisible();
});
