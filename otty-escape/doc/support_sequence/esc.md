# Support ESC Sequences

This document lists ESC (0x1B) sequences supported by `otty-tokenizer`.

## No-Intermediate Sequences

| Sequence | Name |
|---|---|
| ESC D | IND – Index |
| ESC E | NEL – Next Line |
| ESC H | HTS – Horizontal Tab Set |
| ESC M | RI – Reverse Index |
| ESC N | SS2 – Single Shift 2 (next char only) |
| ESC O | SS3 – Single Shift 3 (next char only) |
| ESC V | SPA – Start of Protected Area |
| ESC W | EPA – End of Protected Area |
| ESC X | SOS – Start of String |
| ESC \ | ST – String Terminator |
| ESC ^ | PM – Privacy Message |
| ESC _ | APC – Application Program Command |
| ESC c | RIS – Full Reset |
| ESC F | Cursor Position Lower Left |
| ESC Z | DECID – Return Terminal ID |
| ESC 6 | DECBI – Back Index |
| ESC 7 | DECSC – Save Cursor |
| ESC 8 | DECRC – Restore Cursor |
| ESC = | DECPAM – Application Keypad |
| ESC > | DECPNM – Normal Keypad |

## Character Set Designation (SCS)

Designate G0 or G1 character set.

| Sequence | Name |
|---|---|
| ESC ( 0 | SCS G0 – DEC Line Drawing |
| ESC ( A | SCS G0 – UK |
| ESC ( B | SCS G0 – US ASCII |
| ESC ) 0 | SCS G1 – DEC Line Drawing |
| ESC ) A | SCS G1 – UK |
| ESC ) B | SCS G1 – US ASCII |

## DEC Line Attributes and Alignment

| Sequence | Name |
|---|---|
| ESC # 3 | DECDHL – Double-height line, top half |
| ESC # 4 | DECDHL – Double-height line, bottom half |
| ESC # 5 | DECSWL – Single-width line |
| ESC # 6 | DECDWL – Double-width line |
| ESC # 8 | DECALN – Screen alignment display |

## Application Mode Key Presses (SS3)

Sequences starting with `ESC O` in application mode.

| Sequence | Name |
|---|---|
| ESC O A | Arrow Up |
| ESC O B | Arrow Down |
| ESC O C | Arrow Right |
| ESC O D | Arrow Left |
| ESC O H | Home |
| ESC O F | End |
| ESC O P | F1 |
| ESC O Q | F2 |
| ESC O R | F3 |
| ESC O S | F4 |

Notes:
- For unmapped sequences, the parser yields `Unspecified { control, intermediates }`.
- Some behaviors (e.g., IND vs CursorUp on VT52/Windows consoles) depend on terminal.
