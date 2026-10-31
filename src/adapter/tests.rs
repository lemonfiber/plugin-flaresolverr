use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use lemonfiber_contract::adapter::{about_path, ready_path, About};
use lemonfiber_fixtures::http::{Answer, Fake};
use lemonfiber_fixtures::ports::Resolving;
use serde_json::json;
use tower::ServiceExt as _;

use super::{about, kit, DIGEST, RELEASE, UPSTREAM};
use crate::flaresolverr::FlareSolverr;

fn a_key() -> String {
    ('e'..='t').collect()
}

fn solver(fake: &Arc<Fake>) -> Arc<FlareSolverr> {
    Arc::new(FlareSolverr::new(
        fake.clone(),
        "http://flaresolverr:8191",
        Resolving::anywhere(),
    ))
}

async fn answered(
    solver: Arc<FlareSolverr>,
    method: &str,
    path: &str,
    key: Option<&str>,
    body: &str,
) -> (StatusCode, Vec<u8>) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(key) = key {
        request = request.header(header::AUTHORIZATION, format!("Bearer {key}"));
    }
    let Ok(request) = request.body(Body::from(body.to_owned())) else {
        return (StatusCode::IM_A_TEAPOT, Vec::new());
    };
    let Ok(response) = kit(a_key(), solver).router().oneshot(request).await;
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .map(|bytes| bytes.to_vec())
        .unwrap_or_default();
    (status, body)
}

#[test]
fn it_says_it_speaks_indexer_proxy_for_the_flaresolverr_its_recordings_were_taken_from() {
    let about = about();

    assert_eq!(about.speaks, vec!["indexer.proxy@1".to_owned()]);
    assert_eq!(about.upstream, UPSTREAM);
    assert_eq!(
        about.releases.first().map(|one| one.version.as_str()),
        Some(RELEASE)
    );
    assert_eq!(
        about.releases.first().map(|one| one.digest.as_str()),
        Some(DIGEST)
    );
}

#[tokio::test]
async fn it_names_itself_under_the_key_and_to_nobody_else() {
    let fake = Fake::silent();

    let (status, body) = answered(solver(&fake), "GET", &about_path(), Some(&a_key()), "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(serde_json::from_slice::<About>(&body).ok(), Some(about()));

    let (status, _) = answered(solver(&fake), "GET", &about_path(), Some("another"), "").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn it_is_ready_while_flaresolverr_says_it_is() {
    let ready = Fake::always(Answer::reply(
        200,
        json!({"msg": "FlareSolverr is ready!"}).to_string(),
    ));
    let silent = Fake::silent();

    let (_, answering) = answered(solver(&ready), "GET", &ready_path(), Some(&a_key()), "").await;
    let (_, unanswering) =
        answered(solver(&silent), "GET", &ready_path(), Some(&a_key()), "").await;

    assert_eq!(answering, b"true");
    assert_eq!(unanswering, b"false");
}

#[tokio::test]
async fn a_page_asked_for_over_the_contract_is_fetched_through_flaresolverr() {
    let fake = Fake::always(Answer::reply(
        200,
        json!({"status": "ok", "message": "", "solution": {
            "url": "https://indexer.example/", "status": 200, "cookies": [],
            "userAgent": "Mozilla/5.0", "response": "<html></html>"
        }})
        .to_string(),
    ));

    let (status, body) = answered(
        solver(&fake),
        "POST",
        "/lemonfiber/indexer.proxy/v1/solve",
        Some(&a_key()),
        &json!({ "url": "https://indexer.example/" }).to_string(),
    )
    .await;

    assert!(status.is_success(), "{status}");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).ok(),
        Some(json!({
            "status": 200, "url": "https://indexer.example/", "user_agent": "Mozilla/5.0",
            "cookies": [], "page": "<html></html>"
        }))
    );
}
