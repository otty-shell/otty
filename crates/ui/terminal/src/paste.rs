//! What the terminal writes to its program when text is pasted into it,
//! and when the command of a block runs again.

use otty_libterm::surface::SurfaceMode;

/// The mark a program that asked for bracketed paste sees before pasted
/// text.
const PASTE_START: &[u8] = b"\x1b[200~";
/// The mark after pasted text.
const PASTE_END: &[u8] = b"\x1b[201~";
/// What the Enter key sends.
const ENTER: &[u8] = b"\r";
/// Ctrl E, then Ctrl U: in bash (readline) and zsh (emacs keymap) the
/// cursor goes to the end of the line typed at the prompt, and the whole
/// line is cleared. The shell keeps the cleared text for Ctrl Y.
const CLEAR_LINE: &[u8] = b"\x05\x15";

/// The bytes that paste `text` into a program whose terminal is in `mode`.
///
/// When the program asked for bracketed paste (shells do at the prompt),
/// the text goes between the paste marks, so text of several lines stays
/// one input instead of running line by line. Escape characters are left
/// out of it, so the text can not end the paste early. Without bracketed
/// paste every line break becomes the carriage return of the Enter key.
pub(crate) fn paste_input(text: &str, mode: SurfaceMode) -> Vec<u8> {
    if mode.contains(SurfaceMode::BRACKETED_PASTE) {
        let text = text.replace('\x1b', "");

        return [PASTE_START, text.as_bytes(), PASTE_END].concat();
    }

    text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
}

/// The bytes that type `command` at the prompt of a program in `mode` and
/// press Enter. What is already typed at the prompt is cleared first, so
/// only the command runs. The command is pasted, so a command of several
/// lines runs as one command. Line breaks and spaces at its end are left
/// out.
pub(crate) fn run_again_input(command: &str, mode: SurfaceMode) -> Vec<u8> {
    let paste = paste_input(command.trim_end(), mode);

    [CLEAR_LINE, paste.as_slice(), ENTER].concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_bracketed_paste_when_text_of_two_lines_is_pasted_then_it_is_one_paste()
     {
        let input =
            paste_input("for i in 1 2\ndo echo $i; done", SurfaceMode::all());

        assert_eq!(input, b"\x1b[200~for i in 1 2\ndo echo $i; done\x1b[201~");
    }

    #[test]
    fn given_bracketed_paste_when_the_text_holds_the_end_mark_then_its_escape_is_left_out()
     {
        let input =
            paste_input("a\x1b[201~rm -rf ~", SurfaceMode::BRACKETED_PASTE);

        assert_eq!(input, b"\x1b[200~a[201~rm -rf ~\x1b[201~");
    }

    #[test]
    fn given_no_bracketed_paste_when_text_is_pasted_then_line_breaks_are_carriage_returns()
     {
        let input = paste_input("ls\r\npwd\nid", SurfaceMode::empty());

        assert_eq!(input, b"ls\rpwd\rid");
    }

    #[test]
    fn given_bracketed_paste_when_a_command_runs_again_then_the_line_is_cleared_it_is_pasted_and_enter_follows()
     {
        let input =
            run_again_input("git log --oneline -20\n", SurfaceMode::all());

        assert_eq!(input, b"\x05\x15\x1b[200~git log --oneline -20\x1b[201~\r");
    }

    #[test]
    fn given_no_bracketed_paste_when_a_command_runs_again_then_the_line_is_cleared_it_is_typed_and_enter_follows()
     {
        let input = run_again_input("ls -la", SurfaceMode::empty());

        assert_eq!(input, b"\x05\x15ls -la\r");
    }
}
