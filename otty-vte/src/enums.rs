/// The [`State`] enum captures the current position in the parser's control flow.
/// It mirrors the state machine defined by DEC/ECMA-48 terminals where input
/// bytes drive transitions between high level modes like ground text handling,
/// escape sequences, control sequence introducer (CSI) parsing, device control
/// strings (DCS) and operating system commands (OSC).
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    #[default]
    /// The ground state is the parser’s default, steady-state mode.
    /// In this state, *by default* printable characters and graphic codes
    /// are emitted (as [`Action::Print`]), and many C0 control codes are executed
    /// immediately (as [`Action::Execute`]), while certain lead bytes may trigger
    /// transitions into higher-level sequence parsers (ESC, CSI, DCS, OSC, etc.)
    /// depending on parser/terminal mode.
    ///
    /// Think of this as the “text flow” path: bytes that do not start a valid
    /// structured control sequence are handled here (either printed or
    /// executed immediately, depending on their class).
    ///
    /// ## What the happens in [`State::Ground`]
    /// | Class                       | Byte range            | Action / Transition                          |
    /// |-----------------------------|-----------------------|----------------------------------------------|
    /// | Printable (graphic)         | `0x20..=0x7E`         | [`Action::Print`] / [`State::Ground`]        |
    /// | C0 controls (excluding ESC) | `0x00..=0x1F`         | [`Action::Execute`] / [`State::Ground`]      |
    /// | ESC                         | `0x1B`                | [`Action::None`] / [`State::Escape`]         |
    /// | CSI (C1 mode)               | `0x9B`                | [`Action::None`] / [`State::CsiEntry`]       |
    /// | DCS (C1 mode)               | `0x90`                | [`Action::None`] / [`State::DcsEntry`]       |
    /// | OSC (C1 mode)               | `0x9D`                | [`Action::None`] / [`State::OscString`]      |
    /// | SOS / PM / APC (C1 mode)    | `0x98`/`0x9E`/`0x9F`  | [`Action::None`] / [`State::SosPmApcString`] |
    /// | UTF-8 lead (impl. mode)     | `0xC2..=0xF4`         | [`Action::Utf8`] / [`State::Utf8Sequence`]   |
    ///
    /// > Note: The recognition of raw C1 bytes (0x80..0x9F) and their mapping
    /// > to sequences (CSI, OSC, etc.) is optional in ECMA-48 and depends
    /// > on terminal mode. Error recovery (invalid bytes in sequences) is
    /// > implementation-dependent by standard.
    ///
    /// > Note: UTF-8 is handled by [`State::Utf8Sequence`]. It starts UTF-8 handling mode
    /// > to accumulate a multibyte code point before emitting it as `Print`
    ///
    /// ## Examples
    /// - `Hello, world!` — bytes `0x20..=0x7E` are emitted as `Print`.
    /// - `BEL` (`0x07`) — executed as `Execute` (ring bell).
    /// - `ESC [` — transitions to [`State::CsiEntry`].
    /// - `0x9D` — transitions to [`State::OscString`] if C1 is recognized.
    Ground,
    /// Entry state after receiving the C0 `ESC` (0x1B). The next byte(s)
    /// determine which family of sequence follows:
    ///
    /// - Plain ESC sequences with optional intermediates and a final byte
    ///   (e.g., `ESC ( B`) are dispatched as `EscDispatch`.
    /// - Family introducers like `ESC [` (CSI), `ESC ]` (OSC), `ESC P` (DCS),
    ///   and `ESC X`/`ESC ^`/`ESC _` (SOS/PM/APC) transition into their
    ///   respective dedicated states.
    /// - C0 controls are executed immediately even while in [`State::Escape`].
    ///
    /// ## What the happens in [`State::Escape`]
    /// | Class                       | Byte range                                                | Action / Transition                                 |
    /// |-----------------------------|-----------------------------------------------------------|-----------------------------------------------------|
    /// | C0 controls (except ESC)    | `0x00..=0x17, 0x19, 0x1C..=0x1F`                          | [`Action::Execute`] / [`State::Escape`]             |
    /// | DEL                         | `0x7F`                                                    | [`Action::Ignore`] / [`State::Escape`]              |
    /// | Intermediates               | `0x20..=0x2F`                                             | [`Action::Collect`] / [`State::EscapeIntermediate`] |
    /// | Finals                      | `0x30..=0x4F, 0x51..=0x57, 0x59, 0x5A, 0x5C, 0x60..=0x7E` | [`Action::EscDispatch`] / [`State::Ground`]         |
    /// | CSI introducer              | `[` (`0x5B`)                                              | [`Action::None`] / [`State::CsiEntry`]              |
    /// | OSC introducer              | `]` (`0x5D`)                                              | [`Action::None`] / [`State::OscString`]             |
    /// | DCS introducer              | `P` (`0x50`)                                              | [`Action::None`] / [`State::DcsEntry`]              |
    /// | SOS / PM / APC introducers  | `X`/`^`/`_` (`0x58`/`0x5E`/`0x5F`)                        | [`Action::None`] / [`State::SosPmApcString`]        |
    ///
    /// ## Examples
    /// - `ESC ( B` — collects `(` as intermediate, dispatches final `B` (select G0 ASCII).
    /// - `ESC ] 0 ; title ST` — switches to OSC string collection.
    /// - `ESC P ... ST` — enters DCS passthrough via `DcsEntry`.
    /// - `ESC 0x07` — executed as `Execute` (ring bell).
    Escape,
    /// In terminal control sequences that start with `ESC` (0x1B), *escape intermediate*
    /// bytes are the optional bytes in the range `0x20..=0x2F` that appear **between** the
    /// initial `ESC` and the final byte. They refine or qualify the meaning of the sequence.
    ///
    /// **Why they exist:** they let the protocol encode compact variants of a command
    /// (e.g., selecting different character sets) while staying extensible and backward-
    /// compatible across VT100/VT220/ECMA-48 style terminals.
    ///
    /// **Shape of an ESC sequence:**
    /// ```text
    /// ESC (0x1B) + [ zero or more intermediates 0x20..=0x2F ] + final 0x30..=0x7E
    /// ```
    ///
    /// ## What the happens in [`State::EscapeIntermediate`]
    /// | Class                       | Byte range                       | Action / Transition                                 |
    /// |-----------------------------|----------------------------------|-----------------------------------------------------|
    /// | C0 controls (except ESC)    | `0x00..=0x17, 0x19, 0x1C..=0x1F` | [`Action::Execute`] / [`State::EscapeIntermediate`] |
    /// | DEL                         | `0x7F`                           | [`Action::Ignore`] / [`State::EscapeIntermediate`]  |
    /// | Intermediates               | `0x20..=0x2F`                    | [`Action::Collect`] / [`State::EscapeIntermediate`] |
    /// | Finals                      | `0x30..=0x7E`                    | [`Action::EscDispatch`] / [`State::Ground`]         |
    ///
    /// > Note: CSI sequences (`ESC [` or `0x9B`) have their own grammar; *this section
    /// > is specifically about plain `ESC` sequences.*
    ///
    /// **Examples**
    /// - `ESC ( B` — Select G0 = ASCII. Here `(` is an intermediate, `B` is the final.
    /// - `ESC ) 0` — Select G1 = DEC Special Graphics. `)` is an intermediate, `0` is final.
    /// - `ESC % G` — Enter UTF-8 mode in some terminals. `%` is intermediate, `G` is final.
    EscapeIntermediate,
    /// Entry state after a CSI introducer is observed: either `ESC [` (`0x1B 0x5B`)
    /// or the single-byte C1 form `0x9B` (in 8-bit mode). From here the parser
    /// validates and gathers parameters and optional intermediates before the
    /// final byte determines the command to dispatch.
    ///
    /// ## What the happens in [`State::CsiEntry`]
    /// | Class                       | Byte range                      | Action / Transition                              |
    /// |-----------------------------|---------------------------------|--------------------------------------------------|
    /// | C0 controls (except ESC)    | `0x00..=0x17, 0x19, 0x1C..=0x1F`| [`Action::Execute`] / [`State::CsiEntry`]        |
    /// | DEL                         | `0x7F`                          | [`Action::Ignore`] / [`State::CsiEntry`]         |
    /// | Params (digits/`;`)         | `0x30..=0x39`, `0x3B`           | [`Action::Param`] / [`State::CsiParam`]          |
    /// | Private params              | `0x3C..=0x3F`                   | [`Action::Collect`] / [`State::CsiParam`]        |
    /// | Intermediates               | `0x20..=0x2F`                   | [`Action::Collect`] / [`State::CsiIntermediate`] |
    /// | Invalid (colon)             | `0x3A`                          | [`Action::None`] / [`State::CsiIgnore`]          |
    /// | Finals                      | `0x40..=0x7E`                   | [`Action::CsiDispatch`] / [`State::Ground`]      |
    ///
    /// ## Examples
    /// - `ESC [ m` — SGR with no parameters: enters CSI and immediately dispatches on final `m`.
    /// - `ESC [ ? 25 l` — DEC Private Mode Reset: enters CSI, collects `?` (private), `25` (param), final `l`.
    CsiEntry,
    /// Parameter collection state after entering CSI. Accumulates numeric
    /// parameters separated by semicolons, and handles private parameter bytes.
    /// Transitions to intermediates or dispatches when a final byte arrives.
    ///
    /// ## What are parameters
    /// - Parameters are decimal numbers formed by ASCII digits `0..=9`.
    /// - Multiple parameters are separated by `;` (semicolon).
    /// - Empty fields are allowed (e.g., `ESC [ ; m`) and mean “use the
    ///   default” for that position; the specific default depends on the final.
    /// - Private parameter bytes (`0x3C..=0x3F`, i.e. `<`, `=`, `>`, `?`) may
    ///   appear immediately after CSI and are consumed in [`State::CsiEntry`]
    ///   to qualify the entire sequence; seeing them here is considered
    ///   invalid and transitions to [`State::CsiIgnore`].
    /// - Colon `:` (`0x3A`) sub-parameter separators are not supported by this
    ///   parser and are treated as invalid.
    ///
    /// ## What the happens in [`State::CsiParam`]
    /// | Class                       | Byte range                      | Action / Transition                              |
    /// |-----------------------------|---------------------------------|--------------------------------------------------|
    /// | C0 controls (except ESC)    | `0x00..=0x17, 0x19, 0x1C..=0x1F`| [`Action::Execute`] / [`State::CsiParam`]        |
    /// | DEL                         | `0x7F`                          | [`Action::Ignore`] / [`State::CsiParam`]         |
    /// | Params (digits/`;`)         | `0x30..=0x39`, `0x3B`           | [`Action::Param`] / [`State::CsiParam`]          |
    /// | Private params              | `0x3C..=0x3F`                   | [`Action::None`] / [`State::CsiIgnore`]          |
    /// | Intermediates               | `0x20..=0x2F`                   | [`Action::Collect`] / [`State::CsiIntermediate`] |
    /// | Finals                      | `0x40..=0x7E`                   | [`Action::CsiDispatch`] / [`State::Ground`]      |
    ///
    /// ## Examples
    /// - `ESC [ 2 J` — ED (Erase in Display) with parameter `2`.
    /// - `ESC [ 1 ; 31 m` — SGR with two parameters: bold (`1`) and red (`31`).
    /// - `ESC [ ; m` — SGR with an empty first parameter (defaults apply).
    /// - `ESC [ ? 25 h` — DEC private mode set (`?` handled in `CsiEntry`),
    ///   parameter `25` (show cursor).
    CsiParam,
    /// Intermediate collection after CSI parameters. Collects bytes in the
    /// range `0x20..=0x2F` that further qualify the final. Some bytes in
    /// `0x30..=0x3F` are ignored here. On a valid final, the sequence is
    /// dispatched.
    ///
    /// ## What the happens in [`State::CsiIntermediate`]
    /// | Class                       | Byte range                       | Action / Transition                                         |
    /// |-----------------------------|----------------------------------|-------------------------------------------------------------|
    /// | C0 controls (except ESC)    | `0x00..=0x17, 0x19, 0x1C..=0x1F` | [`Action::Execute`] / [`State::CsiIntermediate`]            |
    /// | DEL                         | `0x7F`                           | [`Action::Ignore`] / [`State::CsiIntermediate`]             |
    /// | Intermediates               | `0x20..=0x2F`                    | [`Action::Collect`] / [`State::CsiIntermediate`]            |
    /// | Ignored (params/privates)   | `0x30..=0x3F`                    | [`Action::None`] / [`State::CsiIntermediate`]               |
    /// | Finals                      | `0x40..=0x7E`                    | [`Action::CsiDispatch`] / [`State::Ground`]                 |
    ///
    /// ## Examples
    /// - `CSI ! p` — DECSTR (soft terminal reset); `!` is an intermediate, final is `p`.
    /// - `CSI Ps SP q` — DECSCUSR (set cursor style); `SP` (space) is an intermediate, final is `q`.
    CsiIntermediate,
    /// Error-recovery state for malformed CSI. Consumes bytes until a final
    /// byte is seen, then returns to ground without dispatch.
    ///
    /// ## What the happens in [`State::CsiIgnore`]
    /// | Class                       | Byte range                       | Action / Transition                                   |
    /// |-----------------------------|----------------------------------|-------------------------------------------------------|
    /// | C0 controls (except ESC)    | `0x00..=0x17, 0x19, 0x1C..=0x1F` | [`Action::Execute`] / [`State::CsiIgnore`]            |
    /// | Params/interm./privates     | `0x20..=0x3F`                    | [`Action::Ignore`] / [`State::CsiIgnore`]             |
    /// | DEL                         | `0x7F`                           | [`Action::Ignore`] / [`State::CsiIgnore`]             |
    /// | Finals                      | `0x40..=0x7E`                    | [`Action::None`] / [`State::Ground`]                  |
    ///
    /// ## Examples
    /// - `ESC [ : 1 m` — invalid `:` after CSI puts parser into ignore until final `m`, then returns to ground.
    /// - `ESC [ 1 : 2 m` — colon in parameters is invalid here; bytes are ignored until final.
    CsiIgnore,
    /// Entry after a DCS introducer (`ESC P` or C1 `0x90`). Collects
    /// parameters and intermediates, or enters passthrough on valid finals.
    ///
    /// ## What the happens in [`State::DcsEntry`]
    /// | Class                       | Byte range                      | Action / Transition                              |
    /// |-----------------------------|---------------------------------|--------------------------------------------------|
    /// | C0 controls (except ESC)    | `0x00..=0x17, 0x19, 0x1C..=0x1F`| [`Action::Execute`] / [`State::DcsEntry`]        |
    /// | DEL                         | `0x7F`                          | [`Action::Ignore`] / [`State::DcsEntry`]         |
    /// | Invalid (colon)             | `0x3A`                          | [`Action::None`] / [`State::DcsIgnore`]          |
    /// | Intermediates               | `0x20..=0x2F`                   | [`Action::Collect`] / [`State::DcsIntermediate`] |
    /// | Params (digits/`;`)         | `0x30..=0x39`, `0x3B`           | [`Action::Param`] / [`State::DcsParam`]          |
    /// | Private params              | `0x3C..=0x3F`                   | [`Action::Collect`] / [`State::DcsParam`]        |
    /// | Finals                      | `0x40..=0x7E`                   | [`Action::None`] / [`State::DcsPassthrough`]     |
    ///
    /// ## Examples
    /// - `ESC P q ... ST` — Sixel: after the `q` final, enters passthrough to stream image data.
    /// - `ESC P $ q Pt ST` — DECRQSS: `$` intermediate then `q` final; device responds with a DCS status string.
    DcsEntry,
    /// Parameter collection for DCS. Gathers numeric parameters and separators;
    /// private bytes may invalidate and switch to ignore. Finals enter
    /// passthrough mode.
    ///
    /// ## What the happens in [`State::DcsParam`]
    /// | Class                       | Byte range                             | Action / Transition                              |
    /// |-----------------------------|----------------------------------------|--------------------------------------------------|
    /// | C0/DEL                      | `0x00..=0x17, 0x19, 0x1C..=0x1F, 0x7F` | [`Action::Ignore`] / [`State::DcsParam`]         |
    /// | Params (digits/`;`)         | `0x30..=0x39`, `0x3B`                  | [`Action::Param`] / [`State::DcsParam`]          |
    /// | Invalid (colon/privates)    | `0x3A`, `0x3C..=0x3F`                  | [`Action::None`] / [`State::DcsIgnore`]          |
    /// | Intermediates               | `0x20..=0x2F`                          | [`Action::Collect`] / [`State::DcsIntermediate`] |
    /// | Finals                      | `0x40..=0x7E`                          | [`Action::None`] / [`State::DcsPassthrough`]     |
    ///
    /// ## Examples
    /// - `ESC P 1 ; 0 ; 8 q ... ST` — Sixel: collects numeric parameters `1;0;8`,
    ///   then final `q` enters passthrough to stream the image payload until `ST`.
    /// - `ESC P 1 ; 2 $ q Pt ST` — DECRQSS with numeric params `1;2` collected in
    ///   `DcsParam`, then `$` (intermediate) and final `q` follow.
    DcsParam,
    /// Intermediate collection for DCS sequences. Continues to collect in
    /// `0x20..=0x2F`. Digits and private bytes here invalidate and switch to
    /// ignore. Finals enter passthrough.
    ///
    /// ## What the happens in [`State::DcsIntermediate`]
    /// | Class                       | Byte range                             | Action / Transition                              |
    /// |-----------------------------|----------------------------------------|--------------------------------------------------|
    /// | C0/DEL                      | `0x00..=0x17, 0x19, 0x1C..=0x1F, 0x7F` | [`Action::Ignore`] / [`State::DcsIntermediate`]  |
    /// | Intermediates               | `0x20..=0x2F`                          | [`Action::Collect`] / [`State::DcsIntermediate`] |
    /// | Invalid (digits/privates)   | `0x30..=0x3F`                          | [`Action::None`] / [`State::DcsIgnore`]          |
    /// | Finals                      | `0x40..=0x7E`                          | [`Action::None`] / [`State::DcsPassthrough`]     |
    ///
    /// ## Examples
    /// - `ESC P $ q Pt ST` — DECRQSS (Request Status String): `$` is an intermediate,
    ///   `q` is the final; `Pt` is the selector for the requested status.
    /// - `ESC P 0 ; 1 $ q Pt ST` — Same as above but with numeric parameters before `$`.
    DcsIntermediate,
    /// Error-recovery state for malformed DCS. Swallows bytes until a string
    /// terminator is observed (ST: `ESC \\` or C1 `0x9C`), then returns to
    /// ground.
    ///
    /// ## What the happens in [`State::DcsIgnore`]
    /// | Class                       | Byte range                                    | Action / Transition                       |
    /// |-----------------------------|-----------------------------------------------|-------------------------------------------|
    /// | C0 controls / Printable     | `0x00..=0x17, 0x19, 0x1C..=0x1F, 0x20..=0x7F` | [`Action::Ignore`] / [`State::DcsIgnore`] |
    /// | String Terminator (ST)      | `0x9C` or `ESC \\`                            | [`Action::None`] / [`State::Ground`]      |
    ///
    /// ## Examples
    /// - `ESC P : ... ST` — invalid `:` after DCS causes ignore until `ST`.
    /// - `ESC P 3 : ... ST` — digits then `:` are invalid here; ignore until terminator.
    DcsIgnore,
    /// Payload streaming mode for DCS. Forwards bytes to the active handler
    /// via [`Action::Put`] until the terminator (ST: `ESC \\` or C1 `0x9C`).
    /// `DEL` is ignored.
    ///
    /// ## What the happens in [`State::DcsPassthrough`]
    /// | Class                       | Byte range                                    | Action / Transition                            |
    /// |-----------------------------|-----------------------------------------------|------------------------------------------------|
    /// | C0 controls / Printable     | `0x00..=0x17, 0x19, 0x1C..=0x1F, 0x20..=0x7E` | [`Action::Put`] / [`State::DcsPassthrough`]    |
    /// | DEL                         | `0x7F`                                        | [`Action::Ignore`] / [`State::DcsPassthrough`] |
    /// | String Terminator (ST)      | `0x9C` or `ESC \\`                            | [`Action::None`] / [`State::Ground`]           |
    ///
    /// ## Examples
    /// - `ESC P q <sixel-data> ST` — streams sixel payload via `Put` until `ST`.
    /// - `ESC P 1 ; 2 q <payload> ST` — parameters collected earlier; `q` final enters passthrough.
    DcsPassthrough,
    /// Collects an OSC payload after `ESC ]` (or C1 `0x9D`). On entry it
    /// triggers [`Action::OscStart`], then accumulates bytes as [`Action::OscPut`]
    /// until terminated by BEL (`0x07`) or ST (`ESC \\` or C1 `0x9C`). Other C0
    /// controls are ignored within the string. On exit, dispatches via
    /// [`Action::OscEnd`].
    ///
    /// ## What the happens in [`State::OscString`]
    /// | Class                       | Byte range                                    | Action / Transition                        |
    /// |-----------------------------|-----------------------------------------------|--------------------------------------------|
    /// | C0 (except BEL)             | `0x00..=0x06, 0x08..=0x17, 0x19, 0x1C..=0x1F` | [`Action::Ignore`] / [`State::OscString`]  |
    /// | BEL terminator              | `0x07`                                        | [`Action::None`] / [`State::Ground`]       |
    /// | Printable                   | `0x20..=0x7F`                                 | [`Action::OscPut`] / [`State::OscString`]  |
    /// | UTF-8 lead                  | `0xC2..=0xF4`                                 | [`Action::Utf8`] / [`State::Utf8Sequence`] |
    /// | String Terminator (ST)      | `0x9C` or `ESC \\`                            | [`Action::None`] / [`State::Ground`]       |
    ///
    /// ## Examples
    /// - `ESC ] 0 ; My Title BEL` — set window/icon title.
    /// - `ESC ] 52 ; c ; <base64> ST` — clipboard (OSC 52) with base64 payload, terminated by `ST`.
    OscString,
    /// Collects SOS, PM, or APC strings (introduced via `ESC X`, `ESC ^`,
    /// `ESC _` or C1 `0x98`/`0x9E`/`0x9F`). Bytes are ignored until a string
    /// terminator (ST: `ESC \\` or C1 `0x9C`) is observed, then the parser
    /// returns to ground.
    ///
    /// ## What the happens in [`State::SosPmApcString`]
    /// | Class                       | Byte range            | Action / Transition                            |
    /// |-----------------------------|-----------------------|------------------------------------------------|
    /// | C0 / Printable              | `0x00..=0x7F`         | [`Action::Ignore`] / [`State::SosPmApcString`] |
    /// | String Terminator (ST)      | `0x9C` or `ESC \\`    | [`Action::None`] / [`State::Ground`]           |
    ///
    /// ## Examples
    /// - `ESC _ note goes here ST` — APC string ignored until `ST`.
    /// - `ESC ^ program message ST` — PM string ignored until `ST`.
    SosPmApcString,
    /// Idle state used when no data has been processed yet or as a sentinel
    /// for unreachable cases. No actions are performed.
    Nothing,
    /// UTF-8 continuation bytes are being processed.
    Utf8Sequence,
    #[allow(dead_code)]
    /// Wildcard state used internally by the state machine logic for transitions
    /// that can occur from any state (e.g., handling an immediate ESC to cancel
    /// a sequence). The parser itself is never in the Anywhere state.
    Anywhere,
}

/// The [`Action`] enum accompanies the state machine. Each parsed byte maps to a
/// transition and an action telling higher-level code how to process the
/// character (emit printable data, accumulate parameters, dispatch a sequence,
/// etc.). A transition table typically couples both enums so that the parser
/// remains declarative and extensible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    /// No side effects required.
    None,
    /// Byte is ignored entirely.
    Ignore,
    /// Byte is part of an in-progress UTF-8 sequence.
    Utf8,
    /// Printable character should be forwarded to the output.
    Print,
    /// C0/C1 control should be executed immediately.
    Execute,
    /// Reset accumulated buffers (parameters, intermediates, etc.).
    Clear,
    /// Collect intermediate bytes for ESC/DCS/CSI sequences.
    Collect,
    /// Collect numeric parameters for CSI/DCS sequences.
    Param,
    /// Dispatch an ESC sequence to the higher-level handler.
    EscDispatch,
    /// Dispatch a CSI sequence to the higher-level handler.
    CsiDispatch,
    /// Start streaming data for a DCS handler.
    Hook,
    /// Pass a byte through to the active DCS handler.
    Put,
    /// Terminate the active DCS handler.
    Unhook,
    /// Begin collecting an OSC payload.
    OscStart,
    /// Append a byte to the OSC payload buffer.
    OscPut,
    /// Finalize the OSC payload and dispatch it.
    OscEnd,
}
