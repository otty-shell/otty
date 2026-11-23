# How OpenCode signs in to ChatGPT and what it keeps

Researched on 2026-11-23.

Sources:

- OpenCode source at commit [`23fd3da`](https://github.com/anomalyco/opencode/tree/23fd3da78601f3afd94b9845db815bbf11edf408) (2026-11-22). The repository moved: `github.com/sst/opencode` now redirects to `github.com/anomalyco/opencode` (default branch `dev`).
- OpenAI's "Sign in with ChatGPT" (SIWC) token-sharing guide: [overview](https://developers.openai.com/siwc/token-sharing-open-source), [sign in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in), [token reference](https://developers.openai.com/siwc/token-sharing-open-source/token-reference), [accounts and sessions](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions), [models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference).
- OpenAI's sample code: [openai/sign-in-with-chatgpt-devkit](https://github.com/openai/sign-in-with-chatgpt-devkit), file [`packages/local/src/oauth.ts`](https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/packages/local/src/oauth.ts).
- The public discovery document [`https://auth.openai.com/.well-known/openid-configuration`](https://auth.openai.com/.well-known/openid-configuration) (fetched without credentials).
- Codex CLI source: [openai/codex](https://github.com/openai/codex) at `44848822`.

Each section separates **what the code or docs show** from **guesses**.

Short links below:

- `chatgpt.ts` = [`packages/core/src/plugin/provider/chatgpt.ts`](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts)
- `openai.ts` = [`packages/core/src/plugin/provider/openai.ts`](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/openai.ts)

## 1. The login methods

The `openai` integration has three OAuth methods. All three write rows with `integration_id = "openai"` into the `credential` table.

| Method id | Label in OpenCode | Plugin | Client id |
| --- | --- | --- | --- |
| `chatgpt-token-sharing` | "Sign in with ChatGPT" | `opencode.provider.chatgpt` (`chatgpt.ts`) | issued per user by OpenAI (dynamic registration) |
| `chatgpt-browser` | "Codex browser (legacy)" | `opencode.provider.openai` (`openai.ts`) | `app_EMoamEEZ73f0CkXaXp7hrann` (the Codex CLI client id) |
| `chatgpt-headless` | "Codex device code (legacy)" | `opencode.provider.openai` (`openai.ts`) | `app_EMoamEEZ73f0CkXaXp7hrann` |

The rows you saw (`methodID: "chatgpt-token-sharing"`, `metadata: {clientID, scopes}`) are the first method.

### 1a. `chatgpt-token-sharing` ("Sign in with ChatGPT")

What the code shows ([`chatgpt.ts` L23-L33](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L23-L33), [L105-L213](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L105-L213), [L570-L594](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L570-L594)):

1. **Host id.** OpenCode reads `chatgpt-agent-host-id` from the plugin's key-value storage. If missing, it makes `urn:uuid:<random v4>` and saves it ([L115-L117](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L115-L117)). This is the `kv` row you found. The key prefix is `plugin:` plus the plugin id `opencode.provider.chatgpt` written as 4 hex digits per character, then `:` ([`plugin/host.ts` L663-L667](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/host.ts#L663-L667)). There is one host id per OpenCode database, shared by all ChatGPT logins in it.
2. **Local callback.** It makes a PKCE pair, a `state` and a `nonce`, and starts an HTTP listener on `127.0.0.1` on a random free port, path `/auth/callback`.
3. **Browser.** It opens `https://auth.openai.com/api/accounts/authorize` with:
   - `client_id` = the saved client id of the current ChatGPT login, or `dynamic_agent_client` on the first sign-in;
   - `agent_name_hint=OpenCode` (first sign-in only);
   - `ext_agent_host_id=<host id>`;
   - `response_type=code`, `redirect_uri=http://127.0.0.1:<port>/auth/callback`;
   - `scope=openid profile email offline_access resource.invoke chatgpt.tokens.use.direct`;
   - `resource=https://api.openai.com/v1`;
   - `state`, `nonce`, `code_challenge_method=S256`, `code_challenge`.
   - No `id_token_hint` and no `login_hint`; `force_reconsent` is commented out.
4. **Callback.** OpenAI redirects back with `code`, `state` and, on a first sign-in, the newly issued `client_id`. If a saved client id exists and the callback brings a different one, the sign-in fails ([L181-L187](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L181-L187)). The user can also paste the redirect URL by hand.
5. **Token exchange.** A form POST to `https://auth.openai.com/api/accounts/oauth/token` with `grant_type=authorization_code`, `client_id`, `code`, `code_verifier`, `redirect_uri`, `resource=https://api.openai.com/v1` ([L392-L405](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L392-L405)). No client secret.
6. **Checks.** The granted `scope` must contain `chatgpt.tokens.use.direct`. The response must contain an `id_token`. OpenCode checks it against `https://auth.openai.com/.well-known/jwks.json`: issuer, audience = the client id, RS256, `exp`, `sub` present, `nonce` matches ([L189-L196](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L189-L196), [L509-L529](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L509-L529)).
7. **What is saved** ([L544-L553](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L544-L553)): `{type: "oauth", methodID: "chatgpt-token-sharing", refresh, access, expires: now + expires_in (default 3600 s), metadata: {clientID, scopes}}`. That is exactly the JSON in the `value` column.

**The `id_token` is discarded.** It is only verified. OpenCode does not keep it, nor its `sub`, `email` or `name`. So the database has no email and no stable account identity for this login, only the opaque access-token claims.

The row's `label` is not taken from the account. It is the integration name ("OpenAI"), or "OpenAI 2", "OpenAI 3" … when that name is taken ([`integration.ts` L378-L387](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/integration.ts#L378-L387), [L486-L489](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/integration.ts#L486-L489)). The ChatGPT plugin has no `label` hook. The OpenCode Console plugin does, and uses the org name.

**What "token sharing" and "agent host id" mean** (from OpenAI's guide):

- "Token sharing" is OpenAI's partner program that lets a third-party app spend the user's ChatGPT plan on Responses API calls. The scope `chatgpt.tokens.use.direct` is the grant. The guide says it "does not grant access to their ChatGPT conversations or other account context" ([overview](https://developers.openai.com/siwc/token-sharing-open-source)).
- Each user registers their own OAuth client the first time (`client_id=dynamic_agent_client` → OpenAI issues a real client id). "Each issued `client_id` is bound to the authenticated user and the workspace selected during registration" ([overview](https://developers.openai.com/siwc/token-sharing-open-source)).
- A "host" is a place where the app runs. `ext_agent_host_id` is a stable opaque id for that host, "not an authentication credential"; it must not be an email or user id ([overview](https://developers.openai.com/siwc/token-sharing-open-source)).

The OpenCode PRs that added this: [#52147](https://github.com/anomalyco/opencode/pull/52147) (new method, 2026-09-29), [#52162](https://github.com/anomalyco/opencode/pull/52162) ("align with updated partner guide"), [#64947](https://github.com/anomalyco/opencode/pull/64947) (revoke on logout).

**Older rows.** Before #52147, token-sharing rows were stored under `chatgpt-browser`. At startup the ChatGPT plugin moves any `chatgpt-browser` row whose metadata has a `clientID` to `chatgpt-token-sharing` ([L256-L275](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L256-L275)). Real Codex-style rows (metadata `{accountID}`) are left alone.

### 1b. `chatgpt-browser` and `chatgpt-headless` (legacy Codex-style logins)

What the code shows (`openai.ts`):

- Browser ([L58-L116](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/openai.ts#L58-L116), [L418-L431](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/openai.ts#L418-L431)): `https://auth.openai.com/oauth/authorize` with `client_id=app_EMoamEEZ73f0CkXaXp7hrann`, `scope=openid profile email offline_access`, PKCE, `id_token_add_organizations=true`, `codex_cli_simplified_flow=true`, `originator=opencode`, redirect `http://localhost:1455/auth/callback` (fallback port 1457). The same client and port as Codex CLI.
- Device code ([L178-L237](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/openai.ts#L178-L237)): POST `https://auth.openai.com/api/accounts/deviceauth/usercode` with the client id, user enters the code at `https://auth.openai.com/codex/device`, OpenCode polls `https://auth.openai.com/api/accounts/deviceauth/token`, which returns an `authorization_code` and `code_verifier`; these are exchanged with redirect `https://auth.openai.com/deviceauth/callback`.
- Exchange and refresh go to `https://auth.openai.com/oauth/token` ([L353-L382](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/openai.ts#L353-L382)).
- Saved value ([L395-L405](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/openai.ts#L395-L405), [L433-L447](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/openai.ts#L433-L447)): `{type: "oauth", methodID, refresh, access, expires, metadata: {accountID}}`. `accountID` is read without verification from the `id_token` (then the access token): `chatgpt_account_id`, or `https://api.openai.com/auth.chatgpt_account_id`, or `organizations[0].id`. The `id_token` itself is discarded here too. No email is kept.

## 2. How OpenCode calls models with the token-sharing token

What the code shows:

- Base URL `https://api.openai.com/v1`, HTTP transport (not WebSocket), zstd request bodies unless the user set a custom `baseURL`, summary compaction (the native `/responses/compact` is not supported) ([`chatgpt.ts` L303-L328](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L303-L328), [L329-L347](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L329-L347)). Requests go through OpenCode's OpenAI Responses provider, so they hit `POST /v1/responses`.
- Auth: the access token is passed as the provider `apiKey` ([`model-resolver.ts` L418-L438](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/model-resolver.ts#L418-L438)) and sent as `Authorization: Bearer <access>` ([`packages/ai/src/route/auth.ts` L47](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/ai/src/route/auth.ts#L47)).
- Extra headers: `session-id`, `thread-id`, `x-client-request-id`, copied from what Codex sends, for prompt-cache affinity ([L288-L302](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L288-L302), PR [#53767](https://github.com/anomalyco/opencode/pull/53767)).
- **No `ChatGPT-Account-Id` header** on this route. A test checks that it is absent ([`provider-chatgpt.test.ts` L1120](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/test/plugin/provider-chatgpt.test.ts#L1120)). OpenAI's guide lists only `Authorization: Bearer` too ([models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)); the issued client already carries the user and workspace.
- Model list: OpenCode does **not** call `/v1/models` today; that sync is commented out (PR [#53466](https://github.com/anomalyco/opencode/pull/53466), "OpenAI bugs"). It uses a list from models.dev, or a built-in fallback list ([L35-L57](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L35-L57), [L277-L278](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L277-L278)). `fetchModels` still exists but is unused.
- Errors such as `subscription_sharing_usage_limit_exceeded` stop retries ([L58-L67](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L58-L67)).

For comparison, the legacy Codex-style logins call `https://chatgpt.com/backend-api/codex` with `originator: opencode`, `x-codex-beta-features: remote_compaction_v2` and `chatgpt-account-id: <metadata.accountID>` ([`openai.ts` L268-L286](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/openai.ts#L268-L286)).

**No call returns the email, account id or plan.** I found no userinfo, "me", accounts or usage call for ChatGPT logins in OpenCode. The only usage call (`usage-limits.ts`) is for the OpenCode Console. For ChatGPT, the app just links to `https://chatgpt.com/settings/usage`.

**The email is not shown anywhere,** because it is never stored. `opencode auth list` prints the integration name, the row label ("OpenAI", "OpenAI 2") and "stored" ([`packages/cli/src/commands/handlers/auth/list.ts`](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/cli/src/commands/handlers/auth/list.ts)). `opencode auth export` prints the raw rows, so also no email.

## 3. Refresh

What the code shows:

- When a credential is resolved for a request and it expires within 5 minutes, OpenCode refreshes it ([`integration.ts` L700-L715](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/integration.ts#L700-L715), [`credential.ts` L32-L34](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/credential.ts#L32-L34)).
- One refresh at a time per row, across processes, under a file lock `credential-refresh:<id>`. Under the lock the row is read again; if another process already refreshed it, that row is used. If the row changed while the refresh ran, the new result is thrown away ([`credential.ts` L136-L184](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/credential.ts#L136-L184)). Note: a refresh token is single-use, so a second tool refreshing the same row would break one of them.
- The request ([`chatgpt.ts` L407-L428](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L407-L428)): form POST to `https://auth.openai.com/api/accounts/oauth/token` with `grant_type=refresh_token`, `client_id=<metadata.clientID>`, `refresh_token`, `resource=https://api.openai.com/v1`. On `invalid_grant` it says "This ChatGPT sign-in has expired or was revoked. Connect again."
- What it writes back: the whole `value` is replaced with `{type, methodID: "chatgpt-token-sharing", refresh: <new>, access: <new>, expires: now + expires_in, metadata: {clientID, scopes}}`. `scopes` come from the response, or the old ones if the response has none. Any `id_token` in the refresh response is ignored. Label, id and active flag stay the same.
- Logout calls `https://auth.openai.com/api/accounts/oauth/revoke` with the refresh token and client id (5 s timeout), then deletes the row ([L430-L455](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/plugin/provider/chatgpt.ts#L430-L455)).

From OpenAI's [token reference](https://developers.openai.com/siwc/token-sharing-open-source/token-reference): access tokens last 1 hour; refresh tokens last 30 days and each refresh returns a new refresh token with a fresh 30 days.

## 4. Reusing a Codex login, sharing logins, importing tokens

What the code shows:

- OpenCode **does not read `~/.codex/auth.json`** or `CODEX_HOME`. I found no code that does.
- The only import paths are OpenCode's own: the one-time import of its old v1 `auth.json` from its data folder ([migration `20260805200742_import_legacy_credentials.ts` L39](https://github.com/anomalyco/opencode/blob/23fd3da78601f3afd94b9845db815bbf11edf408/packages/core/src/database/migration/20260805200742_import_legacy_credentials.ts#L39); it maps the old `accountId` to `metadata.accountID`), and `opencode auth import` of a file made by `opencode auth export`.
- The legacy Codex-style methods use the same client id as Codex CLI (`app_EMoamEEZ73f0CkXaXp7hrann`), but they make their own separate login. They do not share Codex's tokens.
- The docs for OpenCode Console's Codex harness only tell users to edit `~/.codex/config.toml` to add a provider; that is unrelated to login.

What this means for telling two logins apart without a network call:

- **Legacy OpenCode rows (`chatgpt-browser` / `chatgpt-headless`)**: `metadata.accountID` is the same `chatgpt_account_id` that Codex keeps in `~/.codex/auth.json` (`tokens.account_id`, and in the `id_token` claim `https://api.openai.com/auth`). Compare them directly. (Codex reads `email`, `chatgpt_plan_type`, `chatgpt_user_id`, `chatgpt_account_id` from its `id_token`: [`codex-rs/login/src/token_data.rs`](https://github.com/openai/codex/blob/44848822b1a58fa40cdcd89ec3161c757735cfd0/codex-rs/login/src/token_data.rs).)
- **Token-sharing rows (`chatgpt-token-sharing`)**: the code and docs give no offline way. The access token has only an opaque `sub` and the opaque `per_user_salt` / `encrypted_auth_metadata` ("your app does not need to interpret it", [token reference](https://developers.openai.com/siwc/token-sharing-open-source/token-reference)). The `id_token` with `sub`/`email` is not stored. The `clientID` is per registration, so two OpenCode rows with different client ids are two registrations, maybe of the same person.

Guesses (not checked, do not rely on them):

- `per_user_salt` might be the same for one ChatGPT user across clients. If so, comparing it between "OpenAI" and "OpenAI 2" would hint at the same person. OpenAI calls it opaque, so this could change at any time, and it would not link to a Codex token unless Codex access tokens carry the same field.
- The opaque per-client `sub` in the token-sharing access token looks pairwise (different per client). The docs only say "Opaque subject identifier", so this is not confirmed.

## 5. The userinfo endpoint

What the docs and code show:

- The discovery document lists `"userinfo_endpoint": "https://auth.openai.com/api/accounts/oauth/userinfo"`, with claims such as `sub`, `name`, `email`, `email_verified`, `picture`, `updated_at`, and `scopes_supported` only `openid profile email offline_access`. It does not list any ChatGPT account, plan or workspace claim.
- OpenAI's SIWC guide does not mention userinfo. For identity it says to take `sub` (and `email` for display) from the verified `id_token`, and to keep the `id_token` for `id_token_hint` on later sign-ins ([sign in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in)). It also warns: "Email helps users recognize an account but may be identical across account registrations," and email and `sub` are not workspace identifiers ([accounts and sessions](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions)).
- OpenAI's devkit keeps `profileIdToken`, `subject`, `name`, `email` from the `id_token` and refuses a re-login or refresh whose `sub` differs ([`oauth.ts`](https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/packages/local/src/oauth.ts)). It never calls userinfo.
- Neither OpenCode nor Codex CLI calls the userinfo endpoint (GitHub code search for `api/accounts/oauth/userinfo` in `openai/codex` has no hits; none in OpenCode).

Guesses (not checked):

- Userinfo would normally need an access token whose audience is the auth server. The token-sharing access token has audience `https://api.openai.com/v1`, so userinfo may reject it. Even if it accepts it, it would most likely return the same per-client `sub` and an email, not a ChatGPT account id or plan. Trying it needs a real token and a network call, which this research did not do.

## Takeaways for OTTY

- From an OpenCode `chatgpt-token-sharing` row we can read: client id, scopes, expiry, and the opaque access-token claims. We cannot read email, ChatGPT account id or plan, and OpenCode itself never knows them.
- Linking such a row to a Codex login offline is not possible from documented data. Linking a legacy OpenCode Codex-style row is possible through `metadata.accountID`.
- Never refresh OpenCode's tokens ourselves: refresh tokens rotate, and OpenCode would lose its login.

## How CodexBar does it

Source: [steipete/CodexBar](https://github.com/steipete/CodexBar), read at commit `a26ca57` (2026-11-21, version 0.85.1 in development). Paths below are relative to that repository.

What the code shows:

- CodexBar has two OpenCode providers, and neither one handles OpenCode's ChatGPT login:
  - `opencode` (`Sources/CodexBarCore/Providers/OpenCode/`) reads the opencode.ai web console with browser cookies (`auth`, `__Host-console_session`): `GET https://opencode.ai/console/api/orgs`, `/console/api/go/status` and `/console/api/billing/status` with an `x-org-id` header, `/console/api/usage/summary?range=30d`, and legacy `POST https://opencode.ai/_server` calls. See `docs/opencode.md`.
  - `opencodego` (`Sources/CodexBarCore/Providers/OpenCodeGo/`) reads OpenCode Go quotas from `GET https://opencode.ai/zen/go/v1/usage` with `Authorization: Bearer <key or access token>`. Without a configured key it borrows the OpenCode CLI login: `OpenCodeGoLocalUsageReader.loadCLICredential` runs `SELECT integration_id, value FROM credential WHERE active = 1 AND integration_id IN ('opencode-go', 'opencode')` on `opencode.db` (path from `XDG_DATA_HOME` and `OPENCODE_DB`), reads only `type`, `key`, `access`, `expires` from the JSON value, and falls back to the `opencode-go` `api` entry of the old `auth.json` only when the `credential` table does not exist. Two active rows for the same integration count as an error ("never pick an arbitrary account"). It also reads `opencode-go` cost history from the `message`, `part` and `session_message` tables.
- The ChatGPT part sits in the Codex provider as an opt-in setting, **External Codex OAuth sources** (off by default; `docs/codex.md`, "Optional external OAuth sources"). In `Sources/CodexBarCore/Providers/Codex/CodexOAuth/CodexOAuthCredentials.swift`, `loadForUsage` tries OpenCode only when `~/.codex/auth.json` is missing (`.notFound`) and `CODEX_HOME` is not set. The order is `~/.codex/auth.json`, then `~/.config/codex/auth.json`, then `~/.local/share/opencode/auth.json`.
- `parseOpenCode` reads only the legacy `auth.json` entry `openai`: `type == "oauth"`, `access`, `refresh`, `expires` (milliseconds), and a top-level `accountId`. It does not read the `credential` table for `openai`, `methodID`, or `metadata` (so neither `metadata.accountID` nor `metadata.clientID`). It sets `idToken` to nil and does not decode the access token, so for OpenCode it never gets an account id from JWT claims. For native Codex logins it does decode JWTs (`accountIDFromJWT`: `chatgpt_account_id`, `https://api.openai.com/auth.chatgpt_account_id`, `organizations[0].id`).
- Limits: `GET https://chatgpt.com/backend-api/wham/usage` (or `chatgpt_base_url` from `~/.codex/config.toml`) with `Authorization: Bearer <access>`, `User-Agent: CodexBar`, `Accept: application/json`, and `ChatGPT-Account-Id: <accountId>` only when an account id is known (`CodexOAuthUsageFetcher.swift`). So for a login with no account id, it calls `wham/usage` without the account header. The response's `plan_type` and `account_id` are decoded.
- Account name and plan (`CodexReconciledState.oauthIdentity`): the email comes only from `id_token` claims (`email` or `https://api.openai.com/profile.email`). An OpenCode login therefore has no email in CodexBar. The plan comes from the `wham/usage` field `plan_type`, falling back to the `id_token` claim `chatgpt_plan_type`. The response's `account_id` is used only for the credits and balance calls, not as the account's identity.
- Refresh: CodexBar does not refresh OpenCode tokens. A token that expires within 60 seconds raises `readOnlySource` ("stale and read-only. Sign in again with its owning app"), and no fallback follows. The code comment explains why: redeeming a shared refresh token without writing the new one back leaves the owner with a dead token. (A refresher for `https://auth.openai.com/oauth/token` with client id `app_EMoamEEZ73f0CkXaXp7hrann` exists, but writing back is allowed only for the native `~/.codex` source.) The OpenCode Go reader also refuses an expired OAuth row instead of refreshing it.
- Duplicate accounts: CodexBar has no logic that links an OpenCode login to a Codex login. It never has both at once, because OpenCode is read only when the Codex file is missing. Codex accounts are identified by `CodexIdentityResolver` (`Sources/CodexBarCore/Providers/Codex/CodexIdentity.swift`): the account id first, then the lowercased email, otherwise `unresolved`.
- One more identity endpoint, used only for Codex personal access tokens (not OAuth tokens): `GET https://auth.openai.com/api/accounts/v1/user-auth-credential/whoami` with `Authorization: Bearer <token>`, a Codex CLI `User-Agent` and an `originator` header. It returns `chatgpt_account_id`, `chatgpt_plan_type` and `email` (`Sources/CodexBarCore/Providers/Codex/CodexPAT/CodexPATUsageFetcher.swift`).

Guesses (not checked):

- A current OpenCode `chatgpt-token-sharing` login that lives only in `opencode.db` is invisible to CodexBar, because it reads only the `openai` entry of the old `auth.json`.
- Whether `wham/usage` accepts a token-sharing access token (audience `https://api.openai.com/v1`) without `ChatGPT-Account-Id` is not shown by CodexBar's code or tests. If it does, the `plan_type` and `account_id` in its response would be the only way to name the account. They give a plan and an account id, not an email.
- The `whoami` endpoint might also accept an OAuth access token and return an email, but CodexBar only calls it with personal access tokens. Nothing confirms that it works with OpenCode's token.

Takeaways for OTTY:

- CodexBar's answer to a login with no email is to show no email. The plan comes from the limits response.
- It never refreshes another app's tokens. When the token is stale, it tells the user to sign in again in the app that owns the login. This matches the rule in "Takeaways for OTTY" above.
- Like OTTY, it treats one active row per integration as required and reports two as an error instead of guessing.

## Probe with a real token-sharing access token (2026-11-23)

With the owner's go-ahead, one GET each with the active OpenCode
`chatgpt-token-sharing` access token (freshly refreshed by OpenCode
itself, not by us), `Authorization: Bearer`, no account header. Only the
status and the field names were looked at, never the values.

| Endpoint | Status | Answer |
|---|---|---|
| `https://chatgpt.com/backend-api/wham/usage` | 401 | `rejected_by_access_enforcement`, code `no_matching_rule` |
| `https://auth.openai.com/api/accounts/v1/user-auth-credential/whoami` | 401 | `rejected_by_access_enforcement`, code `auth_token_type_not_allowed` |
| `https://auth.openai.com/api/accounts/oauth/userinfo` | 200 | only `sub` (no `email`, no `name`, despite the `email profile` scopes) |

So a token-sharing login gives no ChatGPT limits, no account id, no plan
and no email through these endpoints: only the opaque per-client `sub`.
An OTTY "Sign in with ChatGPT" of the same kind would get the email once
from the id_token at sign-in, but no limits. ChatGPT limits stay
reachable only through a Codex-style login (the Codex CLI client).
