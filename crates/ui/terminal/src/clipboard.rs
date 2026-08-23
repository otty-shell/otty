use iced_core::clipboard::{Clipboard, Kind as ClipboardKind};

/// Read text for a terminal paste operation.
pub(crate) fn read_paste_text(clipboard: &dyn Clipboard) -> Option<String> {
    #[cfg(target_os = "macos")]
    if let Some(path) = macos_file_url_path() {
        return Some(path);
    }

    #[cfg(target_os = "macos")]
    {
        clipboard
            .read(ClipboardKind::Standard)
            .map(|text| file_url_to_posix_path(&text).unwrap_or(text))
    }

    #[cfg(not(target_os = "macos"))]
    {
        clipboard.read(ClipboardKind::Standard)
    }
}

#[cfg(target_os = "macos")]
fn macos_file_url_path() -> Option<String> {
    use objc2_app_kit::NSPasteboard;

    let pasteboard = unsafe { NSPasteboard::generalPasteboard() };
    macos_file_url_path_from(&pasteboard)
}

#[cfg(target_os = "macos")]
fn macos_file_url_path_from(
    pasteboard: &objc2_app_kit::NSPasteboard,
) -> Option<String> {
    use objc2_app_kit::NSPasteboardTypeFileURL;

    let file_url =
        unsafe { pasteboard.stringForType(NSPasteboardTypeFileURL) }?;

    file_url_to_posix_path(&file_url.to_string())
}

#[cfg(target_os = "macos")]
fn file_url_to_posix_path(value: &str) -> Option<String> {
    use objc2_foundation::{NSString, NSURL};

    let value = NSString::from_str(value);
    let url = unsafe { NSURL::URLWithString(&value) }?;
    let scheme = unsafe { url.scheme()? };
    if !scheme.to_string().eq_ignore_ascii_case("file") {
        return None;
    }

    unsafe { url.path() }.map(|path| path.to_string())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use objc2_app_kit::{
        NSPasteboard, NSPasteboardTypeFileURL, NSPasteboardTypeString,
    };
    use objc2_foundation::NSString;

    use super::{file_url_to_posix_path, macos_file_url_path_from};

    #[test]
    fn file_url_to_posix_path_decodes_file_url() {
        assert_eq!(
            file_url_to_posix_path(
                "file:///Users/example/My%20File%20%E3%83%86%E3%82%B9%E3%83%88.txt"
            )
            .as_deref(),
            Some("/Users/example/My File テスト.txt")
        );
    }

    #[test]
    fn file_url_to_posix_path_rejects_non_file_url() {
        assert_eq!(
            file_url_to_posix_path("https://example.com/file.txt"),
            None
        );
    }

    #[test]
    fn macos_pasteboard_prefers_file_url_over_text_representation() {
        let pasteboard = unsafe { NSPasteboard::pasteboardWithUniqueName() };
        let file_url = NSString::from_str(
            "file:///Users/example/My%20File%20%E3%83%86%E3%82%B9%E3%83%88.txt",
        );
        let written = unsafe {
            pasteboard.setString_forType(&file_url, NSPasteboardTypeFileURL)
        };
        let text = NSString::from_str("otty-file-icon.icns");
        let text_written = unsafe {
            pasteboard.setString_forType(&text, NSPasteboardTypeString)
        };

        assert!(written);
        assert!(text_written);
        assert_eq!(
            macos_file_url_path_from(&pasteboard).as_deref(),
            Some("/Users/example/My File テスト.txt")
        );
    }
}
