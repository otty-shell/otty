//! What the part asks the user to do, in the language of OTTY's
//! interface: OTTY shows the need as it is, with the plugin's name, and
//! its button does the need's action. A text missing in a language falls
//! back to English.

use otty_plugin_sdk::env::in_language;
use otty_plugin_sdk::{KeyAsk, Need, NeedAction};

/// The title of [`new_key`] and [`replace_key`].
const NEW_KEY_TITLE: &[(&str, &str)] = &[
    ("en", "The key was refused"),
    ("zh-CN", "密钥被拒绝"),
    ("zh-TW", "金鑰遭拒"),
    ("ja", "キーが拒否されました"),
    ("ko", "키가 거부되었습니다"),
    ("fr", "La clé a été refusée"),
    ("de", "Der Schlüssel wurde abgelehnt"),
    ("es", "La clave fue rechazada"),
    ("pt-BR", "A chave foi recusada"),
    ("ru", "Ключ отклонён"),
];

/// The text of [`new_key`].
const NEW_KEY_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "OpenRouter refused the key: it is wrong, expired or revoked. Make a \
         new key on the OpenRouter page and put it in place of the old one.",
    ),
    (
        "zh-CN",
        "OpenRouter 拒绝了该密钥：它有误、已过期或已被吊销。请在 OpenRouter \
         页面上创建新密钥，并用它替换旧密钥。",
    ),
    (
        "zh-TW",
        "OpenRouter 拒絕了此金鑰：它有誤、已過期或已被撤銷。請在 OpenRouter \
         頁面上建立新金鑰，並用它取代舊金鑰。",
    ),
    (
        "ja",
        "OpenRouter がキーを拒否しました。キーが間違っているか、期限切れか、\
         取り消されています。OpenRouter のページで新しいキーを作り、\
         古いキーと置き換えてください。",
    ),
    (
        "ko",
        "OpenRouter가 키를 거부했습니다. 키가 잘못되었거나 만료되었거나 \
         폐기되었습니다. OpenRouter 페이지에서 새 키를 만들어 이전 키 자리에 \
         넣으세요.",
    ),
    (
        "fr",
        "OpenRouter a refusé la clé : elle est fausse, expirée ou révoquée. \
         Créez une nouvelle clé sur la page d’OpenRouter et mettez-la à la \
         place de l’ancienne.",
    ),
    (
        "de",
        "OpenRouter hat den Schlüssel abgelehnt: Er ist falsch, abgelaufen \
         oder widerrufen. Erstelle auf der Seite von OpenRouter einen neuen \
         Schlüssel und setze ihn an die Stelle des alten.",
    ),
    (
        "es",
        "OpenRouter rechazó la clave: es incorrecta, caducó o fue revocada. \
         Crea una clave nueva en la página de OpenRouter y ponla en lugar de \
         la anterior.",
    ),
    (
        "pt-BR",
        "O OpenRouter recusou a chave: ela está errada, expirou ou foi \
         revogada. Crie uma chave nova na página do OpenRouter e coloque-a no \
         lugar da antiga.",
    ),
    (
        "ru",
        "OpenRouter отклонил ключ: он неверный, истёк или отозван. Создайте \
         новый ключ на странице OpenRouter и поставьте его вместо старого.",
    ),
];

/// The text of [`replace_key`].
const REPLACE_KEY_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "OpenRouter refused the key: it is wrong, expired or revoked. Make a \
         new key on the OpenRouter page and give it here in place of the old \
         one.",
    ),
    (
        "zh-CN",
        "OpenRouter 拒绝了该密钥：它有误、已过期或已被吊销。请在 OpenRouter \
         页面上创建新密钥，并在此处提供它来替换旧密钥。",
    ),
    (
        "zh-TW",
        "OpenRouter 拒絕了此金鑰：它有誤、已過期或已被撤銷。請在 OpenRouter \
         頁面上建立新金鑰，並在此處提供它來取代舊金鑰。",
    ),
    (
        "ja",
        "OpenRouter がキーを拒否しました。キーが間違っているか、期限切れか、\
         取り消されています。OpenRouter のページで新しいキーを作り、\
         古いキーの代わりにここで指定してください。",
    ),
    (
        "ko",
        "OpenRouter가 키를 거부했습니다. 키가 잘못되었거나 만료되었거나 \
         폐기되었습니다. OpenRouter 페이지에서 새 키를 만들어 이전 키 대신 \
         여기에 지정하세요.",
    ),
    (
        "fr",
        "OpenRouter a refusé la clé : elle est fausse, expirée ou révoquée. \
         Créez une nouvelle clé sur la page d’OpenRouter et indiquez-la ici à \
         la place de l’ancienne.",
    ),
    (
        "de",
        "OpenRouter hat den Schlüssel abgelehnt: Er ist falsch, abgelaufen \
         oder widerrufen. Erstelle auf der Seite von OpenRouter einen neuen \
         Schlüssel und gib ihn hier an Stelle des alten an.",
    ),
    (
        "es",
        "OpenRouter rechazó la clave: es incorrecta, caducó o fue revocada. \
         Crea una clave nueva en la página de OpenRouter e indícala aquí en \
         lugar de la anterior.",
    ),
    (
        "pt-BR",
        "O OpenRouter recusou a chave: ela está errada, expirou ou foi \
         revogada. Crie uma chave nova na página do OpenRouter e informe-a \
         aqui no lugar da antiga.",
    ),
    (
        "ru",
        "OpenRouter отклонил ключ: он неверный, истёк или отозван. Создайте \
         новый ключ на странице OpenRouter и укажите его здесь вместо \
         старого.",
    ),
];

/// The name of the field of [`replace_key`].
const KEY_LABEL: &[(&str, &str)] = &[
    ("en", "OpenRouter key"),
    ("zh-CN", "OpenRouter 密钥"),
    ("zh-TW", "OpenRouter 金鑰"),
    ("ja", "OpenRouter キー"),
    ("ko", "OpenRouter 키"),
    ("fr", "Clé OpenRouter"),
    ("de", "OpenRouter-Schlüssel"),
    ("es", "Clave de OpenRouter"),
    ("pt-BR", "Chave do OpenRouter"),
    ("ru", "Ключ OpenRouter"),
];

/// The line under the field of [`replace_key`].
const KEY_HINT: &[(&str, &str)] = &[
    (
        "en",
        "Takes the place of the refused key. OTTY saves only this path or \
         name, never the key.",
    ),
    (
        "zh-CN",
        "替换被拒绝的密钥。OTTY 只保存这个路径或名称，从不保存密钥。",
    ),
    (
        "zh-TW",
        "取代遭拒的金鑰。OTTY 只儲存這個路徑或名稱，從不儲存金鑰。",
    ),
    (
        "ja",
        "拒否されたキーと置き換わります。OTTY が保存するのはこのパスか名前\
         だけで、キーは保存しません。",
    ),
    (
        "ko",
        "거부된 키를 대신합니다. OTTY는 이 경로나 이름만 저장하고 키는 \
         저장하지 않습니다.",
    ),
    (
        "fr",
        "Remplace la clé refusée. OTTY n'enregistre que ce chemin ou ce nom, \
         jamais la clé.",
    ),
    (
        "de",
        "Ersetzt den abgelehnten Schlüssel. OTTY speichert nur diesen Pfad \
         oder Namen, nie den Schlüssel.",
    ),
    (
        "es",
        "Reemplaza la clave rechazada. OTTY guarda solo esta ruta o nombre, \
         nunca la clave.",
    ),
    (
        "pt-BR",
        "Substitui a chave recusada. O OTTY salva só este caminho ou nome, \
         nunca a chave.",
    ),
    (
        "ru",
        "Заменяет отклонённый ключ. OTTY сохраняет только этот путь или имя, \
         но не сам ключ.",
    ),
];

/// What an OpenRouter key starts with.
const KEY_PREFIX: &str = "sk-or-";

/// OpenRouter refused the account's key, one an agent keeps: the button
/// opens `keys_page`, where the user makes a new one, which they put
/// where the agent keeps the old one.
pub(crate) fn new_key(keys_page: &str) -> Need {
    Need {
        id: "new-key".to_string(),
        title: in_language(NEW_KEY_TITLE).to_string(),
        text: in_language(NEW_KEY_TEXT).to_string(),
        action: NeedAction::OpenPage(keys_page.to_string()),
    }
}

/// OpenRouter refused the account's key, one the user gave in OTTY: the
/// button opens a field for the new key, with no slot, so it takes the
/// old one's place, and `keys_page` at hand.
pub(crate) fn replace_key(keys_page: &str) -> Need {
    Need {
        id: "replace-key".to_string(),
        title: in_language(NEW_KEY_TITLE).to_string(),
        text: in_language(REPLACE_KEY_TEXT).to_string(),
        action: NeedAction::AskKey(KeyAsk {
            slot: None,
            label: in_language(KEY_LABEL).to_string(),
            hint: in_language(KEY_HINT).to_string(),
            prefix: Some(KEY_PREFIX.to_string()),
            page: Some(keys_page.to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tags of OTTY's interface languages.
    const LANGUAGES: [&str; 10] = [
        "en", "zh-CN", "zh-TW", "ja", "ko", "fr", "de", "es", "pt-BR", "ru",
    ];

    #[test]
    fn every_text_is_written_in_every_language_of_otty() {
        for texts in [
            NEW_KEY_TITLE,
            NEW_KEY_TEXT,
            REPLACE_KEY_TEXT,
            KEY_LABEL,
            KEY_HINT,
        ] {
            let tags: Vec<&str> = texts.iter().map(|(tag, _)| *tag).collect();

            assert_eq!(tags, LANGUAGES);
        }
    }
}
