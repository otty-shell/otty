//! What the part asks the user to do, in the language of OTTY's
//! interface: OTTY shows the need as it is, with the plugin's name, and
//! its button does the need's action. A text missing in a language falls
//! back to English.

use otty_plugin_sdk::env::in_language;
use otty_plugin_sdk::{Need, NeedAction};

/// The title of [`new_key`].
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

/// OpenRouter refused the account's key: the button opens `keys_page`,
/// where the user makes a new one. The key may be one the user added by
/// hand or one an agent keeps, so the need says only to put the new key
/// where the old one is.
pub(crate) fn new_key(keys_page: &str) -> Need {
    Need {
        id: "new-key".to_string(),
        title: in_language(NEW_KEY_TITLE).to_string(),
        text: in_language(NEW_KEY_TEXT).to_string(),
        action: NeedAction::OpenPage(keys_page.to_string()),
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
        for texts in [NEW_KEY_TITLE, NEW_KEY_TEXT] {
            let tags: Vec<&str> = texts.iter().map(|(tag, _)| *tag).collect();

            assert_eq!(tags, LANGUAGES);
        }
    }
}
