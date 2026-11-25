# 40 Icons: the Projects plus, Containers, the error icon, unused icons

Status: done
Blocked by: 39

## Why

The owner's notes of 2026-11-17 and 2026-11-18:

- "every + is lit blue, Projects must be no exception": the "+" of the
  Projects header (issue 36) had the default tone; the "+" of Profiles and
  the New session split buttons are accent.
- The Containers mode icon (a box with three blocks on top) said nothing.
  Five drawings were shown; the owner chose the cube (B).
- The "!" inside the error icon looked eaten: the svg had no round line
  caps, so its dot, a line 0.5 long, drew as a 2 × 0.5 strip.
- Two icons were used nowhere: `sidebar-collapse.svg` and
  `sidebar-expand.svg` (left with the screen redesign #115). The owner
  said to remove them.

## What was done (coordinator)

- `view/projects_section.rs`: the "+" New session gets `IconTone::Accent`,
  as the "+" Add profile of the Profiles header.
- `assets/svg/mode-containers.svg`: a cube in the stroke style of the other
  mode icons (24 × 24, 1.7 wide, round caps and joins).
- `assets/svg/notice-error.svg`: round caps on the stem, the dot a filled
  circle of radius 1.25.
- `sidebar-collapse.svg` and `sidebar-expand.svg` removed.

No test was added: the tone is one line of view styling, and the kit's
icon button tests already check that an accent icon is drawn in accent.
The icons are pictures, checked by eye on a rendered sheet in both
themes and at the switcher size.

## Check by hand

1. Agents mode: the "+" of the Projects header is blue like the "+" of
   Profiles.
2. The mode switcher: Containers shows a cube.
3. An error notice or toast: the "!" has a round dot and a whole stem.
