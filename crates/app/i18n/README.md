# otty-app-i18n

The translations of the OTTY app: the text keys, the languages and the
translated texts built into the app.

It is a separate crate so that every feature can show translated text. It has
no `iced` and no `tokio`; its only outside call is asking the system which
language it prefers.
