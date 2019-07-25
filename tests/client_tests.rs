use tus_client;
use tus_client::{HttpRequest, HttpHandler, HttpResponse};
use std::collections::HashMap;
use std::io;

struct TestClient {
    pub upload_progress: usize,
    pub total_upload_size: usize,
}

impl HttpHandler for TestClient {
    fn head(&self, req: HttpRequest<()>) -> Result<HttpResponse, io::Error> {
        let mut headers = HashMap::new();
        headers.insert("upload-length".to_owned(), self.total_upload_size.to_string());
        headers.insert("upload-offset".to_owned(), self.upload_progress.to_string());

        Ok(HttpResponse {
            status_code: 204,
            headers,
        })
    }
}

#[test]
fn should_report_correct_upload_progress() {
    let client = tus_client::Client::new(TestClient{
        upload_progress: 1234,
        total_upload_size: 2345,
    });

    let progress = client.get_progress("/something")
        .expect("'get_progress' call failed");

    assert_eq!(1234, progress.bytes_uploaded);
    assert_eq!(2345, progress.total_size.unwrap());
}
