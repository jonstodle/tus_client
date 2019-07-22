use crate::http::{HttpHandler, HttpRequest, default_headers};
use std::ops::Deref;
use std::io;
use std::num::ParseIntError;

mod headers;
mod http;

pub struct Client {
    http_handler: Box<dyn HttpHandler>,
}

impl Client {
    /// Get the number of bytes already uploaded to the server
    pub fn get_progress(&self, url: &str) -> Result<ProgressResponse, Error> {
        let req = HttpRequest {
            headers: default_headers(),
            url: String::from(url),
            body: ()
        };

        let response = self.http_handler.deref().head(req)?;

        if response.status_code.to_string().starts_with("4") ||
            !response.headers.contains_key(headers::UPLOAD_OFFSET) {
            return Err(Error::NotFoundError)
        }

        let bytes_uploaded: usize = response.headers.get(headers::UPLOAD_OFFSET).unwrap().parse()?;
        let total_size = response.headers.get(headers::UPLOAD_LENGTH)
            .and_then(|l| l.parse::<usize>().ok());

        Ok(ProgressResponse{
            bytes_uploaded,
            total_size
        })
    }
}

pub struct ProgressResponse {
    bytes_uploaded: usize,
    total_size: Option<usize>,
}

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
