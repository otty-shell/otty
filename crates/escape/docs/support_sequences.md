# Supported Terminal Sequences

`otty-escape` implements a ANSI parser that expands incoming byte streams
into high-level actions. This document enumerates the control, escape, CSI, and
OSC sequences that currently produce side effects. Sequence names follow the
DEC/xterm conventions.

## Notation

- `ESC` represents the escape byte (`0x1B`).
- `CSI` (`ESC [`) introduces control sequence introducer commands.
- `OSC` (`ESC ]`) introduces operating system control strings. Each OSC listed
  here may be terminated by either `BEL` or the `ST` (`ESC \`) string
  terminator.
- `ST` denotes the string terminator (`ESC \`).
- `Ps`, `Pm`, `Pn` follow the standard parameter placeholders. When omitted,
  the implementation uses the usual VT defaults (commonly `Ps = 1`).

## C0/C1 Control Codes

| Byte | Mnemonic | Effect |
| ---- | -------- | ------ |
| `0x07` | `BEL` | Ring the terminal bell. |
| `0x08` | `BS` | Backspace one character cell. |
| `0x09` | `HT` | Advance to the next tab stop (`Action::InsertTabs(1)`). |
| `0x0A` | `LF` | Line feed (moves down, scrolls if needed). |
| `0x0B` | `VT` | Treated as line feed. |
| `0x0C` | `FF` | Treated as line feed. |
| `0x0D` | `CR` | Carriage return (move to column 0). |
| `0x0E` | `SO` | Select charset `G1` for subsequent bytes. |
| `0x0F` | `SI` | Select charset `G0` for subsequent bytes. |
| `0x1A` | `SUB` | Emit the substitution action (`Action::Substitute`). |
| `0x84` | `IND` | Index (line feed). |
| `0x85` | `NEL` | Next line (line feed plus carriage return). |
| `0x88` | `HTS` | Set a horizontal tab stop at the cursor column. |

All other control bytes are ignored (only logged for diagnostics).

## ESC Sequences

| Sequence | Name | Effect |
| -------- | ---- | ------ |
| `ESC D` | `IND` | Index (line feed). |
| `ESC E` | `NEL` | Next line (line feed with carriage return). |
| `ESC H` | `HTS` | Set a tab stop at the current column. |
| `ESC M` | `RI` | Reverse index (move cursor up, scroll if required). |
| `ESC c` | `RIS` | Full reset of parser state. |
| `ESC Z` | `DECID` | Identify terminal (reports default identity). |
| `ESC 7` | `DECSC` | Save cursor position and rendition state. |
| `ESC 8` | `DECRC` | Restore the saved cursor state. |
| `ESC =` | `DECPAM` | Enter application keypad mode. |
| `ESC >` | `DECPNM` | Enter numeric keypad mode. |
| `ESC ( 0`, `ESC ) 0`, `ESC * 0`, `ESC + 0` | `SCS` | Designate DEC line-drawing charset for `G0`/`G1`/`G2`/`G3`. |
| `ESC ( B`, `ESC ) B`, `ESC * B`, `ESC + B` | `SCS` | Designate US-ASCII charset for `G0`/`G1`/`G2`/`G3`. |
| `ESC # 8` | `DECALN` | Trigger the screen alignment test (fill screen with `E`). |
| `ESC \` | `ST` | Recognised string terminator (no standalone action). |

Only the ASCII and DEC line-drawing charsets are currently supported.

## CSI Sequences

### Mode and Synchronisation Control

| Sequence | Effect | Notes |
| -------- | ------ | ----- |
| `CSI Pm h` (`SM`) | Enable ANSI/VT modes. | Recognised public modes: `4` (IRM / insert mode), `20` (LNM / newline). Unrecognised codes are surfaced as `Unknown`. |
| `CSI Pm l` (`RM`) | Disable ANSI/VT modes. | Same mode set as above. |
| `CSI ? Pm h` (`DECSET`) | Enable DEC private modes. | Supported private modes: `1` (cursor keys), `3` (132-column), `6` (origin), `7` (autowrap), `12` (blinking cursor), `25` (show cursor), `1000/1002/1003` (mouse tracking variants), `1004` (focus events), `1005` (UTF-8 mouse), `1006` (SGR mouse), `1007` (alternate scroll), `1042` (urgency hints), `1049` (alt screen), `2004` (bracketed paste), `2026` (synchronized updates). Enabling `?2026` also calls `begin_sync`. Unknown codes are forwarded as `Unknown`. |
| `CSI ? Pm l` (`DECRST`) | Disable DEC private modes. | Disabling `?2026` triggers `end_sync`. |
| `CSI ! p` (`DECSTR`) | Soft terminal reset. | Ends any active synchronized update batch. |
| `CSI $ p` | Report ANSI mode 0. | Emits `ReportMode(Mode::Unknown(0))`. |
| `CSI Ps $ p` (`DECRQM`) | Report ANSI mode `Ps`. | `Ps` values `4` and `20` map to the known named modes. |
| `CSI ? $ p` | Report DEC private mode 0. | Reports `PrivateMode::Unknown(0)`. |
| `CSI ? Ps $ p` | Report DEC private mode `Ps`. | Named modes follow the list above. |

### ModifyOtherKeys and Kitty Keyboard Protocol

| Sequence | Effect | Notes |
| -------- | ------ | ----- |
| `CSI ? 4 m` | Report the current `modifyOtherKeys` state. | Emits `ReportModifyOtherKeysState`. |
| `CSI > 4 ; Ps m` | Set `modifyOtherKeys` state. | `Ps = 0` resets, `1` enables except well-defined keys, `2` enables for all keys. Extra parameters are ignored. |
| `CSI = Ps ; Pb u` | Configure Kitty keyboard protocol. | `Ps` is a bitmask over `DISAMBIGUATE_ESC_CODES (1)`, `REPORT_EVENT_TYPES (2)`, `REPORT_ALTERNATE_KEYS (4)`, `REPORT_ALL_KEYS_AS_ESC (8)`, `REPORT_ASSOCIATED_TEXT (16)`. `Pb = 0/1` replaces, `Pb = 2` unions, `Pb = 3` subtracts from the current mask. |
| `CSI > Ps u` | Push a Kitty keyboard mode mask onto the stack. | `Ps` uses the same bit definitions. |
| `CSI < Ps u` | Pop Kitty keyboard mode frames. | Pops `Ps` frames; values `< 1` default to `1`. |
| `CSI ? u` | Report the active Kitty keyboard mode. | Emits `ReportKeyboardMode`. |

### Cursor Shape and State

| Sequence | Effect | Notes |
| -------- | ------ | ----- |
| `CSI Ps SP q` (`DECSCUSR`) | Set cursor style. | Values `0/1/2` → block, `3/4` → underline, `5/6` → beam; odd numbers request a blinking variant. Omitted `Ps` defaults to `0`. |
| `CSI s` (`SCOSC`) | Save cursor position/state. | Saves cursor position plus rendition state. |
| `CSI u` (`SCORC`) | Restore cursor position/state. | With parameters, the sequence is delegated to keyboard handling (`CSI =`, `CSI <`, `CSI >`, `CSI ?`). |

### Cursor Motion and Positioning

| Sequence | Effect | Notes |
| -------- | ------ | ----- |
| `CSI Ps A` (`CUU`) | Move cursor up `Ps` rows. | Default `Ps = 1`; no carriage return. |
| `CSI Ps B` (`CUD`) | Move cursor down `Ps` rows. | Default `Ps = 1`. |
| `CSI Ps e` (`VPR`) | Vertical position relative (down). | Alias of `CUD`. |
| `CSI Ps C` (`CUF`) | Move cursor right `Ps` columns. | Default `Ps = 1`. |
| `CSI Ps a` (`HPR`) | Horizontal position relative. | Alias of `CUF`. |
| `CSI Ps D` (`CUB`) | Move cursor left `Ps` columns. | Default `Ps = 1`. |
| `CSI Ps E` (`CNL`) | Move to next line `Ps` times with carriage return. | Default `Ps = 1`. |
| `CSI Ps F` (`CPL`) | Move to previous line `Ps` times with carriage return. | Default `Ps = 1`. |
| `CSI Ps G` (`CHA`) | Move to column `Ps` (1-based). | Defaults to column 1. |
| `CSI Ps \`` (`HPA`) | Horizontal position absolute. | Same semantics as `CHA`. |
| `CSI Ps d` (`VPA`) | Move to row `Ps` (1-based). | Defaults to row 1. |
| `CSI Ps ; Ps H` (`CUP`) | Move to row/column. | Missing parameters default to `1;1`. |
| `CSI Ps ; Ps f` (`HVP`) | Horizontal/vertical position. | Alias of `CUP`. |
| `CSI Ps I` (`CHT`) | Move forward `Ps` tab stops. | Default `Ps = 1`. |
| `CSI Ps Z` (`CBT`) | Move backward `Ps` tab stops. | Default `Ps = 1`. |
| `CSI Ps b` (`REP`) | Repeat the preceding printable character `Ps` times. | No effect if no preceding character is cached. |

### Tab Stop Control

| Sequence | Effect | Notes |
| -------- | ------ | ----- |
| `CSI Ps g` (`TBC`) | Clear tab stops. | `Ps = 0` (or omitted) clears the stop under the cursor; `Ps = 3` clears all stops. |
| `CSI ? 5 W` (`DECST8C`) | Restore 8-column tab stops. | Calls `Action::SetTabs(8)`. |

### Insertion, Deletion, and Erasure

| Sequence | Effect | Notes |
| -------- | ------ | ----- |
| `CSI Ps @` (`ICH`) | Insert `Ps` blank characters. | Default `Ps = 1`. |
| `CSI Ps P` (`DCH`) | Delete `Ps` characters. | Default `Ps = 1`. |
| `CSI Ps X` (`ECH`) | Erase `Ps` characters with blanks. | Default `Ps = 1`. |
| `CSI Ps L` (`IL`) | Insert `Ps` blank lines. | Default `Ps = 1`. |
| `CSI Ps M` (`DL`) | Delete `Ps` lines. | Default `Ps = 1`. |
| `CSI Ps J` (`ED`) | Erase display. | `Ps = 0` (below), `1` (above), `2` (entire screen), `3` (scrollback/buffer). |
| `CSI Ps K` (`EL`) | Erase line. | `Ps = 0` (to right), `1` (to left), `2` (entire line). |

### Scrolling Region and Viewport

| Sequence | Effect | Notes |
| -------- | ------ | ----- |
| `CSI Ps ; Ps r` (`DECSTBM`) | Set scrolling region. | Parameters are top and bottom margins (1-based). |
| `CSI Ps S` (`SU`) | Scroll up `Ps` lines within the region. | Default `Ps = 1`. |
| `CSI Ps T` (`SD`) | Scroll down `Ps` lines within the region. | Default `Ps = 1`. |

### Reports and Window Operations

| Sequence | Effect | Notes |
| -------- | ------ | ----- |
| `CSI Ps n` (`DSR`) | Report device status / cursor position. | Emits `ReportDeviceStatus(Ps)`. |
| `CSI Ps c` (`DA`) | Identify terminal. | No parameter → default ID; `'>'` param selects alternate response; numeric parameters are passed through as Unicode code points. |
| `CSI Ps t` (`XTWINOPS`) | Window operations. | `Ps = 14` request text area size (pixels), `18` request size in columns/rows, `22` push window title, `23` pop window title. |

### Select Graphic Rendition (SGR)

`CSI Pm m` drives the standard text attributes. Multiple parameters may be
supplied in one sequence.

- Reset and style toggles: `0` (reset), `1` (bold), `2` (dim), `3` (italic),
  `4` (underline), `4:2` (double underline), `4:3` (undercurl), `4:4`
  (dotted underline), `4:5` (dashed underline), `5` (blink slow), `6` (blink
  fast), `7` (reverse), `8` (hidden), `9` (strikeout).
- Cancelling attributes: `21` (cancel bold), `22` (cancel bold/dim), `23`
  (cancel italic), `24` (cancel underline), `25` (cancel blink), `27` (cancel
  reverse), `28` (cancel hidden), `29` (cancel strikeout).
- Standard foreground colours: `30–37`, reset with `39`.
- Standard background colours: `40–47`, reset with `49`.
- Bright foreground colours: `90–97`.
- Bright background colours: `100–107`.
- Extended colours: `38;5;index` / `48;5;index` for 256-colour palette, or
  `38;2;r;g;b` / `48;2;r;g;b` for truecolour values.
- Underline colour: `58;5;index` / `58;2;r;g;b` set the underline colour,
  `59` clears it.

Colour specifications accept the standard xterm formats handled by `xparse_color`
(`#rgb`, `#rrggbb`, `rgb:r/g/b`) as well as `#RRGGBB` / `0xRRGGBB` literals when
used in OSC 10/11/12 (see below).

### Hyperlink and Tab State Synchronisation

The parser saves/restores hyperlink state through `OSC 8`, and the `Save Cursor`
and `Restore Cursor` sequences capture charset and mode state alongside the
position.

## OSC Sequences

| Sequence | Effect | Notes |
| -------- | ------ | ----- |
| `OSC 0 ; text ST` | Set the icon/window title. | Leading/trailing whitespace is trimmed; multiple `;`-separated chunks are joined with literal semicolons. |
| `OSC 2 ; text ST` | Set the window title (alias of `OSC 0`). | |
| `OSC 4 ; index ; spec [; index ; spec ...] ST` | Set indexed palette colours. | `spec` accepts `#rgb`, `#rrggbb`, `rgb:r/g/b`. `spec = ?` requests the current value. |
| `OSC 8 ; params ; uri ST` | Set/reset hyperlinks. | `params` may contain `id=...` pairs. Supplying an empty `uri` clears the active hyperlink. |
| `OSC 10 ; spec ST` | Set the default foreground colour. | `spec` accepts the xterm colour formats or `#RRGGBB` / `0xRRGGBB`. `spec = ?` queries the current value. |
| `OSC 11 ; spec ST` | Set the default background colour. | Same formats and query support as OSC 10. |
| `OSC 12 ; spec ST` | Set the default cursor colour. | Same formats and query support as OSC 10. |
| `OSC 22 ; cursor ST` | Set the mouse cursor icon. | `cursor` must match a `cursor_icon::CursorIcon` name (CSS cursor keywords). Unknown names are ignored. |
| `OSC 50 ; CursorShape=Ps ST` | Set the text cursor shape. | `Ps = 0` block, `1` beam, `2` underline. Invalid values are ignored. |
| `OSC 104 ST` | Reset all 0–255 palette entries to defaults. | |
| `OSC 104 ; index ... ST` | Reset specific palette entries. | Each `index` is reset individually. |
| `OSC 110 ST` | Reset default foreground colour. | |
| `OSC 111 ST` | Reset default background colour. | |
| `OSC 112 ST` | Reset default cursor colour. | |

Sequences not listed above are parsed but result only in diagnostic logging.
