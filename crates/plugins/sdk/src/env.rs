//! The home folder and the variables the called part lists in `env`.

/// The variable `name` as text; `None` when it is not set or the part does
/// not list it. A variable that holds a key is read only through
/// [`keys::open`](crate::keys::open).
pub fn var(name: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::env::var(name)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::var(name)
    }
}

/// The user's home folder, what `~` stands for in the manifest.
pub fn home() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::env::home()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::home()
    }
}

/// The language of OTTY's interface, as a tag such as `en`, `ru` or
/// `pt-BR`. A part writes the texts of its needs in it, or in English;
/// [`in_language`] picks one of a part's translations.
pub fn language() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::env::language()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::language()
    }
}

/// The text of `texts`, pairs of a language tag and a text, for the
/// language of OTTY's interface ([`language`]); see [`pick_language`].
pub fn in_language<'a>(texts: &[(&str, &'a str)]) -> &'a str {
    pick_language(&language(), texts)
}

/// The text of `texts`, pairs of a language tag and a text, for the
/// language `tag`: the same tag, else the same language with another
/// region or none (`pt` for `pt-BR`), else English (`en`), else the first
/// text; an empty text when there is none.
///
/// ```
/// use otty_plugin_sdk::env::pick_language;
///
/// let texts = [("en", "Log in"), ("pt-BR", "Entrar"), ("ru", "Войти")];
///
/// assert_eq!(pick_language("ru", &texts), "Войти");
/// assert_eq!(pick_language("pt", &texts), "Entrar");
/// assert_eq!(pick_language("ko", &texts), "Log in");
/// ```
pub fn pick_language<'a>(tag: &str, texts: &[(&str, &'a str)]) -> &'a str {
    let language = |tag: &str| tag.split('-').next().unwrap_or(tag).to_string();
    let wanted = language(tag);

    let exact = texts
        .iter()
        .find(|(text_tag, _)| text_tag.eq_ignore_ascii_case(tag));
    let same_language = texts
        .iter()
        .find(|(text_tag, _)| language(text_tag).eq_ignore_ascii_case(&wanted));
    let english = texts.iter().find(|(text_tag, _)| *text_tag == "en");

    exact
        .or(same_language)
        .or(english)
        .or(texts.first())
        .map_or("", |(_, text)| text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXTS: [(&str, &str); 3] =
        [("en", "Log in"), ("zh-TW", "登入"), ("zh-CN", "登录")];

    #[test]
    fn the_same_tag_wins_over_the_same_language() {
        assert_eq!(pick_language("zh-CN", &TEXTS), "登录");
        assert_eq!(pick_language("ZH-tw", &TEXTS), "登入");
        assert_eq!(pick_language("zh", &TEXTS), "登入");
    }

    #[test]
    fn an_unknown_language_falls_back_to_english_then_the_first() {
        assert_eq!(pick_language("de", &TEXTS), "Log in");
        assert_eq!(pick_language("de", &[("ru", "Войти")]), "Войти");
        assert_eq!(pick_language("de", &[]), "");
    }
}
