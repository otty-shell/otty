use otty_surface::{Point, SurfaceMode};

/// Mouse button or motion code used by terminal mouse-reporting protocols.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalMouseButton {
    /// Primary button press/release.
    Left,
    /// Middle button press/release.
    Middle,
    /// Secondary button press/release.
    Right,
    /// Motion while the primary button is held.
    LeftMove,
    /// Motion while the middle button is held.
    MiddleMove,
    /// Motion while the secondary button is held.
    RightMove,
    /// Motion without a pressed button.
    Move,
    /// Wheel movement toward older content.
    ScrollUp,
    /// Wheel movement toward newer content.
    ScrollDown,
}

impl TerminalMouseButton {
    const fn code(self) -> u8 {
        match self {
            Self::Left => 0,
            Self::Middle => 1,
            Self::Right => 2,
            Self::LeftMove => 32,
            Self::MiddleMove => 33,
            Self::RightMove => 34,
            Self::Move => 35,
            Self::ScrollUp => 64,
            Self::ScrollDown => 65,
        }
    }
}

/// Modifier state encoded into an xterm mouse report.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerminalMouseModifiers {
    shift: bool,
    alt: bool,
    control: bool,
}

impl TerminalMouseModifiers {
    /// Create modifier state from shift, alt, and control/platform flags.
    pub const fn new(shift: bool, alt: bool, control: bool) -> Self {
        Self {
            shift,
            alt,
            control,
        }
    }

    const fn code(self) -> u8 {
        (self.shift as u8) * 4
            + (self.alt as u8) * 8
            + (self.control as u8) * 16
    }
}

/// Encode one xterm SGR, UTF-8, or legacy mouse report.
///
/// Returns `None` when the coordinate cannot be represented by the active
/// protocol. The caller decides whether mouse reporting is enabled.
pub fn encode_mouse_report(
    mode: SurfaceMode,
    point: Point,
    button: TerminalMouseButton,
    modifiers: TerminalMouseModifiers,
    pressed: bool,
) -> Option<Vec<u8>> {
    if point.line.0 < 0 {
        return None;
    }

    let button = button.code() + modifiers.code();
    if mode.contains(SurfaceMode::SGR_MOUSE) {
        return Some(encode_sgr(point, button, pressed));
    }

    encode_legacy(
        point,
        button,
        pressed,
        mode.contains(SurfaceMode::UTF8_MOUSE),
    )
}

fn encode_sgr(point: Point, button: u8, pressed: bool) -> Vec<u8> {
    let suffix = if pressed { 'M' } else { 'm' };

    format!(
        "\x1b[<{button};{};{}{suffix}",
        point.column.0 + 1,
        point.line.0 + 1,
    )
    .into_bytes()
}

fn encode_legacy(
    point: Point,
    button: u8,
    pressed: bool,
    utf8: bool,
) -> Option<Vec<u8>> {
    let line = usize::try_from(point.line.0).ok()?;
    let column = point.column.0;
    let max_point = if utf8 { 2015 } else { 223 };
    if line >= max_point || column >= max_point {
        return None;
    }

    let button = if pressed { button } else { 3 + (button & !3) };
    let mut report = vec![b'\x1b', b'[', b'M', 32 + button];
    encode_legacy_coordinate(&mut report, column, utf8);
    encode_legacy_coordinate(&mut report, line, utf8);

    Some(report)
}

fn encode_legacy_coordinate(
    report: &mut Vec<u8>,
    coordinate: usize,
    utf8: bool,
) {
    let encoded = 33 + coordinate;
    if utf8 && coordinate >= 95 {
        report.push((0xc0 + encoded / 64) as u8);
        report.push((0x80 + (encoded & 63)) as u8);
    } else {
        report.push(encoded as u8);
    }
}

#[cfg(test)]
mod tests {
    use otty_surface::{Column, Line, Point, SurfaceMode};

    use super::*;

    #[test]
    fn encodes_sgr_press_release_and_modifiers() {
        let point = Point::new(Line(2), Column(3));
        let modifiers = TerminalMouseModifiers::new(true, false, true);

        assert_eq!(
            encode_mouse_report(
                SurfaceMode::SGR_MOUSE,
                point,
                TerminalMouseButton::Left,
                modifiers,
                true,
            ),
            Some(b"\x1b[<20;4;3M".to_vec()),
        );
        assert_eq!(
            encode_mouse_report(
                SurfaceMode::SGR_MOUSE,
                point,
                TerminalMouseButton::Left,
                modifiers,
                false,
            ),
            Some(b"\x1b[<20;4;3m".to_vec()),
        );
    }

    #[test]
    fn encodes_legacy_and_utf8_coordinates() {
        let point = Point::new(Line(1), Column(2));
        let expected = vec![0x1b, b'[', b'M', 32, 35, 34];

        assert_eq!(
            encode_mouse_report(
                SurfaceMode::empty(),
                point,
                TerminalMouseButton::Left,
                TerminalMouseModifiers::default(),
                true,
            ),
            Some(expected),
        );

        let utf8_point = Point::new(Line(95), Column(95));
        let report = encode_mouse_report(
            SurfaceMode::UTF8_MOUSE,
            utf8_point,
            TerminalMouseButton::Left,
            TerminalMouseModifiers::default(),
            true,
        )
        .expect("valid UTF-8 mouse report");
        assert!(report.len() > 6);
    }

    #[test]
    fn rejects_coordinates_outside_protocol_range() {
        let point = Point::new(Line(300), Column(300));

        assert_eq!(
            encode_mouse_report(
                SurfaceMode::empty(),
                point,
                TerminalMouseButton::Left,
                TerminalMouseModifiers::default(),
                true,
            ),
            None,
        );
    }
}
