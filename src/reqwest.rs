use crate::http::{HttpHandler, HttpMethod, HttpRequest, HttpResponse};
use crate::Error;
use reqwest::header::{HeaderMap, HeaderName};
use reqwest::Method;
use std::collections::HashMap;
use std::str::FromStr;

impl HttpHandler for reqwest::Client {
    fn handle_request(&self, req: HttpRequest) -> Result<HttpResponse, Error> {
        let mut headers = HeaderMap::new();
        for (key, value) in req.headers {
            headers.insert(HeaderName::from_str(&key).unwrap(), value.parse().unwrap());
        }

        let mut builder = match req.method {
            HttpMethod::Head => self.head(&req.url),
            HttpMethod::Patch => self.patch(&req.url),
            HttpMethod::Options => self.request(Method::OPTIONS, &req.url),
            HttpMethod::Post => self.post(&req.url),
            HttpMethod::Delete => self.delete(&req.url),
        }
        .headers(headers);

        if let Some(body) = req.body {
            builder = builder.body(Vec::from(body));
        }

        let response = match builder.send() {
            Ok(resp) => resp,
            Err(err) => return Err(Error::HttpHandlerError(err.to_string())),
        };

        let mut headers = HashMap::new();
        for (key, value) in response.headers() {
            headers.insert(
                key.to_string(),
                value.to_str().map(String::from).unwrap_or_default(),
            );
        }

        Ok(HttpResponse {
            status_code: response.status().as_u16() as usize,
            headers,
        })
    }
}
