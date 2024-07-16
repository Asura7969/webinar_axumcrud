use axum::{
    body::{Body, Bytes},
    extract::{MatchedPath, Request},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Router,
};
use http_body_util::BodyExt;
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::{info, info_span, Span};

use axum::body::HttpBody;
use std::time::Duration;
use uuid::Uuid;

pub async fn print_request(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print("request", body).await?;
    let req = Request::from_parts(parts, Body::from(bytes));
    let res = next.run(req).await;
    Ok(res)
}

async fn buffer_and_print<B>(direction: &str, body: B) -> Result<Bytes, (StatusCode, String)>
where
    B: HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read {direction} body: {err}"),
            ));
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        if !body.is_empty() {
            info!("{direction}: {body:?}");
        }
    }

    Ok(bytes)
}

pub fn trace_layer(router: Router) -> Router {
    // https://docs.rs/tower-http/latest/tower_http/trace/index.html#customization
    router
        .route_layer(middleware::from_fn(print_request))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|_request: &axum::http::Request<_>| {
                    // Log the matched route's path (with placeholders not filled in).
                    // Use request.uri() or OriginalUri if you want the real path.

                    let id = Uuid::new_v4();
                    info_span!(
                        "",
                        req_id = ?id,
                        some_other_field = tracing::field::Empty,
                    )
                })
                .on_request(|request: &axum::http::Request<_>, _span: &Span| {
                    let path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);

                    info!(
                        "method: {:?}, path: {:?}",
                        request.method(),
                        path.unwrap_or("")
                    );
                    // You can use `_span.record("some_other_field", value)` in one of these
                    // closures to attach a value to the initially empty field in the info_span
                    // created above.
                })
                .on_response(|response: &Response, latency: Duration, _span: &Span| {
                    info!(
                        "status_code: {:?}, latency: {:?}ms",
                        &display(response.status()),
                        latency.as_millis()
                    );
                    // ...
                })
                .on_body_chunk(|chunk: &Bytes, _latency: Duration, _span: &Span| {
                    if let Ok(resp_body) = std::str::from_utf8(chunk.iter().as_slice()) {
                        if !resp_body.is_empty() {
                            info!("response: {:?}", resp_body);
                        }
                    }
                    // ...
                })
                .on_eos(
                    |_trailers: Option<&HeaderMap>, _stream_duration: Duration, _span: &Span| {
                        // ...
                    },
                )
                .on_failure(
                    |_error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        // ...
                    },
                ),
        )
}
