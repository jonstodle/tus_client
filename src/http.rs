use std::collections::HashMap;
use std::io::Error;

#[derive(Debug)]
pub enum HttpMethod {
    Head,
    Patch,
    Options,
    Post,
    Delete,
}

#[derive(Debug)]
pub struct HttpRequest<T> {
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub url: String,
    pub body: T,
}

#[derive(Debug)]
pub struct HttpResponse {
    pub headers: HashMap<String, String>,
    pub status_code: usize,
}

pub trait HttpHandler {
    fn handle_request(&self, req: HttpRequest<()>) -> Result<HttpResponse, Error>;
}

pub fn default_headers() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert(String::from(crate::headers::TUS_RESUMABLE), String::from("1.0.0"));
    map
}
