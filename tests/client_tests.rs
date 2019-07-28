use std::collections::HashMap;
use std::io;
use std::io::Write;
use tempfile::NamedTempFile;
use tus_client;
use tus_client::http::{HttpHandler, HttpMethod, HttpRequest, HttpResponse};
use tus_client::TusExtension;

struct TestHandler {
    pub upload_progress: usize,
    pub total_upload_size: usize,
    pub status_code: usize,
    pub tus_version: String,
    pub extensions: String,
    pub max_upload_size: usize,
}

impl Default for TestHandler {
    fn default() -> Self {
        TestHandler {
            upload_progress: 1234,
            total_upload_size: 2345,
            status_code: 200,
            tus_version: String::from("1.0.0"),
            extensions: String::from(""),
            max_upload_size: 12345,
        }
    }
}

impl HttpHandler for TestHandler {
    fn handle_request(&self, req: HttpRequest) -> Result<HttpResponse, io::Error> {
        match &req.method {
            HttpMethod::Head => {
                let mut headers = HashMap::new();
                headers.insert(
                    "upload-length".to_owned(),
                    self.total_upload_size.to_string(),
                );
                headers.insert("upload-offset".to_owned(), self.upload_progress.to_string());
                headers.insert(
                    "upload-metadata".to_owned(),
                    base64::encode("key_one:value_one;key_two:value_two;k"),
                );

                Ok(HttpResponse {
                    status_code: self.status_code,
                    headers,
                })
            }
            HttpMethod::Options => {
                let mut headers = HashMap::new();
                headers.insert("tus-version".to_owned(), self.tus_version.clone());
                headers.insert("tus-extension".to_owned(), self.extensions.clone());
                headers.insert("tus-max-size".to_owned(), self.max_upload_size.to_string());

                Ok(HttpResponse {
                    status_code: self.status_code,
                    headers,
                })
            }
            HttpMethod::Patch => {
                let mut headers = HashMap::new();
                headers.insert("tus-version".to_owned(), self.tus_version.clone());
                headers.insert(
                    "upload-offset".to_owned(),
                    (req.body.unwrap().len()
                        + req
                            .headers
                            .get("upload-offset")
                            .unwrap()
                            .parse::<usize>()
                            .unwrap())
                    .to_string(),
                );

                Ok(HttpResponse {
                    status_code: self.status_code,
                    headers,
                })
            }
            HttpMethod::Post => {
                let mut headers = HashMap::new();
                headers.insert("tus-version".to_owned(), self.tus_version.clone());
                headers.insert("location".to_owned(), "/something_else".to_owned());

                Ok(HttpResponse {
                    status_code: self.status_code,
                    headers,
                })
            }
            HttpMethod::Delete => {
                let mut headers = HashMap::new();
                headers.insert("tus-version".to_owned(), self.tus_version.clone());

                Ok(HttpResponse {
                    status_code: self.status_code,
                    headers,
                })
            }
            _ => unreachable!(),
        }
    }
}

fn create_temp_file() -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().unwrap();
    let buffer: Vec<u8> = (0..(1024 * 763)).map(|_| rand::random::<u8>()).collect();
    for _ in 0..20 {
        temp_file.write_all(&buffer[..]).unwrap();
    }
    temp_file
}

#[test]
fn should_report_correct_upload_progress() {
    let client = tus_client::Client::new(TestHandler {
        status_code: 204,
        ..TestHandler::default()
    });

    let info = client
        .get_info("/something")
        .expect("'get_progress' call failed");

    let metadata = info.metadata.unwrap();
    assert_eq!(1234, info.bytes_uploaded);
    assert_eq!(2345, info.total_size.unwrap());
    assert_eq!(
        String::from("value_one"),
        metadata.get("key_one").unwrap().to_owned()
    );
    assert_eq!(
        String::from("value_two"),
        metadata.get("key_two").unwrap().to_owned()
    );
}

#[test]
fn should_return_not_found_at_4xx_status() {
    let client = tus_client::Client::new(TestHandler {
        status_code: 400,
        ..TestHandler::default()
    });

    let result = client.get_info("/something");

    assert!(result.is_err());
    match result {
        Err(tus_client::Error::NotFoundError) => {}
        _ => panic!("Expected 'Error::NotFoundError'"),
    }
}

#[test]
fn should_return_server_info() {
    let client = tus_client::Client::new(TestHandler {
        status_code: 204,
        tus_version: String::from("1.0.0,0.2.2"),
        extensions: String::from("creation, termination"),
        ..TestHandler::default()
    });

    let result = client
        .get_server_info("/something")
        .expect("'get_server_info' call failed");

    assert_eq!(vec!["1.0.0", "0.2.2"], result.supported_versions);
    assert_eq!(
        vec![TusExtension::Creation, TusExtension::Termination],
        result.extensions
    );
    assert_eq!(12345, result.max_upload_size.unwrap());
}

#[test]
fn should_upload_file() {
    let temp_file = create_temp_file();

    let client = tus_client::Client::new(TestHandler {
        upload_progress: 0,
        total_upload_size: temp_file.as_file().metadata().unwrap().len() as usize,
        status_code: 204,
        ..TestHandler::default()
    });

    client
        .upload("/something", temp_file.path())
        .expect("'upload' call failed");
}

#[test]
fn should_upload_file_with_custom_chunk_size() {
    let temp_file = create_temp_file();

    let client = tus_client::Client::new(TestHandler {
        upload_progress: 0,
        total_upload_size: temp_file.as_file().metadata().unwrap().len() as usize,
        status_code: 204,
        ..TestHandler::default()
    });

    client
        .upload_with_chunk_size("/something", temp_file.path(), 9 * 87 * 65 * 43)
        .expect("'upload_with_chunk_size' call failed");
}

#[test]
fn should_receive_upload_path() {
    let temp_file = create_temp_file();

    let client = tus_client::Client::new(TestHandler {
        status_code: 201,
        ..TestHandler::default()
    });

    let mut metadata = HashMap::new();
    metadata.insert("key_one".to_owned(), "value_one".to_owned());
    metadata.insert("key_two".to_owned(), "value_two".to_owned());

    let result = client
        .create("/something", temp_file.path())
        .expect("'create_with_metadata' call failed");

    assert!(!result.is_empty());
}

#[test]
fn should_receive_upload_path_with_metadata() {
    let temp_file = create_temp_file();

    let client = tus_client::Client::new(TestHandler {
        status_code: 201,
        ..TestHandler::default()
    });

    let mut metadata = HashMap::new();
    metadata.insert("key_one".to_owned(), "value_one".to_owned());
    metadata.insert("key_two".to_owned(), "value_two".to_owned());

    let result = client
        .create_with_metadata("/something", temp_file.path(), metadata)
        .expect("'create_with_metadata' call failed");

    assert!(!result.is_empty());
}

#[test]
fn should_receive_204_after_deleting_file() {
    let client = tus_client::Client::new(TestHandler {
        status_code: 204,
        ..TestHandler::default()
    });

    client.delete("/something").expect("'delete' call failed");
}
