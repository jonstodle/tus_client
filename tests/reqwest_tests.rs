use std::collections::HashMap;
use std::io::Write;
use tempfile::NamedTempFile;
use tus_client;

const TUS_ENDPOINT: &str = "http://localhost:1080/files/";

fn create_client<'a>() -> tus_client::Client<'a> {
    tus_client::Client::new(reqwest::Client::new())
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
fn reqwest_should_create_file() {
    let temp_file = create_temp_file();
    let client = create_client();

    let result = client
        .create(TUS_ENDPOINT, temp_file.path())
        .expect("'client.create' call failed");

    assert!(!result.is_empty());
    assert!(result.starts_with(TUS_ENDPOINT));
}

#[test]
fn reqwest_should_upload_file() {
    let temp_file = create_temp_file();
    let client = create_client();
    let mut metadata = HashMap::new();
    metadata.insert("filetype".to_string(), "audio/wav".to_string());
    metadata.insert(
        "filename".to_string(),
        format!(
            "{}.{}",
            temp_file.path().file_stem().unwrap().to_str().unwrap(),
            "png"
        ),
    );

    let upload_path = client
        .create_with_metadata(TUS_ENDPOINT, temp_file.path(), metadata)
        .expect("'client.create' call failed");
    client
        .upload(&upload_path, temp_file.path())
        .expect("'client.upload' call failed");
}
