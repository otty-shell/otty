//! What the parts ask the user to do, in the language of OTTY's
//! interface: OTTY shows each need as it is, with the plugin's name, and
//! its button does the need's action. A text missing in a language falls
//! back to English.

use otty_plugin_sdk::env::in_language;
use otty_plugin_sdk::{Command, KeyAsk, Need, NeedAction};

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
        "Nobody is logged in with this config folder. The agent logs in in \
         a terminal tab; refresh once it is done.",
    ),
    (
        "zh-CN",
        "没有人使用此配置文件夹登录。智能体会在终端标签页中登录；完成后请刷新。",
    ),
    (
        "zh-TW",
        "沒有人以此設定資料夾登入。代理會在終端機分頁中登入；完成後請重新整理。",
    ),
    (
        "ja",
        "この設定フォルダーでログインしているユーザーはいません。\
         エージェントがターミナルタブでログインします。終わったら更新してください。",
    ),
    (
        "ko",
        "이 설정 폴더로 로그인한 사용자가 없습니다. 에이전트가 터미널 탭에서 \
         로그인합니다. 완료 후 새로 고치세요.",
    ),
    (
        "fr",
        "Personne n’est connecté avec ce dossier de configuration. L’agent \
         se connecte dans un onglet de terminal ; actualisez ensuite.",
    ),
    (
        "de",
        "Mit diesem Konfigurationsordner ist niemand angemeldet. Der Agent \
         meldet sich in einem Terminal-Tab an; danach aktualisieren.",
    ),
    (
        "es",
        "Nadie ha iniciado sesión con esta carpeta de configuración. El \
         agente inicia sesión en una pestaña de terminal; después, actualiza.",
    ),
    (
        "pt-BR",
        "Ninguém entrou com esta pasta de configuração. O agente faz login \
         em uma aba de terminal; depois, atualize.",
    ),
    (
        "ru",
        "С этой папкой настроек никто не вошёл. Агент выполнит вход во \
         вкладке терминала; после этого обновите.",
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
        "The provider refused the login of this config folder, or it \
         expired. The agent logs in again in a terminal tab; refresh once \
         it is done.",
    ),
    (
        "zh-CN",
        "提供方拒绝了此配置文件夹的登录，或登录已过期。智能体会在终端标签页中重新登录；完成后请刷新。",
    ),
    (
        "zh-TW",
        "提供者拒絕了此設定資料夾的登入，或登入已過期。代理會在終端機分頁中重新登入；完成後請重新整理。",
    ),
    (
        "ja",
        "プロバイダーがこの設定フォルダーのログインを拒否したか、ログインの期限が\
         切れました。エージェントがターミナルタブでもう一度ログインします。終わったら更新してください。",
    ),
    (
        "ko",
        "제공자가 이 설정 폴더의 로그인을 거부했거나 로그인이 만료되었습니다. \
         에이전트가 터미널 탭에서 다시 로그인합니다. 완료 후 새로 고치세요.",
    ),
    (
        "fr",
        "Le fournisseur a refusé la connexion de ce dossier de configuration, \
         ou elle a expiré. L’agent se reconnecte dans un onglet de terminal ; \
         actualisez ensuite.",
    ),
    (
        "de",
        "Der Anbieter hat die Anmeldung dieses Konfigurationsordners \
         abgelehnt, oder sie ist abgelaufen. Der Agent meldet sich in einem \
         Terminal-Tab erneut an; danach aktualisieren.",
    ),
    (
        "es",
        "El proveedor rechazó el inicio de sesión de esta carpeta de \
         configuración, o caducó. El agente vuelve a iniciar sesión en una \
         pestaña de terminal; después, actualiza.",
    ),
    (
        "pt-BR",
        "O provedor recusou o login desta pasta de configuração, ou ele \
         expirou. O agente entra novamente em uma aba de terminal; depois, \
         atualize.",
    ),
    (
        "ru",
        "Провайдер отклонил вход этой папки настроек, или срок входа истёк. \
         Агент войдёт снова во вкладке терминала; после этого обновите.",
    ),
];

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

/// The text of [`new_key`]; `{provider}` is the provider's name.
const NEW_KEY_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "{provider} refused the key: it is wrong, expired or revoked. Make a \
         new key on its page and put it where the agent reads it.",
    ),
    (
        "zh-CN",
        "{provider} 拒绝了该密钥：它有误、已过期或已被吊销。请在其页面上创建新密钥，\
         并放到智能体读取的位置。",
    ),
    (
        "zh-TW",
        "{provider} 拒絕了此金鑰：它有誤、已過期或已被撤銷。請在其頁面上建立新金鑰，\
         並放到代理讀取的位置。",
    ),
    (
        "ja",
        "{provider} がキーを拒否しました。キーが間違っているか、期限切れか、\
         取り消されています。ページで新しいキーを作り、エージェントが読む場所に置いてください。",
    ),
    (
        "ko",
        "{provider}이(가) 키를 거부했습니다. 키가 잘못되었거나 만료되었거나 \
         폐기되었습니다. 페이지에서 새 키를 만들어 에이전트가 읽는 곳에 두세요.",
    ),
    (
        "fr",
        "{provider} a refusé la clé : elle est fausse, expirée ou révoquée. \
         Créez une nouvelle clé sur sa page et placez-la là où l’agent la lit.",
    ),
    (
        "de",
        "{provider} hat den Schlüssel abgelehnt: Er ist falsch, abgelaufen \
         oder widerrufen. Erstelle auf der Seite einen neuen Schlüssel und \
         lege ihn dort ab, wo der Agent ihn liest.",
    ),
    (
        "es",
        "{provider} rechazó la clave: es incorrecta, caducó o fue revocada. \
         Crea una clave nueva en su página y ponla donde el agente la lee.",
    ),
    (
        "pt-BR",
        "O {provider} recusou a chave: ela está errada, expirou ou foi \
         revogada. Crie uma chave nova na página dele e coloque-a onde o \
         agente a lê.",
    ),
    (
        "ru",
        "{provider} отклонил ключ: он неверный, истёк или отозван. Создайте \
         новый ключ на его странице и положите туда, где его читает агент.",
    ),
];

/// The text of [`replace_key`]; `{provider}` is the provider's name.
const REPLACE_KEY_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "{provider} refused the key: it is wrong, expired or revoked. Make a \
         new key on its page and give it here in place of the old one.",
    ),
    (
        "zh-CN",
        "{provider} 拒绝了该密钥：它有误、已过期或已被吊销。请在其页面上创建新密钥，\
         并在此处提供它来替换旧密钥。",
    ),
    (
        "zh-TW",
        "{provider} 拒絕了此金鑰：它有誤、已過期或已被撤銷。請在其頁面上建立新金鑰，\
         並在此處提供它來取代舊金鑰。",
    ),
    (
        "ja",
        "{provider} がキーを拒否しました。キーが間違っているか、期限切れか、\
         取り消されています。ページで新しいキーを作り、古いキーの代わりにここで指定してください。",
    ),
    (
        "ko",
        "{provider}이(가) 키를 거부했습니다. 키가 잘못되었거나 만료되었거나 \
         폐기되었습니다. 페이지에서 새 키를 만들어 이전 키 대신 여기에 지정하세요.",
    ),
    (
        "fr",
        "{provider} a refusé la clé : elle est fausse, expirée ou révoquée. \
         Créez une nouvelle clé sur sa page et indiquez-la ici à la place de \
         l’ancienne.",
    ),
    (
        "de",
        "{provider} hat den Schlüssel abgelehnt: Er ist falsch, abgelaufen \
         oder widerrufen. Erstelle auf der Seite einen neuen Schlüssel und \
         gib ihn hier an Stelle des alten an.",
    ),
    (
        "es",
        "{provider} rechazó la clave: es incorrecta, caducó o fue revocada. \
         Crea una clave nueva en su página e indícala aquí en lugar de la \
         anterior.",
    ),
    (
        "pt-BR",
        "O {provider} recusou a chave: ela está errada, expirou ou foi \
         revogada. Crie uma chave nova na página dele e informe-a aqui no \
         lugar da antiga.",
    ),
    (
        "ru",
        "{provider} отклонил ключ: он неверный, истёк или отозван. Создайте \
         новый ключ на его странице и укажите его здесь вместо старого.",
    ),
];

/// The name of the field of [`replace_key`]; `{provider}` is the
/// provider's name.
const KEY_LABEL: &[(&str, &str)] = &[
    ("en", "{provider} key"),
    ("zh-CN", "{provider} 密钥"),
    ("zh-TW", "{provider} 金鑰"),
    ("ja", "{provider} キー"),
    ("ko", "{provider} 키"),
    ("fr", "Clé {provider}"),
    ("de", "{provider}-Schlüssel"),
    ("es", "Clave de {provider}"),
    ("pt-BR", "Chave do {provider}"),
    ("ru", "Ключ {provider}"),
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

/// The title of [`bill_key`].
const BILL_KEY_TITLE: &[(&str, &str)] = &[
    ("en", "The real bill needs an admin key"),
    ("zh-CN", "真实账单需要管理员密钥"),
    ("zh-TW", "實際帳單需要管理員金鑰"),
    ("ja", "実際の請求には管理者キーが必要です"),
    ("ko", "실제 청구서에는 관리자 키가 필요합니다"),
    ("fr", "La vraie facture demande une clé d'administration"),
    ("de", "Die echte Rechnung braucht einen Admin-Schlüssel"),
    ("es", "La factura real necesita una clave de administrador"),
    (
        "pt-BR",
        "A fatura real precisa de uma chave de administrador",
    ),
    ("ru", "Для настоящего счёта нужен ключ администратора"),
];

/// The text of [`bill_key`].
const BILL_KEY_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "OTTY shows an estimate from the tokens. The bill of the whole \
         organization needs an admin key, and no agent keeps one.",
    ),
    (
        "zh-CN",
        "OTTY 显示的是根据令牌估算的金额。整个组织的账单需要管理员密钥，而没有智能体保存该密钥。",
    ),
    (
        "zh-TW",
        "OTTY 顯示的是依權杖估算的金額。整個組織的帳單需要管理員金鑰，而沒有代理程式保存該金鑰。",
    ),
    (
        "ja",
        "OTTY はトークンから見積もりを表示しています。組織全体の請求には管理者キーが\
         必要ですが、どのエージェントも保持していません。",
    ),
    (
        "ko",
        "OTTY는 토큰으로 추정한 금액을 보여 줍니다. 조직 전체의 청구서에는 관리자 \
         키가 필요하지만, 어떤 에이전트도 이를 보관하지 않습니다.",
    ),
    (
        "fr",
        "OTTY affiche une estimation à partir des jetons. La facture de toute \
         l'organisation demande une clé d'administration, et aucun agent \
         n'en garde.",
    ),
    (
        "de",
        "OTTY zeigt eine Schätzung aus den Tokens. Die Rechnung der ganzen \
         Organisation braucht einen Admin-Schlüssel, und kein Agent bewahrt \
         einen auf.",
    ),
    (
        "es",
        "OTTY muestra una estimación a partir de los tokens. La factura de \
         toda la organización necesita una clave de administrador, y ningún \
         agente guarda una.",
    ),
    (
        "pt-BR",
        "O OTTY mostra uma estimativa a partir dos tokens. A fatura de toda \
         a organização precisa de uma chave de administrador, e nenhum \
         agente guarda uma.",
    ),
    (
        "ru",
        "OTTY показывает оценку по токенам. Для счёта всей организации нужен \
         ключ администратора, а ни один агент его не хранит.",
    ),
];

/// The name of the field of the key of the bill.
const BILL_KEY_LABEL: &[(&str, &str)] = &[
    ("en", "Admin key"),
    ("zh-CN", "管理员密钥"),
    ("zh-TW", "管理員金鑰"),
    ("ja", "管理者キー"),
    ("ko", "관리자 키"),
    ("fr", "Clé d'administration"),
    ("de", "Admin-Schlüssel"),
    ("es", "Clave de administrador"),
    ("pt-BR", "Chave de administrador"),
    ("ru", "Ключ администратора"),
];

/// The line under the field of the key of the bill.
const BILL_KEY_HINT: &[(&str, &str)] = &[
    (
        "en",
        "Reads the real bill of the whole organization. OTTY saves only this \
         path or name, never the key.",
    ),
    (
        "zh-CN",
        "读取整个组织的真实账单。OTTY 只保存这个路径或名称，从不保存密钥。",
    ),
    (
        "zh-TW",
        "讀取整個組織的真實帳單。OTTY 只儲存這個路徑或名稱，從不儲存金鑰。",
    ),
    (
        "ja",
        "組織全体の実際の請求額を読み取ります。OTTY が保存するのはこのパスか名前\
         だけで、キーは保存しません。",
    ),
    (
        "ko",
        "조직 전체의 실제 청구액을 읽습니다. OTTY는 이 경로나 이름만 저장하고 \
         키는 저장하지 않습니다.",
    ),
    (
        "fr",
        "Lit la vraie facture de toute l'organisation. OTTY n'enregistre que \
         ce chemin ou ce nom, jamais la clé.",
    ),
    (
        "de",
        "Liest die echte Rechnung der ganzen Organisation. OTTY speichert nur \
         diesen Pfad oder Namen, nie den Schlüssel.",
    ),
    (
        "es",
        "Lee la factura real de toda la organización. OTTY guarda solo esta \
         ruta o nombre, nunca la clave.",
    ),
    (
        "pt-BR",
        "Lê a fatura real de toda a organização. O OTTY salva só este caminho \
         ou nome, nunca a chave.",
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

/// A provider refused the login of the profile, or it expired: the button
/// runs `login`, the agent's login command, in a terminal tab.
pub(crate) fn login_refused(login: Command) -> Need {
    Need {
        id: "login-refused".to_string(),
        title: in_language(LOGIN_REFUSED_TITLE).to_string(),
        text: in_language(LOGIN_REFUSED_TEXT).to_string(),
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

/// `provider` refused the account's key, one the user gave in OTTY: the
/// button opens a field for the new key with no slot, so it takes the old
/// one's place, and `keys_page` at hand.
pub(crate) fn replace_key(provider: &str, keys_page: &str) -> Need {
    Need {
        id: "replace-key".to_string(),
        title: in_language(NEW_KEY_TITLE).to_string(),
        text: in_language(REPLACE_KEY_TEXT).replace("{provider}", provider),
        action: NeedAction::AskKey(KeyAsk {
            slot: None,
            label: in_language(KEY_LABEL).replace("{provider}", provider),
            hint: in_language(KEY_HINT).to_string(),
            prefix: None,
            page: Some(keys_page.to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::env::pick_language;

    use super::*;

    /// The tags of OTTY's interface languages.
    const LANGUAGES: [&str; 10] = [
        "en", "zh-CN", "zh-TW", "ja", "ko", "fr", "de", "es", "pt-BR", "ru",
    ];

    #[test]
    fn every_text_is_written_in_every_language_of_otty() {
        for texts in [
            NOT_LOGGED_IN_TITLE,
            NOT_LOGGED_IN_TEXT,
            LOGIN_REFUSED_TITLE,
            LOGIN_REFUSED_TEXT,
            NEW_KEY_TITLE,
            NEW_KEY_TEXT,
            REPLACE_KEY_TEXT,
            KEY_LABEL,
            KEY_HINT,
            BILL_KEY_TITLE,
            BILL_KEY_TEXT,
            BILL_KEY_LABEL,
            BILL_KEY_HINT,
        ] {
            let tags: Vec<&str> = texts.iter().map(|(tag, _)| *tag).collect();

            assert_eq!(tags, LANGUAGES);
        }
    }

    #[test]
    fn the_name_of_the_provider_goes_into_every_refusal() {
        for language in LANGUAGES {
            for texts in [NEW_KEY_TEXT, REPLACE_KEY_TEXT, KEY_LABEL] {
                let text = pick_language(language, texts);

                assert!(text.contains("{provider}"), "{language}: {text}");
            }
        }
    }
}
