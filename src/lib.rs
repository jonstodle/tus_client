use crate::http::{HttpHandler, HttpRequest, HttpResponse, default_headers};
use std::ops::Deref;
use std::io;
use std::num::ParseIntError;
use std::collections::HashMap;

mod headers;
pub mod http;

pub struct Client<'a> {
    http_handler: Box<dyn HttpHandler + 'a>,
}

impl<'a> Client<'a> {
    pub fn new(http_handler: impl HttpHandler + 'a) -> Self {
        Client {
            http_handler: Box::new(http_handler),
        }
    }

    /// Get the number of bytes already uploaded to the server
    pub fn get_progress(&self, url: &str) -> Result<ProgressResponse, Error> {
        let req = HttpRequest {
            headers: default_headers(),
            url: String::from(url),
            body: (),
        };

        let response = self.http_handler.deref().head(req)?;

        if response.status_code.to_string().starts_with("4") ||
            !response.headers.contains_key(headers::UPLOAD_OFFSET) {
            return Err(Error::NotFoundError);
        }

        let bytes_uploaded: usize = response.headers.get(headers::UPLOAD_OFFSET).unwrap().parse()?;
        let total_size = response.headers.get(headers::UPLOAD_LENGTH)
            .and_then(|l| l.parse::<usize>().ok());

        Ok(ProgressResponse {
            bytes_uploaded,
            total_size,
        })
    }
}

#[derive(Debug)]
pub struct ProgressResponse {
    pub bytes_uploaded: usize,
    pub total_size: Option<usize>,
}

#[derive(Debug)]
pub enum Error {
    NotFoundError,
    IoError(io::Error),
    ParsingError(ParseIntError),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoError(e)
    }
}

impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Self {
        Error::ParsingError(e)
    }
}
