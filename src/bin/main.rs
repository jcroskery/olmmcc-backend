use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;

async fn handle_request(request: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());

    match request.method() {
        &Method::POST => {
            let url = request.uri().to_string();
            let request_vector = &hyper::body::to_bytes(request.into_body()).await?.to_vec();
            let request_body = std::str::from_utf8(request_vector).unwrap();
            if let Ok::<HashMap<&str, String>, _>(string_body_hash) =
                serde_json::from_str(request_body)
            {
                let body_hash = string_body_hash
                    .iter()
                    .map(|(k, v)| (*k, v.as_str()))
                    .collect();
                let response_body = olmmcc::formulate_response(&url, body_hash).await;
                *response.body_mut() = Body::from(response_body);
            } else {
                *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
                *response.body_mut() = Body::from(
                    "The OLMMCC api only supports application/x-www-form-urlencoded.".to_string(),
                );
            }
        }
        _ => {
            *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
            *response.body_mut() = Body::from("The OLMMCC api only supports POST.".to_string());
        }
    }
    Ok(response)
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}
