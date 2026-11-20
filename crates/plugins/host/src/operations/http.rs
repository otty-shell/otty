//! `http.send`: an HTTPS request to a host the called part declares, a
//! key handle in a header filled in by the host right before sending.

use std::time::Instant;

use ureq::http::Uri;

use crate::calls::CallState;
use crate::https::HttpsRequest;
use crate::operations::keys::key_of;
use crate::operations::v0_1::otty::host::http::{Request, Response};
use crate::operations::v0_1::otty::host::types::HostError;

/// Headers the host sets itself: a plugin may not send another host name
/// or another `User-Agent` than OTTY's.
const HOST_HEADERS: [&str; 2] = ["host", "user-agent"];

/// Sends `request` and gives the answer, whatever its status. A header
/// with a key gets its value followed by the key, read from its place now
/// and dropped once the request is sent.
pub(crate) fn send(
    state: &mut CallState,
    request: Request,
) -> Result<Response, HostError> {
    let host = https_host(&request.url)?;
    if !state.permissions().may_call(&host) {
        return Err(HostError::NotAllowed(host));
    }

    let mut headers = Vec::with_capacity(request.headers.len());
    for header in &request.headers {
        let name = header.name.to_ascii_lowercase();
        if HOST_HEADERS.contains(&name.as_str()) {
            return Err(HostError::NotAllowed(format!(
                "the header {}, which OTTY sets",
                header.name
            )));
        }

        let value = match &header.key {
            Some(key) => {
                let key = key_of(state, key)?;
                format!("{}{}", header.value, key.as_str())
            },
            None => header.value.clone(),
        };
        headers.push((header.name.clone(), value));
    }
    let sent = HttpsRequest {
        method: request.method,
        url: request.url,
        headers,
        body: request.body,
    };

    let started = Instant::now();
    let answer = state.outside().https().send(&sent);
    drop(sent);
    state.extend_deadline(started.elapsed());

    answer
}

/// The host name of the HTTPS address `url`, lowercase. An address that
/// is not HTTPS, or that names a user before its host, is refused.
fn https_host(url: &str) -> Result<String, HostError> {
    let refused =
        || HostError::NotAllowed(format!("not an HTTPS address: {url}"));
    let address: Uri = url.parse().map_err(|_| refused())?;
    let authority = address.authority().ok_or_else(refused)?;

    if address.scheme_str() != Some("https") || authority.as_str().contains('@')
    {
        return Err(refused());
    }

    Ok(authority.host().to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use otty_plugin_manifest::{Environment, PartRef};

    use super::*;
    use crate::https::MockHttpsSend;
    use crate::keys::KeyHandle;
    use crate::operations::v0_1::otty::host::http::Header;
    use crate::operations::v0_1::otty::host::keys::KeyPlace;
    use crate::system_keyring::MockKeyringRead;
    use crate::testing::{TempDir, engine_with, in_call, load_keys_plugin};

    const KEY: &str = "ex-made-up-key";

    fn env() -> Environment {
        Environment::new("/home/test-user".into()).with_var("CLOUD_KEY", KEY)
    }

    fn get(url: &str) -> Request {
        Request {
            method: "GET".to_string(),
            url: url.to_string(),
            headers: Vec::new(),
            body: None,
        }
    }

    fn header(name: &str, value: &str) -> Header {
        Header {
            name: name.to_string(),
            value: value.to_string(),
            key: None,
        }
    }

    fn answer(body: &[u8]) -> Response {
        Response {
            status: 200,
            headers: vec![("content-type".to_string(), "json".to_string())],
            body: body.to_vec(),
        }
    }

    /// Sends `request` as the part `part`, with a client that must not be
    /// asked.
    fn refused_send(part: &PartRef, request: Request) -> HostError {
        let cache = TempDir::new();
        let mut https = MockHttpsSend::new();
        https.expect_send().never();
        let engine = engine_with(&cache, https, MockKeyringRead::new());
        let plugin = load_keys_plugin(&engine, &env());

        in_call(&plugin, part, |state| send(state, request))
            .expect_err("refused")
    }

    #[test]
    fn the_header_carries_the_key_only_in_the_sent_request() {
        let cache = TempDir::new();
        let mut https = MockHttpsSend::new();
        https
            .expect_send()
            .withf(|sent| {
                sent.method == "GET"
                    && sent.url == "https://api.example.com/v1/balance"
                    && sent.headers
                        == [
                            ("accept".to_string(), "json".to_string()),
                            (
                                "authorization".to_string(),
                                format!("Bearer {KEY}"),
                            ),
                        ]
            })
            .times(1)
            .returning(|_| Ok(answer(b"{\"balance\": 12.5}")));
        let engine = engine_with(&cache, https, MockKeyringRead::new());
        let plugin = load_keys_plugin(&engine, &env());

        let (response, built) =
            in_call(&plugin, &PartRef::provider("test-cloud"), |state| {
                let key = state
                    .table()
                    .push(KeyHandle::new(KeyPlace::EnvVar(
                        "CLOUD_KEY".to_string(),
                    )))
                    .expect("a handle");
                let mut request = get("https://api.example.com/v1/balance");
                request.headers = vec![
                    header("accept", "json"),
                    Header {
                        name: "authorization".to_string(),
                        value: "Bearer ".to_string(),
                        key: Some(key),
                    },
                ];
                let built: Vec<String> = request
                    .headers
                    .iter()
                    .map(|header| header.value.clone())
                    .collect();

                (send(state, request), built)
            });

        assert_eq!(response.expect("an answer").body, b"{\"balance\": 12.5}");
        assert_eq!(built, ["json", "Bearer "]);
    }

    #[test]
    fn a_host_nobody_lists_is_refused() {
        let error = refused_send(
            &PartRef::provider("test-cloud"),
            get("https://evil.example.net/"),
        );

        assert!(
            matches!(error, HostError::NotAllowed(host) if host == "evil.example.net")
        );
    }

    #[test]
    fn a_host_only_another_part_lists_is_refused() {
        let other_provider = refused_send(
            &PartRef::provider("test-cloud"),
            get("https://api.example.org/v1/me"),
        );
        let agent = refused_send(
            &PartRef::agent("test-agent"),
            get("https://api.example.com/v1/me"),
        );

        assert!(matches!(other_provider, HostError::NotAllowed(_)));
        assert!(matches!(agent, HostError::NotAllowed(_)));
    }

    #[test]
    fn only_an_https_address_of_the_host_itself_is_sent() {
        let part = PartRef::provider("test-cloud");
        let addresses = [
            "http://api.example.com/v1/me",
            "https://user@api.example.com/v1/me",
            "https://api.example.com@evil.example.net/",
            "https://sub.api.example.com/",
            "api.example.com/v1/me",
            "not an address",
        ];

        for address in addresses {
            let error = refused_send(&part, get(address));

            assert!(
                matches!(error, HostError::NotAllowed(_)),
                "{address}: {error:?}"
            );
        }
    }

    #[test]
    fn a_plugin_may_not_set_the_host_or_the_user_agent() {
        for name in ["Host", "user-agent"] {
            let mut request = get("https://api.example.com/v1/me");
            request.headers = vec![header(name, "evil")];

            let error = refused_send(&PartRef::provider("test-cloud"), request);

            assert!(matches!(error, HostError::NotAllowed(_)), "{name}");
        }
    }

    #[test]
    fn a_failure_of_the_client_reaches_the_plugin() {
        let cache = TempDir::new();
        let mut https = MockHttpsSend::new();
        https.expect_send().returning(|_| {
            Err(HostError::TimedOut(
                "https://api.example.com/ did not answer".to_string(),
            ))
        });
        let engine = engine_with(&cache, https, MockKeyringRead::new());
        let plugin = load_keys_plugin(&engine, &env());

        let sent =
            in_call(&plugin, &PartRef::provider("test-cloud"), |state| {
                send(state, get("https://API.example.com:443/"))
            });

        assert!(matches!(sent, Err(HostError::TimedOut(_))));
    }
}
