//! HTTPS requests to the hosts the called part declares in `http`.

/// The answer to a request.
pub use crate::bindings::host::otty::host::http::Response;
use crate::{HostError, Key};

/// One request, built with [`Request::get`] or [`Request::post`] and its
/// headers. A key goes into a header only through
/// [`header_with_key`](Self::header_with_key): the host fills it in.
#[derive(Debug)]
pub struct Request<'key> {
    method: String,
    url: String,
    headers: Vec<Header<'key>>,
    body: Option<Vec<u8>>,
}

impl<'key> Request<'key> {
    /// The method, such as `GET`.
    pub fn method(&self) -> &str {
        &self.method
    }

    /// The URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The headers in the order they were added.
    pub fn headers(&self) -> &[Header<'key>] {
        &self.headers
    }

    /// The body, if any.
    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }

    /// A `GET` of `url`.
    pub fn get(url: &str) -> Self {
        Self {
            method: "GET".to_string(),
            url: url.to_string(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// A `POST` of `body` to `url`.
    pub fn post(url: &str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            method: "POST".to_string(),
            url: url.to_string(),
            headers: Vec::new(),
            body: Some(body.into()),
        }
    }

    /// The same request with the header `name: value`.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push(Header {
            name: name.to_string(),
            value: value.to_string(),
            key: None,
        });

        self
    }

    /// The same request with the header `name: <prefix><key>`, such as
    /// `authorization: Bearer <key>`. The plugin never sees the key: the
    /// host fills it in right before sending.
    ///
    /// ```no_run
    /// use otty_plugin_sdk::Key;
    /// use otty_plugin_sdk::http::{self, Request};
    ///
    /// fn balance(key: &Key) -> Option<Vec<u8>> {
    ///     let request = Request::get("https://api.example.com/v1/balance")
    ///         .header_with_key("authorization", "Bearer ", key);
    ///
    ///     http::request(&request).ok().map(|response| response.body)
    /// }
    /// ```
    pub fn header_with_key(
        mut self,
        name: &str,
        prefix: &str,
        key: &'key Key,
    ) -> Self {
        self.headers.push(Header {
            name: name.to_string(),
            value: prefix.to_string(),
            key: Some(key),
        });

        self
    }
}

/// One header of a [`Request`].
#[derive(Debug)]
pub struct Header<'key> {
    name: String,
    value: String,
    key: Option<&'key Key>,
}

impl<'key> Header<'key> {
    /// The header's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The header's value; with a key, the text before the key.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The key the host puts after the value, if any.
    pub fn key(&self) -> Option<&'key Key> {
        self.key
    }
}

/// Sends `request`: HTTPS only, to a host the part declares, with no
/// redirects, within 30 s, at most 8 MiB of answer.
pub fn request(request: &Request<'_>) -> Result<Response, HostError> {
    #[cfg(target_arch = "wasm32")]
    {
        use crate::bindings::host::otty::host::http as wit;

        let headers = request
            .headers
            .iter()
            .map(|header| wit::Header {
                name: header.name.clone(),
                value: header.value.clone(),
                key: header.key.map(Key::wasm),
            })
            .collect();
        let request = wit::Request {
            method: request.method.clone(),
            url: request.url.clone(),
            headers,
            body: request.body.clone(),
        };

        wit::send(&request)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::send(request)
    }
}
