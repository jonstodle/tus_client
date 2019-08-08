# tus_client

A Rust native client library to interact with *tus* enabled endpoints.

## `reqwest` implementation

`tus_client` requires a "handler" which implements the `HttpHandler` trait. To include a default implementation of this trait for [`reqwest`](https://crates.io/crates/reqwest), specify the `reqwest` feature when including `tus_client` as a dependency.

```toml
# Other parts of Cargo.toml omitted for brevity
[dependencies]
tus_client = {version = "x.x.x", features = ["reqwest"]}
```

## Usage

Create an instance of the `tus_client::Client` struct.

```rust
use tus_client::Client;
use reqwest;

// Assumes "reqwest" feature is enabled (see above)
let client = Client::new(reqwest::Client::new());
```

You'll need an upload URL to be able to upload a files. This may be provided to you (through a separate API, for example), or you might need to create the file through the *tus* protocol. If an upload URL is provided for you, you can skip this step.

```rust
let upload_url = client
    .create("https://my.tus.server/files/", "/path/to/file")
    .expect("Failed to create file on server");
```

Next, you can start uploading the file by calling `upload`. The file will be uploaded in 5 MiB chunks by default. To customize the chunk size, use `upload_with_chunk_size` instead of `upload`.

```rust
client
    .upload(&upload_url, "/path/to/file")
    .expect("Failed to upload file to server");
```

`upload` (and `upload_with_chunk_size`) will automatically resume the upload from where it left off, if the upload transfer is interrupted.
