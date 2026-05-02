# Themes

Zeta ships with 9 built-in themes. Switch themes in the settings panel (`Ctrl+O`), via the [First-Run Wizard](First-Run-Wizard), or by editing `theme` in your `config.toml`.

Config value: `theme = "<name>"`

---

## Zeta *(default)*

**Config value:** `theme = "zeta"`

Zeta's own signature theme. High-contrast blues and teals on a dark background with crisp white text. Designed to be easy on the eyes during long sessions while keeping the UI visually distinct and unambiguous.

---

## Fjord

**Config value:** `theme = "fjord"`

Cool Nordic blues and greys inspired by Scandinavian landscapes. Deep navy backgrounds, icy blue accents, and muted grey file rows. A calm, professional look with strong readability.

---

## Sandbar

**Config value:** `theme = "sandbar"`

Warm sandy tones and earthy browns on a light beige background. The only light-background theme in the default set — great for bright environments or users who prefer light UIs.

---

## Oxide

**Config value:** `theme = "oxide"`

Burnt orange and rust accents on a near-black background. Warm and distinctive — inspired by metallic surfaces and oxidized copper. High contrast with a unique color character.

---

## Matrix

**Config value:** `theme = "matrix"`

Classic green-on-black terminal aesthetic. Phosphor green text and accents on pure black. Ideal for the retro hacker look or for blending with terminal color schemes in the same vein.

---

## Norton

**Config value:** `theme = "norton"`

A faithful homage to the original Norton Commander — cyan panels on a blue background with white and yellow text. If you grew up with NC on DOS, this will feel like coming home.

---

## Dracula

**Config value:** `theme = "dracula"`

An adaptation of the popular [Dracula](https://draculatheme.com/) color scheme. Purple, pink, and cyan accents on a dark grey background. Vibrant and colorful while remaining easy to read.

---

## Neon

**Config value:** `theme = "neon"`

Bold neon greens, magentas, and cyans on black. High intensity — designed to pop on OLED screens or for users who want maximum visual energy. Takes some getting used to, but unforgettable once you do.

---

## Monochrome

**Config value:** `theme = "monochrome"`

Pure greyscale. White text, grey accents, and black background with no color. Ideal for accessibility needs, screenshots, or environments where color is a distraction.

---

## Switching Themes

**At runtime** — no restart needed:

1. Press `Ctrl+O` to open the settings panel
2. Arrow through the theme list
3. The UI updates live as you move

**In config** (also live-reloaded):

```toml
theme = "dracula"
```

**During first launch:**

The [First-Run Wizard](First-Run-Wizard) lets you preview all themes before Zeta writes your config.

---

*See also: [First-Run Wizard](First-Run-Wizard) · [Configuration](Configuration)*
