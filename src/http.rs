use crate::Error;
use std::collections::HashMap;
use std::fmt;

/// An alias for `HashMap<String, String>`, which represents a set of HTTP headers and their values.
pub type Headers = HashMap<String, String>;

/// Enumerates the HTTP methods used by `tus_client::Client`.
#[derive(Debug)]
pub enum HttpMethod {
    Head,
    Patch,
    Options,
    Post,
    Delete,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Represents an HTTP request to be executed by the handler.
#[derive(Debug)]
pub struct HttpRequest<'a> {
    pub method: HttpMethod,
    pub headers: Headers,
    pub url: String,
    pub body: Option<&'a [u8]>,
}

/// Represents an HTTP response from the server.
#[derive(Debug)]
pub struct HttpResponse {
    pub headers: Headers,
    pub status_code: usize,
}

/// The required trait used by `tus_client::Client` to represent a handler to execute `HttpRequest`s.
pub trait HttpHandler {
    fn handle_request(&self, req: HttpRequest) -> Result<HttpResponse, Error>;
}

/// Returns the default headers required to make requests to an tus enabled endpoint.
pub fn default_headers() -> Headers {
    let mut map = Headers::new();
    map.insert(
        String::from(crate::headers::TUS_RESUMABLE),
        String::from("1.0.0"),
    );
    map
}
