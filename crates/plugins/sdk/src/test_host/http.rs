//! Canned HTTP answers, and the requests the plugin sent.

use otty_plugin_manifest::Permissions;

use crate::HostError;
use crate::http::{Request, Response};
use crate::test_host::{HostState, SentRequest};

/// Sends `request` to its canned answer, after the checks OTTY makes.
pub(super) fn send(
    state: &mut HostState,
    permissions: &Permissions,
    request: &Request<'_>,
) -> Result<Response, HostError> {
    let url = request.url();
    let Some(host) = https_host(url) else {
        return Err(HostError::NotAllowed(format!("not HTTPS: {url}")));
    };
    if !permissions.may_call(host) {
        return Err(HostError::NotAllowed(host.to_string()));
    }

    let headers = request
        .headers()
        .iter()
        .map(|header| {
            let value = match header.key() {
                Some(key) => {
                    format!("{}{}", header.value(), state.key_text(key.id())?)
                },
                None => header.value().to_string(),
            };

            Ok((header.name().to_string(), value))
        })
        .collect::<Result<Vec<_>, HostError>>()?;
    state.requests.push(SentRequest {
        method: request.method().to_string(),
        url: url.to_string(),
        headers,
        body: request.body().map(<[u8]>::to_vec),
    });

    let answer = state
        .answers
        .iter()
        .find(|answer| answer.method == request.method() && answer.url == url)
        .ok_or_else(|| {
            HostError::Failed(format!(
                "no canned answer for {} {url}",
                request.method()
            ))
        })?;

    Ok(Response {
        status: answer.status,
        headers: Vec::new(),
        body: answer.body.clone(),
    })
}

/// The host of an `https://` URL, without port, path or query.
fn https_host(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let end = rest.find(['/', ':', '?', '#']).unwrap_or(rest.len());
    let host = &rest[..end];

    (!host.is_empty()).then_some(host)
}
