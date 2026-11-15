# UsageDock logo

**Idea:** the app's telemetry dial, made solid and split into two equal halves by a straight divide at 12 o'clock. It's drawn for the 16 px tray first, so every part lands on whole pixels.

Current files are in `final2/`. Earlier rounds (`final/`, `concepts*/`, `tray16/`) are kept for reference.

## Files (`final2/`)

| File | Use |
| --- | --- |
| `mark.svg` / `mark-white.svg` | The symbol (two dial halves), black or white, transparent background |
| `app-icon-small.svg` | The 16-unit pixel-grid tile, used for 16–32 px |
| `app-icon.svg` | The same design ×64 with the tile's subtle gradient and lit rim, for 40 px and up |
| `usagedock.ico` | Windows icon (16, 20, 24, 32, 40, 48, 64, 128, 256 px). Used for the app, tray and installer |
| `lockup-horizontal.svg` / `-white.svg` | Mark + "USAGEDOCK" wordmark |
| `wordmark.svg` / `-white.svg` | "USAGEDOCK" alone |
| `build_lockup.py` | Regenerates the lockups and wordmarks from the Barlow font |

## Construction (16-unit grid)

- Tile: 15 × 15, corner radius 3.5, centred.
- Dial: centre (8, 8.7), outer radius 5.6, inner radius 2.8; 270° sweep, open at the bottom (135° → 45°).
- Divide: a straight slot 2 units wide (x 7–9) at 12 o'clock, so both halves are equal and the gap is exactly 2 px at 16 px.
- Wordmark: Barlow SemiBold (SIL Open Font License), tracked +0.14 em, letters outlined.

## Colour

White `#FFFFFF` on black. The large tile adds a subtle `#1E1E22 → #000000` gradient and a lit top rim; the mark itself is always flat.

## Rules

- Minimum size: 16 px using `app-icon-small.svg`. Don't scale the large file down below 40 px.
- Keep the divide at 12 o'clock and the two halves equal.
- Don't add colour, gradients or shadows to the mark, rotate it, or close the dial's opening.

Trademark clearance hasn't been checked; do a professional search before any commercial use.
