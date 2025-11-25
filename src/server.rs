// HTTP server to serve cached images with correct content-types
// Reason: xAI/Grok API downloads images from URLs we provide. Telegram serves all files
// as application/octet-stream, but Grok requires proper MIME types (image/jpeg, etc.)
// So we cache images locally and serve them with correct content-types.

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn start_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8081));

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });

    let server = Server::bind(&addr).serve(make_svc);

    log::info!(
        "*http server started* serving cached images on http://{}",
        addr
    );
    server.await?;
    Ok(())
}

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let path = req.uri().path().trim_start_matches('/');

    // Only serve files from the images directory
    if !path.starts_with("images/") {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap());
    }

    let file_path = format!("assets/cache/{}", path);

    match File::open(&file_path).await {
        Ok(mut file) => {
            let mut buffer = Vec::new();
            if let Ok(_) = file.read_to_end(&mut buffer).await {
                // Determine content type based on file extension
                let content_type = if path.ends_with(".jpg") || path.ends_with(".jpeg") {
                    "image/jpeg"
                } else if path.ends_with(".png") {
                    "image/png"
                } else if path.ends_with(".webp") {
                    "image/webp"
                } else {
                    "application/octet-stream"
                };

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", content_type)
                    .header("Cache-Control", "public, max-age=31536000") // Cache for 1 year
                    .body(Body::from(buffer))
                    .unwrap())
            } else {
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap())
            }
        }
        Err(_) => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()),
    }
}
