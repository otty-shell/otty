//! What the part asks the user to do, in the language of OTTY's
//! interface: OTTY shows the need as it is, with the plugin's name, and
//! its button does the need's action. A text missing in a language falls
//! back to English.

use otty_plugin_sdk::env::in_language;
use otty_plugin_sdk::{Command, Need, NeedAction};

/// The title of [`not_logged_in`].
const NOT_LOGGED_IN_TITLE: &[(&str, &str)] = &[
    ("en", "Needs login"),
    ("zh-CN", "需要登录"),
    ("zh-TW", "需要登入"),
    ("ja", "ログインが必要"),
    ("ko", "로그인 필요"),
    ("fr", "Connexion requise"),
    ("de", "Anmeldung nötig"),
    ("es", "Requiere iniciar sesión"),
    ("pt-BR", "Requer login"),
    ("ru", "Нужен вход"),
];

/// The text of [`not_logged_in`].
const NOT_LOGGED_IN_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "Nobody is logged in with this config folder. OpenCode logs in in a \
         terminal tab. Refresh once it is done.",
    ),
    (
        "zh-CN",
        "没有人使用此配置文件夹登录。OpenCode 会在终端标签页中登录。完成后请刷新。",
    ),
    (
        "zh-TW",
        "沒有人以此設定資料夾登入。OpenCode 會在終端機分頁中登入。完成後請重新整理。",
    ),
    (
        "ja",
        "この設定フォルダーでログインしているユーザーはいません。\
         OpenCode がターミナルタブでログインします。終わったら更新してください。",
    ),
    (
        "ko",
        "이 설정 폴더로 로그인한 사용자가 없습니다. OpenCode가 터미널 탭에서 \
         로그인합니다. 완료 후 새로 고치세요.",
    ),
    (
        "fr",
        "Personne n’est connecté avec ce dossier de configuration. OpenCode \
         se connecte dans un onglet de terminal. Actualisez ensuite.",
    ),
    (
        "de",
        "Mit diesem Konfigurationsordner ist niemand angemeldet. OpenCode \
         meldet sich in einem Terminal-Tab an. Danach aktualisieren.",
    ),
    (
        "es",
        "Nadie ha iniciado sesión con esta carpeta de configuración. \
         OpenCode inicia sesión en una pestaña de terminal. Después, \
         actualiza.",
    ),
    (
        "pt-BR",
        "Ninguém entrou com esta pasta de configuração. O OpenCode faz login \
         em uma aba de terminal. Depois, atualize.",
    ),
    (
        "ru",
        "С этой папкой настроек никто не вошёл. OpenCode выполнит вход во \
         вкладке терминала. После этого обновите.",
    ),
];

/// The title of [`login_refused`].
const LOGIN_REFUSED_TITLE: &[(&str, &str)] = &[
    ("en", "Log in again"),
    ("zh-CN", "请重新登录"),
    ("zh-TW", "請重新登入"),
    ("ja", "もう一度ログインしてください"),
    ("ko", "다시 로그인하세요"),
    ("fr", "Reconnectez-vous"),
    ("de", "Erneut anmelden"),
    ("es", "Vuelve a iniciar sesión"),
    ("pt-BR", "Entre novamente"),
    ("ru", "Войдите снова"),
];

/// The text of [`login_refused`].
const LOGIN_REFUSED_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "A provider refused a login OpenCode keeps, or it expired. OpenCode \
         logs in again in a terminal tab. Refresh once it is done.",
    ),
    (
        "zh-CN",
        "提供方拒绝了 OpenCode 保存的登录，或登录已过期。OpenCode 会在终端标签页中重新登录。完成后请刷新。",
    ),
    (
        "zh-TW",
        "提供者拒絕了 OpenCode 保存的登入，或登入已過期。OpenCode 會在終端機分頁中重新登入。完成後請重新整理。",
    ),
    (
        "ja",
        "プロバイダーが OpenCode の保存したログインを拒否したか、ログインの期限が\
         切れました。OpenCode がターミナルタブでもう一度ログインします。終わったら更新してください。",
    ),
    (
        "ko",
        "제공자가 OpenCode에 저장된 로그인을 거부했거나 로그인이 만료되었습니다. \
         OpenCode가 터미널 탭에서 다시 로그인합니다. 완료 후 새로 고치세요.",
    ),
    (
        "fr",
        "Un fournisseur a refusé une connexion qu’OpenCode garde, ou elle a \
         expiré. OpenCode se reconnecte dans un onglet de terminal. \
         Actualisez ensuite.",
    ),
    (
        "de",
        "Ein Anbieter hat eine Anmeldung abgelehnt, die OpenCode aufbewahrt, \
         oder sie ist abgelaufen. OpenCode meldet sich in einem Terminal-Tab \
         erneut an. Danach aktualisieren.",
    ),
    (
        "es",
        "Un proveedor rechazó un inicio de sesión que guarda OpenCode, o \
         caducó. OpenCode vuelve a iniciar sesión en una pestaña de \
         terminal. Después, actualiza.",
    ),
    (
        "pt-BR",
        "Um provedor recusou um login que o OpenCode guarda, ou ele expirou. \
         O OpenCode entra novamente em uma aba de terminal. Depois, atualize.",
    ),
    (
        "ru",
        "Провайдер отклонил вход, который хранит OpenCode, или срок входа \
         истёк. OpenCode войдёт снова во вкладке терминала. После этого \
         обновите.",
    ),
];

/// Nobody is logged in with the profile: the button runs `login`,
/// OpenCode's own `opencode auth login`, in a terminal tab.
pub(crate) fn not_logged_in(login: Command) -> Need {
    Need {
        id: "not-logged-in".to_string(),
        title: in_language(NOT_LOGGED_IN_TITLE).to_string(),
        text: in_language(NOT_LOGGED_IN_TEXT).to_string(),
        action: NeedAction::Run(login),
    }
}

/// A provider refused a login the profile keeps, or it expired: the
/// button runs `login`, OpenCode's own `opencode auth login`, in a
/// terminal tab.
pub(crate) fn login_refused(login: Command) -> Need {
    Need {
        id: "login-refused".to_string(),
        title: in_language(LOGIN_REFUSED_TITLE).to_string(),
        text: in_language(LOGIN_REFUSED_TEXT).to_string(),
        action: NeedAction::Run(login),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, host};

    /// The tags of OTTY's interface languages.
    const LANGUAGES: [&str; 10] = [
        "en", "zh-CN", "zh-TW", "ja", "ko", "fr", "de", "es", "pt-BR", "ru",
    ];

    fn login() -> Command {
        Command {
            program: "opencode".to_string(),
            args: vec!["auth".to_string(), "login".to_string()],
            env: Vec::new(),
        }
    }

    #[test]
    fn every_text_is_written_in_every_language_of_otty() {
        for texts in [
            NOT_LOGGED_IN_TITLE,
            NOT_LOGGED_IN_TEXT,
            LOGIN_REFUSED_TITLE,
            LOGIN_REFUSED_TEXT,
        ] {
            let tags: Vec<&str> = texts.iter().map(|(tag, _)| *tag).collect();

            assert_eq!(tags, LANGUAGES);
        }
    }

    /// Owner, 2026-11-22: the words of the plugin take commas or a new
    /// sentence, never a semicolon.
    #[test]
    fn no_text_holds_a_semicolon() {
        for texts in [
            NOT_LOGGED_IN_TITLE,
            NOT_LOGGED_IN_TEXT,
            LOGIN_REFUSED_TITLE,
            LOGIN_REFUSED_TEXT,
        ] {
            for (tag, text) in texts {
                assert!(!text.contains([';', '\u{ff1b}']), "{tag}: {text}");
            }
        }
    }

    /// The need is written in the language of OTTY's interface, and in
    /// English for a language the plugin has no texts for.
    #[test]
    fn the_need_of_a_login_is_in_the_language_of_otty() {
        let russian =
            as_part(&host().with_language("ru"), || not_logged_in(login()));
        let unknown =
            as_part(&host().with_language("eo"), || not_logged_in(login()));

        assert_eq!(russian.title, "Нужен вход");
        assert!(
            russian
                .text
                .starts_with("С этой папкой настроек никто не вошёл"),
            "{}",
            russian.text
        );
        assert_eq!(unknown.title, "Needs login");
        assert!(
            unknown.text.starts_with("Nobody is logged in"),
            "{}",
            unknown.text
        );
        assert_eq!(russian.action, NeedAction::Run(login()));
    }
}
