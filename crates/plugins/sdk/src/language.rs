//! The language receiver `otty:language@0.1.0` as a trait.

use std::marker::PhantomData;

/// A plugin whose texts, such as the titles and texts of its needs, come
/// in several languages. OTTY calls it when the user changes the language
/// of its interface; [`env::language`](crate::env::language) gives the new
/// one from then on. A plugin whose texts are only in English does not
/// implement it, and OTTY never calls it.
///
/// It belongs to no kind of part: it is exported for the whole plugin
/// with `export!(Example with agents::agent, language)`, and the manifest
/// says nothing about it.
pub trait Language {
    /// The user changed the language to `tag`, such as `ru` or `pt-BR`.
    /// `true` when the plugin's texts depend on the language: OTTY reads
    /// its parts again at once, so they show in the new language.
    fn language_changed(tag: &str) -> bool;
}

/// What [`export!`](crate::export) exports for a plugin type `T` with
/// `language`: the binding's trait, served by `T`'s [`Language`].
#[doc(hidden)]
pub struct LanguageExport<T>(PhantomData<T>);

#[cfg(target_arch = "wasm32")]
impl<T: Language>
    crate::bindings::language::exports::otty::language::language::Guest
    for LanguageExport<T>
{
    fn language_changed(tag: String) -> bool {
        T::language_changed(&tag)
    }
}
