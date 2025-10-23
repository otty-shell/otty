#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Charset {
    /// ASCII Character set
    Ascii,
    /// DEC Line Drawing Character set
    DecLineDrawing,
}

impl Default for Charset {
    fn default() -> Self {
        Self::Ascii
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CharsetIndex {
    /// Designate G0 Character Set
    #[default]
    G0,
    /// Designate G1 Character Set
    G1,
    /// Designate G2 Character Set
    G2,
    /// Designate G3 Character Set
    G3,
}

impl Charset {
    // /// Set charset index and return the new charset
    // #[inline]
    // pub fn set_index(self, index: CharsetIndex) -> Self {
    //     match self {
    //         Self::Ascii(_) => Self::Ascii(index),
    //         Self::DecLineDrawing(_) => Self::DecLineDrawing(index),
    //     }
    // }

    /// Switch/Map character to the active charset. Ascii is the common case and
    /// for that we want to do as little as possible.
    #[inline]
    pub fn map(self, c: char) -> char {
        match self {
            Self::Ascii => c,
            Self::DecLineDrawing => match c {
                '_' => ' ',
                '`' => '◆',
                'a' => '▒',
                'b' => '\u{2409}', // Symbol for horizontal tabulation
                'c' => '\u{240c}', // Symbol for form feed
                'd' => '\u{240d}', // Symbol for carriage return
                'e' => '\u{240a}', // Symbol for line feed
                'f' => '°',
                'g' => '±',
                'h' => '\u{2424}', // Symbol for newline
                'i' => '\u{240b}', // Symbol for vertical tabulation
                'j' => '┘',
                'k' => '┐',
                'l' => '┌',
                'm' => '└',
                'n' => '┼',
                'o' => '⎺',
                'p' => '⎻',
                'q' => '─',
                'r' => '⎼',
                's' => '⎽',
                't' => '├',
                'u' => '┤',
                'v' => '┴',
                'w' => '┬',
                'x' => '│',
                'y' => '≤',
                'z' => '≥',
                '{' => 'π',
                '|' => '≠',
                '}' => '£',
                '~' => '·',
                _ => c,
            },
        }
    }
}
