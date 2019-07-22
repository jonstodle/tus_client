use std::collections::HashMap;
use std::io::Error;

pub struct HttpRequest<T> {
    pub headers: HashMap<String, String>,
    pub url: String,
    pub body: T,
}

pub struct HttpResponse {
    pub headers: HashMap<String, String>,
    pub status_code: usize,
}

pub trait HttpHandler {
    fn head(&self, req: HttpRequest<()>) -> Result<HttpResponse, Error>;
}

pub fn default_headers() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert(String::from(crate::headers::TUS_RESUMABLE), String::from("1.0.0"));
    map
}
