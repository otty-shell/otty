//! What the parts ask the user to do, in English or Russian, the language
//! of OTTY's interface: OTTY shows each need as it is, with the plugin's
//! name, and its button does the need's action. Any other language falls
//! back to English.

use otty_plugin_sdk::env::in_language;
use otty_plugin_sdk::{Command, KeyAsk, Need, NeedAction};

/// The title of [`not_logged_in`].
const NOT_LOGGED_IN_TITLE: &[(&str, &str)] =
    &[("en", "Needs login"), ("ru", "Нужен вход")];

/// The text of [`not_logged_in`].
const NOT_LOGGED_IN_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "Nobody is logged in with this config folder. The agent logs in in \
         a terminal tab. Refresh once it is done.",
    ),
    (
        "ru",
        "С этой папкой настроек никто не вошёл. Агент выполнит вход во \
         вкладке терминала. После этого обновите.",
    ),
];

/// The title of [`new_key`].
const NEW_KEY_TITLE: &[(&str, &str)] =
    &[("en", "The key was refused"), ("ru", "Ключ отклонён")];

/// The text of [`new_key`]; `{provider}` is the provider's name.
const NEW_KEY_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "{provider} refused the key: it is wrong, expired or revoked. Make a \
         new key on its page and put it where the agent reads it.",
    ),
    (
        "ru",
        "{provider} отклонил ключ: он неверный, истёк или отозван. Создайте \
         новый ключ на его странице и положите туда, где его читает агент.",
    ),
];

/// The title of [`bill_key`].
const BILL_KEY_TITLE: &[(&str, &str)] = &[
    ("en", "The real bill needs an admin key"),
    ("ru", "Для настоящего счёта нужен ключ администратора"),
];

/// The text of [`bill_key`]: since issue 01 of spec
/// 2026-11-23-accounts-apart-from-profiles OTTY shows no estimate from the
/// tokens, so an organization shows no money until it has an admin key.
const BILL_KEY_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "OTTY shows no money for this organization until it has an admin \
         key, which reads the real bill. No agent keeps one.",
    ),
    (
        "ru",
        "OTTY не показывает деньги этой организации, пока у неё нет ключа \
         администратора, который читает настоящий счёт. Ни один агент его не \
         хранит.",
    ),
];

/// The name of the field of the key of the bill.
const BILL_KEY_LABEL: &[(&str, &str)] =
    &[("en", "Admin key"), ("ru", "Ключ администратора")];

/// The line under the field of the key of the bill.
const BILL_KEY_HINT: &[(&str, &str)] = &[
    (
        "en",
        "Reads the real bill of the whole organization. OTTY saves only this \
         path or name, never the key.",
    ),
    (
        "ru",
        "Читает настоящий счёт всей организации. OTTY сохраняет только этот \
         путь или имя, но не сам ключ.",
    ),
];

/// Nobody is logged in with the profile: the button runs `login`, the
/// agent's login command, in a terminal tab.
pub(crate) fn not_logged_in(login: Command) -> Need {
    Need {
        id: "not-logged-in".to_string(),
        title: in_language(NOT_LOGGED_IN_TITLE).to_string(),
        text: in_language(NOT_LOGGED_IN_TEXT).to_string(),
        action: NeedAction::Run(login),
    }
}

/// The bill needs an admin key of the organization, which the user gives
/// once: OTTY keeps where they keep it and gives it back under `slot`.
/// The key starts with `prefix` and is made on `page`.
pub(crate) fn bill_key(slot: &str, prefix: &str, page: &str) -> Need {
    Need {
        id: "bill-key".to_string(),
        title: in_language(BILL_KEY_TITLE).to_string(),
        text: in_language(BILL_KEY_TEXT).to_string(),
        action: NeedAction::AskKey(KeyAsk {
            slot: Some(slot.to_string()),
            label: in_language(BILL_KEY_LABEL).to_string(),
            hint: in_language(BILL_KEY_HINT).to_string(),
            prefix: Some(prefix.to_string()),
            page: Some(page.to_string()),
        }),
    }
}

/// `provider` refused the key, one an agent keeps: the button opens
/// `keys_page`, where the user makes a new one, which they put where the
/// agent reads the old one.
pub(crate) fn new_key(provider: &str, keys_page: &str) -> Need {
    Need {
        id: "new-key".to_string(),
        title: in_language(NEW_KEY_TITLE).to_string(),
        text: in_language(NEW_KEY_TEXT).replace("{provider}", provider),
        action: NeedAction::OpenPage(keys_page.to_string()),
    }
}
