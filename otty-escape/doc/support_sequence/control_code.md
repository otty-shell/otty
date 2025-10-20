# Support Control Codes (C0/C1)

Brief: this document lists the supported C0/C1 control codes that may occur
outside ESC sequences, based on the implementation in
`otty-escape/src/control.rs`. The tables show the abbreviation, hexadecimal
value, and the mnemonic name for each code.

## C0 (0x00–0x1F, 0x20 SPACE, 0x7F DEL)

| Abbrev | Hex  | Name (mnemonic)       |
|---|---|---|
| NUL | 0x00 | Null |
| SOH | 0x01 | Start of Heading |
| STX | 0x02 | Start of Text |
| ETX | 0x03 | End of Text |
| EOT | 0x04 | End of Transmission |
| ENQ | 0x05 | Enquiry |
| ACK | 0x06 | Acknowledge |
| BEL | 0x07 | Bell |
| BS  | 0x08 | Backspace |
| HT  | 0x09 | Horizontal Tab |
| LF  | 0x0A | Line Feed |
| VT  | 0x0B | Vertical Tab |
| FF  | 0x0C | Form Feed |
| CR  | 0x0D | Carriage Return |
| SO  | 0x0E | Shift Out |
| SI  | 0x0F | Shift In |
| DLE | 0x10 | Data Link Escape |
| DC1 | 0x11 | Device Control 1 (XON) |
| DC2 | 0x12 | Device Control 2 |
| DC3 | 0x13 | Device Control 3 (XOFF) |
| DC4 | 0x14 | Device Control 4 |
| NAK | 0x15 | Negative Acknowledge |
| SYN | 0x16 | Synchronous Idle |
| ETB | 0x17 | End of Transmission Block |
| CAN | 0x18 | Cancel |
| EM  | 0x19 | End of Medium |
| SUB | 0x1A | Substitute |
| FS  | 0x1C | File Separator |
| GS  | 0x1D | Group Separator |
| RS  | 0x1E | Record Separator |
| US  | 0x1F | Unit Separator |
| SPACE | 0x20 | Space |
| DEL | 0x7F | Delete |

## C1 (0x80–0x9F)

| Abbrev | Hex  | Name (mnemonic) |
|---|---|---|
| PAD | 0x80 | Padding Character |
| HOP | 0x81 | High Octet Preset |
| BPH | 0x82 | Break Permitted Here |
| NBH | 0x83 | No Break Here |
| IND | 0x84 | Index |
| NEL | 0x85 | Next Line |
| SSA | 0x86 | Start of Selected Area |
| ESA | 0x87 | End of Selected Area |
| HTS | 0x88 | Horizontal Tab Set |
| HTJ | 0x89 | Horizontal Tab with Justify |
| VTS | 0x8A | Vertical Tab Set |
| PLD | 0x8B | Partial Line Down |
| PLU | 0x8C | Partial Line Up |
| RI  | 0x8D | Reverse Index |
| SS2 | 0x8E | Single‑Shift 2 |
| SS3 | 0x8F | Single‑Shift 3 |
| DCS | 0x90 | Device Control String |
| PU1 | 0x91 | Private Use 1 |
| PU2 | 0x92 | Private Use 2 |
| STS | 0x93 | Set Transmitting State |
| CCH | 0x94 | Cancel Character |
| MW  | 0x95 | Message Waiting |
| SPA | 0x96 | Start of Protected Area |
| EPA | 0x97 | End of Protected Area |
| SOS | 0x98 | Start of String |
| SGCI | 0x99 | Single Graphic Char Introducer |
| SCI | 0x9A | Single Character Introducer |
| CSI | 0x9B | Control Sequence Introducer |
| ST  | 0x9C | String Terminator |
| OSC | 0x9D | Operating System Command |
| PM  | 0x9E | Privacy Message |
| APC | 0x9F | Application Program Command |

Notes:
- Some C0 codes have common caret forms (e.g., BEL = ^G) and C escapes
  (e.g., LF = \n). These are present in the source comments but omitted here
  for brevity. See `otty-escape/src/control.rs` for full details.
