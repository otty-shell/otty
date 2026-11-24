//! What the parts of the Anthropic plugin ask the user to do, in the
//! language of OTTY's interface: log in to Claude Code, run Claude Code so
//! it renews its login, and give the admin key of an organization's bill. OTTY shows
//! each need as it is, with the plugin's name, and its button does the
//! need's action. A language with no text of its own gets English.

use otty_plugin_sdk::env::in_language;
use otty_plugin_sdk::{Command, KeyAsk, Need, NeedAction};

/// The slot the admin key is given under: the part's own name for it.
pub(crate) const ADMIN_SLOT: &str = "admin";

/// What an Anthropic admin key starts with. An API key (`sk-ant-api`) or
/// the token of a Claude login (`sk-ant-oat`) does not.
pub(crate) const ADMIN_PREFIX: &str = "sk-ant-admin";

/// The page of the Anthropic Console where an admin of an organization
/// makes an admin key.
const ADMIN_PAGE: &str = "https://console.anthropic.com/settings/admin-keys";

/// The agent the texts name.
const AGENT_NAME: &str = "Claude Code";

/// The provider the texts name.
const PROVIDER_NAME: &str = "Anthropic";

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

/// The text of [`not_logged_in`]; `{agent}` is the agent's name.
const NOT_LOGGED_IN_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "Nobody is logged in with this config folder. {agent} logs in in a \
         terminal tab. Refresh once it is done.",
    ),
    (
        "zh-CN",
        "没有人使用此配置文件夹登录。{agent} 会在终端标签页中登录。完成后请刷新。",
    ),
    (
        "zh-TW",
        "沒有人以此設定資料夾登入。{agent} 會在終端機分頁中登入。完成後請重新整理。",
    ),
    (
        "ja",
        "この設定フォルダーでログインしているユーザーはいません。\
         {agent} がターミナルタブでログインします。終わったら更新してください。",
    ),
    (
        "ko",
        "이 설정 폴더로 로그인한 사용자가 없습니다. {agent}이(가) 터미널 탭에서 \
         로그인합니다. 완료 후 새로 고치세요.",
    ),
    (
        "fr",
        "Personne n’est connecté avec ce dossier de configuration. {agent} \
         se connecte dans un onglet de terminal. Actualisez ensuite.",
    ),
    (
        "de",
        "Mit diesem Konfigurationsordner ist niemand angemeldet. {agent} \
         meldet sich in einem Terminal-Tab an. Danach aktualisieren.",
    ),
    (
        "es",
        "Nadie ha iniciado sesión con esta carpeta de configuración. {agent} \
         inicia sesión en una pestaña de terminal. Después, actualiza.",
    ),
    (
        "pt-BR",
        "Ninguém entrou com esta pasta de configuração. O {agent} faz login \
         em uma aba de terminal. Depois, atualize.",
    ),
    (
        "ru",
        "С этой папкой настроек никто не вошёл. {agent} выполнит вход во \
         вкладке терминала. После этого обновите.",
    ),
];

/// The title of [`login_expired`].
const LOGIN_EXPIRED_TITLE: &[(&str, &str)] = &[
    ("en", "The login expired"),
    ("zh-CN", "登录已过期"),
    ("zh-TW", "登入已過期"),
    ("ja", "ログインの期限が切れました"),
    ("ko", "로그인이 만료되었습니다"),
    ("fr", "La connexion a expiré"),
    ("de", "Die Anmeldung ist abgelaufen"),
    ("es", "La sesión caducó"),
    ("pt-BR", "O login expirou"),
    ("ru", "Вход истёк"),
];

/// The text of [`login_expired`]; `{agent}` is the agent's name.
const LOGIN_EXPIRED_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "OTTY never renews a login it found in an agent. Open {agent}: it \
         renews the login or asks you to log in again, and OTTY picks the \
         new login up by itself.",
    ),
    (
        "zh-CN",
        "OTTY 从不续期它在代理中找到的登录。打开 {agent}：它会续期登录或请你重新登录，\
         OTTY 会自动读取新的登录。",
    ),
    (
        "zh-TW",
        "OTTY 從不續期它在代理中找到的登入。開啟 {agent}：它會續期登入或請你重新登入，\
         OTTY 會自動讀取新的登入。",
    ),
    (
        "ja",
        "OTTY はエージェントで見つけたログインを更新しません。{agent} を開くと、\
         ログインが更新されるか再ログインを求められます。新しいログインは OTTY が\
         自動で読み取ります。",
    ),
    (
        "ko",
        "OTTY는 에이전트에서 찾은 로그인을 갱신하지 않습니다. {agent}을(를) 열면 \
         로그인을 갱신하거나 다시 로그인하라고 묻고, OTTY는 새 로그인을 알아서 \
         읽습니다.",
    ),
    (
        "fr",
        "OTTY ne renouvelle jamais une connexion trouvée dans un agent. \
         Ouvrez {agent} : il renouvelle la connexion ou vous demande de vous \
         reconnecter, et OTTY reprend la nouvelle connexion tout seul.",
    ),
    (
        "de",
        "OTTY erneuert nie eine Anmeldung, die es in einem Agenten gefunden \
         hat. Öffne {agent}: Er erneuert die Anmeldung oder bittet dich, dich \
         neu anzumelden, und OTTY übernimmt die neue Anmeldung von selbst.",
    ),
    (
        "es",
        "OTTY nunca renueva una sesión que encontró en un agente. Abre \
         {agent}: renueva la sesión o te pide iniciarla de nuevo, y OTTY toma \
         la nueva sesión por sí solo.",
    ),
    (
        "pt-BR",
        "O OTTY nunca renova um login que encontrou em um agente. Abra o \
         {agent}: ele renova o login ou pede que você entre de novo, e o OTTY \
         pega o novo login sozinho.",
    ),
    (
        "ru",
        "OTTY никогда не продлевает вход, найденный у агента. Откройте \
         {agent}: он продлит вход или попросит войти заново, а OTTY сам \
         подхватит новый вход.",
    ),
];

/// The title of [`log_in_again`].
const LOG_IN_AGAIN_TITLE: &[(&str, &str)] = &[
    ("en", "Log in again"),
    ("zh-CN", "重新登录"),
    ("zh-TW", "重新登入"),
    ("ja", "もう一度ログイン"),
    ("ko", "다시 로그인"),
    ("fr", "Se reconnecter"),
    ("de", "Erneut anmelden"),
    ("es", "Volver a iniciar sesión"),
    ("pt-BR", "Fazer login novamente"),
    ("ru", "Войти снова"),
];

/// The text of [`log_in_again`]; `{agent}` is the agent's name and
/// `{provider}` the provider's.
const LOG_IN_AGAIN_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "{provider} refused OTTY's own login of this account. {agent} logs \
         in again in a terminal tab, in OTTY's private folder, and your \
         agents stay as they are.",
    ),
    (
        "zh-CN",
        "{provider} 拒绝了 OTTY 为此账户保存的登录。{agent} 会在终端标签页中、\
         在 OTTY 的私有文件夹里重新登录，你的代理保持不变。",
    ),
    (
        "zh-TW",
        "{provider} 拒絕了 OTTY 為此帳戶保存的登入。{agent} 會在終端機分頁中、\
         在 OTTY 的私人資料夾裡重新登入，你的代理維持不變。",
    ),
    (
        "ja",
        "{provider} が、このアカウントの OTTY 独自のログインを拒否しました。\
         {agent} がターミナルタブで OTTY の非公開フォルダーに再ログインします。\
         お使いのエージェントはそのままです。",
    ),
    (
        "ko",
        "{provider}이(가) 이 계정의 OTTY 전용 로그인을 거부했습니다. \
         {agent}이(가) 터미널 탭에서 OTTY 비공개 폴더에 다시 로그인하며, \
         사용 중인 에이전트는 그대로 유지됩니다.",
    ),
    (
        "fr",
        "{provider} a refusé la connexion propre à OTTY pour ce compte. \
         {agent} se reconnecte dans un onglet de terminal, dans le dossier \
         privé d’OTTY, et vos agents restent tels quels.",
    ),
    (
        "de",
        "{provider} hat OTTYs eigene Anmeldung für dieses Konto abgelehnt. \
         {agent} meldet sich in einem Terminal-Tab neu an, im privaten Ordner \
         von OTTY, und deine Agenten bleiben, wie sie sind.",
    ),
    (
        "es",
        "{provider} rechazó el inicio de sesión propio de OTTY para esta \
         cuenta. {agent} inicia sesión de nuevo en una pestaña de terminal, \
         en la carpeta privada de OTTY, y tus agentes quedan como están.",
    ),
    (
        "pt-BR",
        "{provider} recusou o login próprio do OTTY para esta conta. O \
         {agent} entra de novo em uma aba de terminal, na pasta privada do \
         OTTY, e seus agentes ficam como estão.",
    ),
    (
        "ru",
        "{provider} отклонил собственный вход OTTY для этого аккаунта. \
         {agent} выполнит вход заново во вкладке терминала, в личной папке \
         OTTY, а ваши агенты останутся как есть.",
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
        "zh-CN",
        "在有管理员密钥之前，OTTY 不显示此组织的金额。管理员密钥读取真实账单，\
         而没有智能体保存该密钥。",
    ),
    (
        "zh-TW",
        "在有管理員金鑰之前，OTTY 不會顯示此組織的金額。管理員金鑰讀取實際帳單，\
         而沒有代理程式保存該金鑰。",
    ),
    (
        "ja",
        "管理者キーが渡されるまで、OTTY はこの組織の金額を表示しません。\
         管理者キーで実際の請求を読み取りますが、どのエージェントも保持していません。",
    ),
    (
        "ko",
        "관리자 키가 있기 전까지 OTTY는 이 조직의 금액을 표시하지 않습니다. \
         관리자 키로 실제 청구서를 읽으며, 어떤 에이전트도 이를 보관하지 않습니다.",
    ),
    (
        "fr",
        "OTTY n'affiche aucun montant pour cette organisation tant qu'elle \
         n'a pas de clé d'administration, qui lit la vraie facture. Aucun \
         agent n'en garde.",
    ),
    (
        "de",
        "OTTY zeigt für diese Organisation keinen Betrag, bis sie einen \
         Admin-Schlüssel hat, der die echte Rechnung liest. Kein Agent \
         bewahrt einen auf.",
    ),
    (
        "es",
        "OTTY no muestra ningún importe de esta organización hasta que tenga \
         una clave de administrador, que lee la factura real. Ningún agente \
         guarda una.",
    ),
    (
        "pt-BR",
        "O OTTY não mostra nenhum valor desta organização até que ela tenha \
         uma chave de administrador, que lê a fatura real. Nenhum agente \
         guarda uma.",
    ),
    (
        "ru",
        "OTTY не показывает деньги этой организации, пока у неё нет ключа \
         администратора, который читает настоящий счёт. Ни один агент его не \
         хранит.",
    ),
];

/// The title of [`new_bill_key`].
const NEW_BILL_KEY_TITLE: &[(&str, &str)] = &[
    ("en", "The admin key was refused"),
    ("zh-CN", "管理员密钥被拒绝"),
    ("zh-TW", "管理員金鑰遭拒"),
    ("ja", "管理者キーが拒否されました"),
    ("ko", "관리자 키가 거부되었습니다"),
    ("fr", "La clé d'administration a été refusée"),
    ("de", "Der Admin-Schlüssel wurde abgelehnt"),
    ("es", "La clave de administrador fue rechazada"),
    ("pt-BR", "A chave de administrador foi recusada"),
    ("ru", "Ключ администратора отклонён"),
];

/// The text of [`new_bill_key`]; `{provider}` is the provider's name.
const NEW_BILL_KEY_TEXT: &[(&str, &str)] = &[
    (
        "en",
        "{provider} refused the admin key: it is wrong, expired or revoked. \
         Make a new one on its page and give it here.",
    ),
    (
        "zh-CN",
        "{provider} 拒绝了该管理员密钥：它有误、已过期或已被吊销。请在其页面上创建新密钥，\
         并在此处提供。",
    ),
    (
        "zh-TW",
        "{provider} 拒絕了此管理員金鑰：它有誤、已過期或已被撤銷。請在其頁面上建立新金鑰，\
         並在此處提供。",
    ),
    (
        "ja",
        "{provider} が管理者キーを拒否しました。キーが間違っているか、期限切れか、\
         取り消されています。ページで新しいキーを作り、ここで指定してください。",
    ),
    (
        "ko",
        "{provider}이(가) 관리자 키를 거부했습니다. 키가 잘못되었거나 만료되었거나 \
         폐기되었습니다. 페이지에서 새 키를 만들어 여기에 지정하세요.",
    ),
    (
        "fr",
        "{provider} a refusé la clé d'administration : elle est fausse, \
         expirée ou révoquée. Créez-en une nouvelle sur sa page et \
         indiquez-la ici.",
    ),
    (
        "de",
        "{provider} hat den Admin-Schlüssel abgelehnt: Er ist falsch, \
         abgelaufen oder widerrufen. Erstelle auf der Seite einen neuen und \
         gib ihn hier an.",
    ),
    (
        "es",
        "{provider} rechazó la clave de administrador: es incorrecta, caducó \
         o fue revocada. Crea una nueva en su página e indícala aquí.",
    ),
    (
        "pt-BR",
        "O {provider} recusou a chave de administrador: ela está errada, \
         expirou ou foi revogada. Crie uma nova na página dele e informe-a \
         aqui.",
    ),
    (
        "ru",
        "{provider} отклонил ключ администратора: он неверный, истёк или \
         отозван. Создайте новый на его странице и укажите его здесь.",
    ),
];

/// The name of the field of the admin key.
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

/// The line under the field of the admin key.
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

/// Nobody is logged in with the profile: the button runs `login`,
/// `claude auth login --claudeai` in the profile's config folder, in a
/// terminal tab.
pub(crate) fn not_logged_in(login: Command) -> Need {
    Need {
        id: "not-logged-in".to_string(),
        title: in_language(NOT_LOGGED_IN_TITLE).to_string(),
        text: in_language(NOT_LOGGED_IN_TEXT).replace("{agent}", AGENT_NAME),
        action: NeedAction::Run(login),
    }
}

/// The Claude login expired or Anthropic refused it, and the plugin never
/// renews it: the button runs `claude`, Claude Code itself, which renews
/// the login or asks to log in again.
pub(crate) fn login_expired(claude: Command) -> Need {
    Need {
        id: "login-expired".to_string(),
        title: in_language(LOGIN_EXPIRED_TITLE).to_string(),
        text: in_language(LOGIN_EXPIRED_TEXT).replace("{agent}", AGENT_NAME),
        action: NeedAction::Run(claude),
    }
}

/// Anthropic refused OTTY's own login, or Claude Code can not renew it
/// there (spec 2026-11-23-accounts-apart-from-profiles, Decision 9): the
/// button runs `login`, Claude Code's login in OTTY's own folder, in a
/// terminal tab. Only the account shows it, there is no profile.
pub(crate) fn log_in_again(login: Command) -> Need {
    Need {
        id: "log-in-again".to_string(),
        title: in_language(LOG_IN_AGAIN_TITLE).to_string(),
        text: in_language(LOG_IN_AGAIN_TEXT)
            .replace("{agent}", AGENT_NAME)
            .replace("{provider}", PROVIDER_NAME),
        action: NeedAction::Run(login),
    }
}

/// The bill of the organization needs an admin key, which the user gives
/// once: OTTY keeps where they keep it and gives it back under
/// [`ADMIN_SLOT`].
pub(crate) fn bill_key() -> Need {
    Need {
        id: "bill-key".to_string(),
        title: in_language(BILL_KEY_TITLE).to_string(),
        text: in_language(BILL_KEY_TEXT).to_string(),
        action: NeedAction::AskKey(bill_key_ask()),
    }
}

/// Anthropic refused the admin key the bill was read with: the button asks
/// for a new one under [`ADMIN_SLOT`].
pub(crate) fn new_bill_key() -> Need {
    Need {
        id: "new-bill-key".to_string(),
        title: in_language(NEW_BILL_KEY_TITLE).to_string(),
        text: in_language(NEW_BILL_KEY_TEXT)
            .replace("{provider}", PROVIDER_NAME),
        action: NeedAction::AskKey(bill_key_ask()),
    }
}

/// The field of the admin key: its slot, its name and hint, what it
/// starts with and the page where it is made.
fn bill_key_ask() -> KeyAsk {
    KeyAsk {
        slot: Some(ADMIN_SLOT.to_string()),
        label: in_language(BILL_KEY_LABEL).to_string(),
        hint: in_language(BILL_KEY_HINT).to_string(),
        prefix: Some(ADMIN_PREFIX.to_string()),
        page: Some(ADMIN_PAGE.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::env::pick_language;

    use super::*;
    use crate::testing::{as_part, host};

    /// The tags of OTTY's interface languages.
    const LANGUAGES: [&str; 10] = [
        "en", "zh-CN", "zh-TW", "ja", "ko", "fr", "de", "es", "pt-BR", "ru",
    ];

    fn claude() -> Command {
        Command {
            program: "claude".to_string(),
            args: Vec::new(),
            env: Vec::new(),
        }
    }

    /// Every need, made in the language `tag`.
    fn needs_in(tag: &str) -> Vec<Need> {
        let host = host().with_language(tag);

        as_part(&host, || {
            vec![
                not_logged_in(claude()),
                login_expired(claude()),
                bill_key(),
                new_bill_key(),
                log_in_again(claude()),
            ]
        })
    }

    /// Issue 21 of spec 2026-11-23-accounts-apart-from-profiles: since
    /// issue 01 OTTY shows no estimate from the tokens, so the need of the
    /// admin key says the organization shows no money until it has one, in
    /// every language.
    #[test]
    fn the_admin_key_need_promises_no_estimate() {
        let estimate =
            ["estimat", "估算", "見積", "추정", "Schätzung", "оценк"];

        for (tag, text) in BILL_KEY_TEXT {
            for word in estimate {
                assert!(!text.contains(word), "{tag}: {text}");
            }
        }
        let english = pick_language("en", BILL_KEY_TEXT);
        assert!(english.contains("shows no money"), "{english}");
    }

    #[test]
    fn every_text_is_written_in_every_language_of_otty() {
        for texts in [
            NOT_LOGGED_IN_TITLE,
            NOT_LOGGED_IN_TEXT,
            LOGIN_EXPIRED_TITLE,
            LOGIN_EXPIRED_TEXT,
            BILL_KEY_TITLE,
            BILL_KEY_TEXT,
            NEW_BILL_KEY_TITLE,
            NEW_BILL_KEY_TEXT,
            BILL_KEY_LABEL,
            BILL_KEY_HINT,
            LOG_IN_AGAIN_TITLE,
            LOG_IN_AGAIN_TEXT,
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
            LOGIN_EXPIRED_TITLE,
            LOGIN_EXPIRED_TEXT,
            BILL_KEY_TITLE,
            BILL_KEY_TEXT,
            NEW_BILL_KEY_TITLE,
            NEW_BILL_KEY_TEXT,
            BILL_KEY_LABEL,
            BILL_KEY_HINT,
            LOG_IN_AGAIN_TITLE,
            LOG_IN_AGAIN_TEXT,
        ] {
            for (tag, text) in texts {
                assert!(!text.contains([';', '\u{ff1b}']), "{tag}: {text}");
            }
        }
    }

    #[test]
    fn every_language_names_the_agent_and_the_provider() {
        for language in LANGUAGES {
            for texts in [NOT_LOGGED_IN_TEXT, LOGIN_EXPIRED_TEXT] {
                let text = pick_language(language, texts);
                assert!(text.contains("{agent}"), "{language}: {text}");
            }
            let refused = pick_language(language, NEW_BILL_KEY_TEXT);
            assert!(refused.contains("{provider}"), "{language}: {refused}");
            let again = pick_language(language, LOG_IN_AGAIN_TEXT);
            assert!(again.contains("{agent}"), "{language}: {again}");
            assert!(again.contains("{provider}"), "{language}: {again}");
        }
    }

    #[test]
    fn the_ids_are_stable_and_differ() {
        let ids: Vec<String> =
            needs_in("en").into_iter().map(|need| need.id).collect();

        assert_eq!(
            ids,
            [
                "not-logged-in",
                "login-expired",
                "bill-key",
                "new-bill-key",
                "log-in-again"
            ]
        );
    }

    #[test]
    fn the_texts_come_in_the_language_of_otty() {
        let needs = needs_in("ru");

        assert_eq!(needs[0].title, "Нужен вход");
        assert!(needs[0].text.contains("Claude Code выполнит вход"));
        assert_eq!(needs[1].title, "Вход истёк");
        assert!(needs[1].text.contains("Откройте Claude Code"));
        assert_eq!(
            needs[2].title,
            "Для настоящего счёта нужен ключ администратора"
        );
        assert!(needs[3].text.starts_with("Anthropic отклонил"));
        let NeedAction::AskKey(ask) = &needs[2].action else {
            panic!("a key to ask for: {:?}", needs[2]);
        };
        assert_eq!(ask.label, "Ключ администратора");
    }

    #[test]
    fn an_unknown_language_gets_english() {
        let needs = needs_in("xx");

        assert_eq!(needs[0].title, "Needs login");
        assert_eq!(needs[1].title, "The login expired");
        assert!(needs[1].text.contains("Open Claude Code"));
        assert_eq!(needs[3].title, "The admin key was refused");
        assert_eq!(needs[4].title, "Log in again");
        assert!(needs[4].text.contains("Anthropic refused OTTY's own login"));
    }

    /// Issue 09 of spec 2026-11-23-accounts-apart-from-profiles: the need
    /// of a refused own login runs the login it is given.
    #[test]
    fn log_in_again_runs_the_login_it_is_given() {
        let needs = needs_in("en");

        assert_eq!(needs[4].action, NeedAction::Run(claude()));
    }

    #[test]
    fn both_bill_key_needs_ask_for_the_same_field() {
        let needs = needs_in("en");

        assert_eq!(needs[2].action, needs[3].action);
        let NeedAction::AskKey(ask) = &needs[2].action else {
            panic!("a key to ask for: {:?}", needs[2]);
        };
        assert_eq!(ask.slot.as_deref(), Some(ADMIN_SLOT));
        assert_eq!(ask.prefix.as_deref(), Some(ADMIN_PREFIX));
        assert_eq!(ask.page.as_deref(), Some(ADMIN_PAGE));
    }
}
