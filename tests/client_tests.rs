use tus_client;
use tus_client::http::{HttpRequest, HttpHandler, HttpResponse};
use std::collections::HashMap;
use std::io;
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
    fn head(&self, req: HttpRequest<()>) -> Result<HttpResponse, io::Error> {
        let mut headers = HashMap::new();
        headers.insert("upload-length".to_owned(), self.total_upload_size.to_string());
        headers.insert("upload-offset".to_owned(), self.upload_progress.to_string());

        Ok(HttpResponse {
            status_code: self.status_code,
            headers,
        })
    }

    fn options(&self, req: HttpRequest<()>) -> Result<HttpResponse, io::Error> {
        let mut headers = HashMap::new();
        headers.insert("tus-version".to_owned(), self.tus_version.clone());
        headers.insert("tus-extension".to_owned(), self.extensions.clone());
        headers.insert("tus-max-size".to_owned(), self.max_upload_size.to_string());

        Ok(HttpResponse {
            status_code: self.status_code,
            headers,
        })
    }
}

#[test]
fn should_report_correct_upload_progress() {
    let client = tus_client::Client::new(TestHandler {
        status_code: 204,
        ..TestHandler::default()
    });

    let progress = client.get_progress("/something")
        .expect("'get_progress' call failed");

    assert_eq!(1234, progress.bytes_uploaded);
    assert_eq!(2345, progress.total_size.unwrap());
}

#[test]
fn should_return_not_found_at_4xx_status() {
    let client = tus_client::Client::new(TestHandler {
        status_code: 400,
        ..TestHandler::default()
    });

    let result = client.get_progress("/something");

    assert!(result.is_err());
    match result {
        Err(tus_client::Error::NotFoundError) => {}
        _ => panic!("Expected 'Error::NotFoundError'")
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

    let result = client.get_server_info("/something")
        .expect("'get_server_info' call failed");

    assert_eq!(vec!["1.0.0", "0.2.2"], result.supported_versions);
    assert_eq!(vec![TusExtension::Creation, TusExtension::Termination], result.extensions);
    assert_eq!(12345, result.max_upload_size.unwrap());
}
