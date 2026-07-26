use std::ops::Range;

pub(crate) fn utf16_range_to_byte(
    text: &str,
    range: Range<usize>,
) -> Range<usize> {
    let start = utf16_offset_to_byte(text, range.start);
    let end = utf16_offset_to_byte(text, range.end);

    start.min(end)..start.max(end)
}

pub(crate) fn byte_range_to_utf16(
    text: &str,
    range: Range<usize>,
) -> Range<usize> {
    let start = previous_char_boundary(text, range.start);
    let end = previous_char_boundary(text, range.end);
    let start = text[..start].encode_utf16().count();
    let end = text[..end].encode_utf16().count();

    start.min(end)..start.max(end)
}

fn utf16_offset_to_byte(text: &str, offset: usize) -> usize {
    let mut utf16_offset = 0;
    for (byte_offset, character) in text.char_indices() {
        if utf16_offset >= offset {
            return byte_offset;
        }

        let next = utf16_offset + character.len_utf16();
        if offset < next {
            return byte_offset;
        }
        utf16_offset = next;
    }

    text.len()
}

fn previous_char_boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }

    offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_utf16_ranges_without_splitting_unicode_scalars() {
        let text = "a😀b";

        assert_eq!(utf16_range_to_byte(text, 1..3), 1..5);
        assert_eq!(byte_range_to_utf16(text, 1..5), 1..3);
    }

    #[test]
    fn clamps_out_of_bounds_ime_ranges() {
        let text = "abc";
        let reversed_start = 99;
        let reversed_end = 1;

        assert_eq!(utf16_range_to_byte(text, 2..99), 2..3);
        assert_eq!(
            utf16_range_to_byte(text, reversed_start..reversed_end),
            1..3
        );
    }
}
