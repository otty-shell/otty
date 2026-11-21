//! What the requests of the provider part `openai` share: the JSON answer
//! of a request and the percent encoding of a value put into an address.

use otty_plugin_sdk::agents::ReadError;
use otty_plugin_sdk::http::{self, Request};
use serde_json::Value;

/// The JSON answer of `request` to `url`. A refused key (401 or 403) is
/// the error `refused` gives, such as a refused login or a need for a new
/// key, any other status is a failure and an answer that is not JSON
/// unexpected. The error never holds what the answer said.
pub(crate) fn json_answer(
    request: &Request<'_>,
    url: &str,
    refused: impl FnOnce() -> ReadError,
) -> Result<Value, ReadError> {
    let response = http::request(request).map_err(ReadError::Host)?;

    match response.status {
        200 => serde_json::from_slice(&response.body).map_err(|_| {
            ReadError::Unexpected(format!("the answer of {url} is not JSON"))
        }),
        401 | 403 => Err(refused()),
        status => Err(ReadError::Failed(format!("{url} answered {status}"))),
    }
}

/// `text` with every byte but the letters, the digits and `-._~` written
/// as `%XX`, for a value put into an address.
pub(crate) fn percent_encoded(text: &str) -> String {
    let mut encoded = String::new();

    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_unreserved_bytes_stay_as_they_are() {
        assert_eq!(percent_encoded("page_MjAy+/="), "page_MjAy%2B%2F%3D");
        assert_eq!(percent_encoded("page 2"), "page%202");
        assert_eq!(percent_encoded("a-b.c_d~e"), "a-b.c_d~e");
    }
}
