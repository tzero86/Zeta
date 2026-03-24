# Neo-Commander UI/UX Design

## Goal

Evolve Zeta from a functional commander-style TUI into a more premium, cohesive interface with a richer color hierarchy, clearer layer separation, and stronger visual branding.

## Design Principles

- Keep the keyboard-first workflow.
- Preserve file-manager efficiency and scan density.
- Add visual hierarchy, not decoration for its own sake.
- Make the app feel deliberate across panes, tools, modals, and the top bar.

## Visual Direction

### Palette Ladder

Use a small, intentional color ladder:

- brand accent: `[Z]`, active focus, key actions, selected palette rows
- surface 1: main panes and list areas
- surface 2: tools panel, preview, editor
- surface 3: dialogs, palette, prompts
- muted text: metadata, paths, secondary hints
- state colors: success, warning, destructive actions

The key shift is from "one highlight color" to "surface-aware color meaning."

### Shell Hierarchy

The interface keeps the current commander-inspired structure, but each layer should feel distinct:

- top bar: branded strip with `[Z]` accent and clear menu labels
- active pane: brighter surface, stronger border, obvious focus
- inactive pane: quieter surface and lower contrast
- tools panel: dedicated lower utility area for preview/editor/search/settings
- dialogs / palette / prompts: elevated overlay surfaces with stronger internal spacing
- status bar: operational telemetry, not decoration

### Interaction Polish

The app should feel precise in motion:

- focus states must be unmistakable
- hints and shortcuts should be easier to scan
- status feedback should be consistent and readable
- the tools panel should behave like a deliberate utility drawer

## Component Plan

### Top Bar

- show the `[Z]` accent in brand color
- keep menus compact
- use a slightly richer surface tone than the panes

### Panes

- active pane gets the clearest border and strongest text contrast
- inactive pane stays readable but softer
- metadata should use muted color, not the same tone as filenames

### Tools Panel

- preview, editor, search, and settings share the same lower region
- this region should read as a separate layer from the file browser
- panel should use a dedicated surface tone and border treatment

### Modals and Palette

- keep overlays centered and elevated
- use richer internal spacing than the current dialogs
- key hints should be highlighted consistently

## Implementation Phases

### Phase 1 — Palette and surfaces

- refine `ThemePalette`
- define the surface ladder and accent usage
- update top bar, panes, tools panel, and overlays to use it

### Phase 2 — Focus and hierarchy

- improve active/inactive pane contrast
- make the tools panel feel like a deliberate lower layer
- refine modal/palette borders and spacing

### Phase 3 — Micro-polish

- tighten hint colors
- improve status feedback
- tune icon/metadata contrast

## Success Criteria

- the app feels more cohesive at a glance
- the active pane is immediately obvious
- tools/modals are clearly separated from browsing
- the top bar reads like a branded control strip
- the UI still feels fast and keyboard-first
