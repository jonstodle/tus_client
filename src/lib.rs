//! # tus_client
//!
//! A Rust native client library to interact with *tus* enabled endpoints.
//!
//! ## `reqwest` implementation
//!
//! `tus_client` requires a "handler" which implements the `HttpHandler` trait. To include a default implementation of this trait for [`reqwest`](https://crates.io/crates/reqwest), specify the `reqwest` feature when including `tus_client` as a dependency.
//!
//! ```toml
//! # Other parts of Cargo.toml omitted for brevity
//! [dependencies]
//! tus_client = {version = "x.x.x", features = ["reqwest"]}
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use tus_client::Client;
//! use reqwest;
//!
//! // Create an instance of the `tus_client::Client` struct.
//! // Assumes "reqwest" feature is enabled (see above)
//! let client = Client::new(reqwest::Client::new());
//!
//! // You'll need an upload URL to be able to upload a files.
//! // This may be provided to you (through a separate API, for example),
//! // or you might need to create the file through the *tus* protocol.
//! // If an upload URL is provided for you, you can skip this step.
//!
//! let upload_url = client
//! .create("https://my.tus.server/files/", "/path/to/file")
//! .expect("Failed to create file on server");
//!
//! // Next, you can start uploading the file by calling `upload`.
//! // The file will be uploaded in 5 MiB chunks by default.
//! // To customize the chunk size, use `upload_with_chunk_size` instead of `upload`.
//!
//! client
//! .upload(&upload_url, "/path/to/file")
//! .expect("Failed to upload file to server");
//! ```
//!
//! `upload` (and `upload_with_chunk_size`) will automatically resume the upload from where it left off, if the upload transfer is interrupted.
use crate::http::{default_headers, Headers, HttpHandler, HttpMethod, HttpRequest};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::num::ParseIntError;
use std::ops::Deref;
use std::path::Path;
use std::str::FromStr;

mod headers;
/// Contains the `HttpHandler` trait and related structs. This module is only relevant when implement `HttpHandler` manually.
pub mod http;

#[cfg(feature = "reqwest")]
mod reqwest;

const DEFAULT_CHUNK_SIZE: usize = 5 * 1024 * 1024;

/// Used to interact with a [tus](https://tus.io) endpoint.
pub struct Client<'a> {
    use_method_override: bool,
    http_handler: Box<dyn HttpHandler + 'a>,
}

impl<'a> Client<'a> {
    /// Instantiates a new instance of `Client`. `http_handler` needs to implement the `HttpHandler` trait.
    /// A default implementation of this trait for the `reqwest` library is available by enabling the `reqwest` feature.
    pub fn new(http_handler: impl HttpHandler + 'a) -> Self {
        Client {
            use_method_override: false,
            http_handler: Box::new(http_handler),
        }
    }

    /// Some environments might not support using the HTTP methods `PATCH` and `DELETE`. Use this method to create a `Client` which uses the `X-HTTP-METHOD-OVERRIDE` header to specify these methods instead.
    pub fn with_method_override(http_handler: impl HttpHandler + 'a) -> Self {
        Client {
            use_method_override: true,
            http_handler: Box::new(http_handler),
        }
    }

    /// Get info about a file on the server.
    pub fn get_info(&self, url: &str) -> Result<UploadInfo, Error> {
        let req = self.create_request(HttpMethod::Head, url, None, Some(default_headers()));

        let response = self.http_handler.deref().handle_request(req)?;

        let bytes_uploaded = response.headers.get_by_key(headers::UPLOAD_OFFSET);
        let total_size = response
            .headers
            .get_by_key(headers::UPLOAD_LENGTH)
            .and_then(|l| l.parse::<usize>().ok());
        let metadata = response
            .headers
            .get_by_key(headers::UPLOAD_METADATA)
            .and_then(|data| base64::decode(data).ok())
            .map(|decoded| {
                String::from_utf8(decoded).unwrap().split(';').fold(
                    HashMap::new(),
                    |mut acc, key_val| {
                        let mut parts = key_val.splitn(2, ':');
                        if let Some(key) = parts.next() {
                            acc.insert(
                                String::from(key),
                                String::from(parts.next().unwrap_or_default()),
                            );
                        }
                        acc
                    },
                )
            });

        if response.status_code.to_string().starts_with('4') || bytes_uploaded.is_none() {
            return Err(Error::NotFoundError);
        }

        let bytes_uploaded = bytes_uploaded.unwrap().parse()?;

        Ok(UploadInfo {
            bytes_uploaded,
            total_size,
            metadata,
        })
    }

    /// Upload a file to the specified upload URL.
    pub fn upload(&self, url: &str, path: &Path) -> Result<(), Error> {
        self.upload_with_chunk_size(url, path, DEFAULT_CHUNK_SIZE)
    }

    /// Upload a file to the specified upload URL with the given chunk size.
    pub fn upload_with_chunk_size(
        &self,
        url: &str,
        path: &Path,
        chunk_size: usize,
    ) -> Result<(), Error> {
        let info = self.get_info(url)?;
        let file = File::open(path)?;
        let file_len = file.metadata()?.len();

        if let Some(total_size) = info.total_size {
            if file_len as usize != total_size {
                return Err(Error::UnequalSizeError);
            }
        }

        let mut reader = BufReader::new(&file);
        let mut buffer = vec![0; chunk_size];
        let mut progress = info.bytes_uploaded;

        reader.seek(SeekFrom::Start(progress as u64))?;

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                return Err(Error::FileReadError);
            }

            let req = self.create_request(
                HttpMethod::Patch,
                url,
                Some(&buffer[..bytes_read]),
                Some(create_upload_headers(progress)),
            );

            let response = self.http_handler.deref().handle_request(req)?;

            if response.status_code == 409 {
                return Err(Error::WrongUploadOffsetError);
            }

            if response.status_code == 404 {
                return Err(Error::NotFoundError);
            }

            if response.status_code != 204 {
                return Err(Error::UnexpectedStatusCode(response.status_code));
            }

            let upload_offset = match response.headers.get_by_key(headers::UPLOAD_OFFSET) {
                Some(offset) => Ok(offset),
                None => Err(Error::MissingHeader(headers::UPLOAD_OFFSET.to_owned())),
            }?;

            progress = upload_offset.parse()?;

            if progress >= file_len as usize {
                break;
            }
        }

        Ok(())
    }

    /// Get information about the tus server
    pub fn get_server_info(&self, url: &str) -> Result<ServerInfo, Error> {
        let req = self.create_request(HttpMethod::Options, url, None, None);

        let response = self.http_handler.deref().handle_request(req)?;

        if ![200_usize, 204].contains(&response.status_code) {
            return Err(Error::UnexpectedStatusCode(response.status_code));
        }

        let supported_versions: Vec<String> = response
            .headers
            .get_by_key(headers::TUS_VERSION)
            .unwrap()
            .split(',')
            .map(String::from)
            .collect();
        let extensions: Vec<TusExtension> =
            if let Some(ext) = response.headers.get_by_key(headers::TUS_EXTENSION) {
                ext.split(',')
                    .map(str::parse)
                    .filter(Result::is_ok)
                    .map(Result::unwrap)
                    .collect()
            } else {
                Vec::new()
            };
        let max_upload_size = response
            .headers
            .get_by_key(headers::TUS_MAX_SIZE)
            .and_then(|h| h.parse::<usize>().ok());

        Ok(ServerInfo {
            supported_versions,
            extensions,
            max_upload_size,
        })
    }

    /// Create a file on the server, receiving the upload URL of the file.
    pub fn create(&self, url: &str, path: &Path) -> Result<String, Error> {
        self.create_with_metadata(url, path, HashMap::new())
    }

    /// Create a file on the server including the specified metadata, receiving the upload URL of the file.
    pub fn create_with_metadata(
        &self,
        url: &str,
        path: &Path,
        metadata: HashMap<String, String>,
    ) -> Result<String, Error> {
        let mut headers = default_headers();
        headers.insert(
            headers::UPLOAD_LENGTH.to_owned(),
            path.metadata()?.len().to_string(),
        );
        if !metadata.is_empty() {
            let data = metadata
                .iter()
                .map(|(key, value)| format!("{} {}", key, base64::encode(value)))
                .collect::<Vec<_>>()
                .join(",");
            headers.insert(headers::UPLOAD_METADATA.to_owned(), data);
        }

        let req = self.create_request(HttpMethod::Post, url, None, Some(headers));

        let response = self.http_handler.deref().handle_request(req)?;

        if response.status_code == 413 {
            return Err(Error::FileTooLarge);
        }

        if response.status_code != 201 {
            return Err(Error::UnexpectedStatusCode(response.status_code));
        }

        let location = response.headers.get_by_key(headers::LOCATION);

        if location.is_none() {
            return Err(Error::MissingHeader(headers::LOCATION.to_owned()));
        }

        Ok(location.unwrap().to_owned())
    }

    /// Delete a file on the server.
    pub fn delete(&self, url: &str) -> Result<(), Error> {
        let req = self.create_request(HttpMethod::Delete, url, None, Some(default_headers()));

        let response = self.http_handler.deref().handle_request(req)?;

        if response.status_code != 204 {
            return Err(Error::UnexpectedStatusCode(response.status_code));
        }

        Ok(())
    }

    fn create_request<'b>(
        &self,
        method: HttpMethod,
        url: &str,
        body: Option<&'b [u8]>,
        headers: Option<Headers>,
    ) -> HttpRequest<'b> {
        let mut headers = headers.unwrap_or_default();

        let method = if self.use_method_override {
            headers.insert(
                headers::X_HTTP_METHOD_OVERRIDE.to_owned(),
                method.to_string(),
            );
            HttpMethod::Post
        } else {
            method
        };

        HttpRequest {
            method,
            url: String::from(url),
            body,
            headers,
        }
    }
}

/// Describes a file on the server.
#[derive(Debug)]
pub struct UploadInfo {
    /// How many bytes have been uploaded.
    pub bytes_uploaded: usize,
    /// The total size of the file.
    pub total_size: Option<usize>,
    /// Metadata supplied when the file was created.
    pub metadata: Option<HashMap<String, String>>,
}

/// Describes the tus enabled server.
#[derive(Debug)]
pub struct ServerInfo {
    /// The different versions of the tus protocol supported by the server, ordered by preference.
    pub supported_versions: Vec<String>,
    /// The extensions to the protocol supported by the server.
    pub extensions: Vec<TusExtension>,
    /// The maximum supported total size of a file.
    pub max_upload_size: Option<usize>,
}

/// Enumerates the extensions to the tus protocol.
#[derive(Debug, PartialEq)]
pub enum TusExtension {
    /// The server supports creating files.
    Creation,
    //// The server supports setting expiration time on files and uploads.
    Expiration,
    /// The server supports verifying checksums of uploaded chunks.
    Checksum,
    /// The server supports deleting files.
    Termination,
    /// The server supports parallel uploads of a single file.
    Concatenation,
}

impl FromStr for TusExtension {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "creation" => Ok(TusExtension::Creation),
            "expiration" => Ok(TusExtension::Expiration),
            "checksum" => Ok(TusExtension::Checksum),
            "termination" => Ok(TusExtension::Termination),
            "concatenation" => Ok(TusExtension::Concatenation),
            _ => Err(()),
        }
    }
}

/// Enumerates the errors which can occur during operation
#[derive(Debug)]
pub enum Error {
    /// The status code returned by the server was not one of the expected ones.
    UnexpectedStatusCode(usize),
    /// The file specified was not found by the server.
    NotFoundError,
    /// A required header was missing from the server response.
    MissingHeader(String),
    /// An error occurred while doing disk IO. This may be while reading a file, or during a network call.
    IoError(io::Error),
    /// Unable to parse a value, which should be an integer.
    ParsingError(ParseIntError),
    /// The size of the specified file, and the file size reported by the server do not match.
    UnequalSizeError,
    /// Unable to read the file specified.
    FileReadError,
    /// The `Client` tried to upload the file with an incorrect offset.
    WrongUploadOffsetError,
    /// The specified file is larger that what is supported by the server.
    FileTooLarge,
    /// An error occurred in the HTTP handler.
    HttpHandlerError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        let message = match self {
            Error::UnexpectedStatusCode(status_code) => format!("The status code returned by the server was not one of the expected ones: {}", status_code),
            Error::NotFoundError => "The file specified was not found by the server".to_string(),
            Error::MissingHeader(header_name) => format!("The '{}' header was missing from the server response", header_name),
            Error::IoError(error) => format!("An error occurred while doing disk IO. This may be while reading a file, or during a network call: {}", error),
            Error::ParsingError(error) => format!("Unable to parse a value, which should be an integer: {}", error),
            Error::UnequalSizeError => "The size of the specified file, and the file size reported by the server do not match".to_string(),
            Error::FileReadError => "Unable to read the specified file".to_string(),
            Error::WrongUploadOffsetError => "The client tried to upload the file with an incorrect offset".to_string(),
            Error::FileTooLarge => "The specified file is larger that what is supported by the server".to_string(),
            Error::HttpHandlerError(message) => format!("An error occurred in the HTTP handler: {}", message),
        };

        write!(f, "{}", message)?;

        Ok(())
    }
}

impl StdError for Error {}

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

fn create_upload_headers(progress: usize) -> Headers {
    let mut headers = default_headers();
    headers.insert(
        headers::CONTENT_TYPE.to_owned(),
        "application/offset+octet-stream".to_owned(),
    );
    headers.insert(headers::UPLOAD_OFFSET.to_owned(), progress.to_string());
    headers
}
