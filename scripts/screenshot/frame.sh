#!/usr/bin/env bash
# Put a window screenshot on a brand background for the README: rounded
# corners, a soft shadow and a dark gradient made from the logo colors.
#
# Usage:
#   scripts/screenshot/frame.sh <screenshot.png> [output.png]
#
# The output defaults to assets/otty.png. Sizes are for a screenshot taken
# at 2x scale; set PAD and RADIUS for other scales.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INPUT="${1:?Usage: frame.sh <screenshot.png> [output.png]}"
OUTPUT="${2:-$REPO/assets/otty.png}"
PAD="${PAD:-96}"
RADIUS="${RADIUS:-24}"
# The logo magenta and cyan, darkened so the window stays the brightest
# thing in the picture.
FROM_COLOR="#5a0d40"
TO_COLOR="#08504c"

read -r width height < <(magick identify -format '%w %h\n' "$INPUT")
canvas="$((width + 2 * PAD))x$((height + 2 * PAD))"
r="$RADIUS"

nice -n 10 magick \
  -size "$canvas" -define gradient:angle=135 "gradient:$FROM_COLOR-$TO_COLOR" \
  \( "$INPUT" -alpha set \
     \( +clone -alpha extract \
        -draw "fill black polygon 0,0 0,$r $r,0 fill white circle $r,$r $r,0" \
        \( +clone -flip \) -compose Multiply -composite \
        \( +clone -flop \) -compose Multiply -composite \) \
     -alpha off -compose CopyOpacity -composite \
     \( +clone -background black -shadow 55x36+0+24 \) +swap \
     -background none -compose Over -layers merge +repage \) \
  -gravity center -compose Over -composite \
  -strip "$OUTPUT"

echo "Wrote $OUTPUT"
