use crate::http::{HttpHandler, HttpRequest, default_headers};
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

        let bytes_uploaded = response.headers.get_by_key(headers::UPLOAD_OFFSET);
        let total_size = response.headers.get_by_key(headers::UPLOAD_LENGTH)
            .and_then(|l| l.parse::<usize>().ok());

        if response.status_code.to_string().starts_with("4") ||
            bytes_uploaded.is_none() {
            return Err(Error::NotFoundError);
        }

        let bytes_uploaded = bytes_uploaded.unwrap().parse()?;

        Ok(ProgressResponse {
            bytes_uploaded,
            total_size,
        })
    }

    /// Get information about the tus server
    pub fn get_server_info(&self, url: &str) -> Result<ServerInfo, Error> {
        let req = HttpRequest {
            headers: HashMap::new(),
            url: String::from(url),
            body: (),
        };

        let response = self.http_handler.deref().options(req)?;

        if ![200_usize, 204].contains(&response.status_code) {
            return Err(Error::BadResponse);
        }

        let supported_versions: Vec<String> = response.headers.get_by_key(headers::TUS_VERSION).unwrap().split(',')
            .map(String::from)
            .collect();
        let extensions: Vec<TusExtension> = if let Some(ext) = response.headers.get_by_key(headers::TUS_EXTENSION) {
            ext.to_lowercase().split(',')
                .map(|e| match e.trim() {
                    "creation" => Some(TusExtension::Creation),
                    "expiration" => Some(TusExtension::Expiration),
                    "checksum" => Some(TusExtension::Checksum),
                    "termination" => Some(TusExtension::Termination),
                    "concatenation" => Some(TusExtension::Concatenation),
                    _ => None
                })
                .filter(Option::is_some)
                .map(Option::unwrap)
                .collect()
        } else {
            Vec::new()
        };
        let max_upload_size = response.headers.get_by_key(headers::TUS_MAX_SIZE)
            .and_then(|h| h.parse::<usize>().ok());

        Ok(ServerInfo {
            supported_versions,
            extensions,
            max_upload_size,
        })
    }
}

#[derive(Debug)]
pub struct ProgressResponse {
    pub bytes_uploaded: usize,
    pub total_size: Option<usize>,
}

#[derive(Debug)]
pub struct ServerInfo {
    pub supported_versions: Vec<String>,
    pub extensions: Vec<TusExtension>,
    pub max_upload_size: Option<usize>,
}

#[derive(Debug, PartialEq)]
pub enum TusExtension {
    Creation,
    Expiration,
    Checksum,
    Termination,
    Concatenation,
}

#[derive(Debug)]
pub enum Error {
    NotFoundError,
    BadResponse,
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

trait HeaderMap {
    fn get_by_key(&self, key: &str) -> Option<&String>;
}

impl HeaderMap for HashMap<String, String> {
    fn get_by_key(&self, key: &str) -> Option<&String> {
        self.keys()
            .find(|k| k.to_lowercase().as_str() == key)
            .and_then(|k| self.get(k))
    }
}
