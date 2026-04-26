# Site Redesign Design Spec
**Date:** 2026-04-26  
**File:** `site/index.html` (GitHub Pages, served from `./site` via `peaceiris/actions-gh-pages@v4`)

---

## Goal

Refresh the Zeta GitHub Pages site to reflect the app's current state — git diff viewer, flyout View→Themes menu, Norton Commander theme — and present the project with a more polished, "Modern Dev Tool" aesthetic while keeping it a single-file, dependency-free static page.

---

## Visual Direction

**Palette — Dual Accent:**
- Background: `#0d1117` (GitHub dark)
- Surface: `#161b22` (cards, code blocks)
- Border: `#30363d`
- Foreground: `#e6edf3`
- Muted text: `#7d8590`
- **Accent 1 — Periwinkle** `#82aaff`: headings, labels, section titles, logo `[Z]` glyph, icon borders
- **Accent 2 — Sage green** `#c3e88d`: CTAs, install command highlight, "What's New" badge, progress/status indicators
- Top accent bar: gradient sweep `transparent → #82aaff → #c3e88d → transparent`

**Typography:**
- Keep monospace-only stack: `'SF Mono', 'Fira Code', 'Cascadia Code', 'JetBrains Mono', monospace`
- Headings: heavier weight (800), tighter tracking
- Body / muted labels: `#7d8590`, smaller size

---

## Layout Sections (top → bottom)

### 1. Accent Bar
A single `2px` full-width gradient line at the very top of the page. Colors: periwinkle → sage green sweep. Signals the dual-accent brand immediately.

### 2. Hero — Centered + Terminal Chrome
- **Layout:** Centered single column.
- **Headline:** `[Z]eta` — the `Z` in periwinkle, the cursor block `█` in sage green.
- **Sub-label:** `// terminal file manager` in periwinkle, small, all-lowercase monospace, above the headline.
- **Tagline:** `Keyboard-first · Dual pane · Git-aware · Rust` in muted text below headline.
- **Install command:** code block with a sage-green left border:
  ```
  $ cargo install --git https://github.com/tzero86/Zeta
  ```
- **Badges row:** `Rust` · `MIT` · `< 10 MB` — small pill badges with dim borders.
- **Terminal chrome frame:** A fake terminal window below the badges.
  - Title bar: dark surface (`#161b22`), three traffic-light dots (red `#ff5f57`, yellow `#febc2e`, green `#28c840`), path text `zeta — ~/projects` in muted.
  - Body: actual `screenshot.png` (the existing file at repo root) displayed inside, full width.
  - The frame has a subtle periwinkle border `1px solid rgba(130,170,255,0.25)` and `4px` border-radius.

### 3. What's New
- **Section label:** `// what's new` in periwinkle
- **Layout:** 3-card horizontal row (wraps on mobile)
- **Cards:** each has a sage-green `NEW` badge top-right, an icon/emoji, title, and 2–3 sentence description.
  1. **Git Diff Viewer** — side-by-side colored diff with line numbers, gutter, and full keyboard/mouse scroll.
  2. **View → Themes Flyout** — nested flyout submenu under View for live theme switching without leaving the keyboard.
  3. **Norton Commander Theme** — authentic CGA/DOS palette (#0000AA, #55FFFF, #FFFF55) for the classic feel.
- Cards use `#161b22` surface, `#30363d` border, periwinkle accent on hover.

### 4. Features Grid
Keep existing 11 cards. Update copy for the git diff viewer card (already present). Add a **Flyout Menus** card (12th) if not already present.  
Grid: `repeat(auto-fill, minmax(280px, 1fr))`.

### 5. Keyboard Shortcuts
Keep existing section. No content changes needed.

### 6. Origin Story
Keep existing narrative. No content changes needed.

### 7. Footer
Keep existing footer structure. Update copyright year to 2026 if needed.

---

## CSS Architecture

All styles inline in `<style>` inside `site/index.html` — no external stylesheets or JS frameworks.

New CSS variables to add/change:
```css
--accent: #82aaff;          /* periwinkle — primary accent */
--accent2: #c3e88d;         /* sage green — CTAs, highlights */
--accent-bar: linear-gradient(90deg, transparent, #82aaff 30%, #c3e88d 70%, transparent);
```

Replace existing `--accent: #3fb950` references: structural/heading accents → `var(--accent)`, CTA/install/badge accents → `var(--accent2)`.

---

## Content Additions

- What's New section: 3 new feature blurbs (git diff viewer, flyout themes, norton theme)
- Feature card 12: "Flyout Menus" if not present
- Hero sub-label copy: `// terminal file manager`
- Badges: `Rust` · `MIT` · `< 10 MB` (keep existing)
- Screenshot: use existing `app.png` (already in `site/`) inside the terminal chrome frame

---

## Out of Scope

- No JavaScript beyond the existing minimal scroll behavior
- No external fonts or CDN dependencies
- No animation / transitions beyond CSS hover states
- No changes to `root/index.html` (the non-deployed version)
- No changes to any Rust source files

---

## Success Criteria

1. `site/index.html` renders correctly in a modern browser with the new dual-accent palette.
2. The terminal chrome frame shows `screenshot.png` correctly.
3. The What's New section appears between the hero and the features grid.
4. All existing content (features, shortcuts, origin story, footer) is preserved.
5. No external dependencies introduced (still a single-file static page).
6. Passes a quick mobile responsiveness check (cards wrap, hero stays readable at 375px wide).
