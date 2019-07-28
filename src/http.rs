use std::collections::HashMap;
use std::io::Error;
use std::fmt;

pub type Headers = HashMap<String, String>;

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

#[derive(Debug)]
pub struct HttpRequest<'a> {
    pub method: HttpMethod,
    pub headers: Headers,
    pub url: String,
    pub body: Option<&'a [u8]>,
}

#[derive(Debug)]
pub struct HttpResponse {
    pub headers: Headers,
    pub status_code: usize,
}

pub trait HttpHandler {
    fn handle_request(&self, req: HttpRequest) -> Result<HttpResponse, Error>;
}

pub fn default_headers() -> Headers {
    let mut map = Headers::new();
    map.insert(String::from(crate::headers::TUS_RESUMABLE), String::from("1.0.0"));
    map
}
