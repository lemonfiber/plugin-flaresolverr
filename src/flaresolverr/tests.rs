use std::net::IpAddr;
use std::sync::Arc;

use lemonfiber_fixtures::http::{Answer, Fake};
use lemonfiber_fixtures::ports::Resolving;
use lemonfiber_ports::http::Method;
use lemonfiber_ports::service::{Cookie, Failure, Solved, Solving as _};
use serde_json::{json, Value};

use super::{FlareSolverr, SERVICE, SOLVE_WITHIN};

const BASE: &str = "http://flaresolverr:8191";

const SITE: &str = "https://indexer.example/";

fn solved() -> String {
    json!({
        "status": "ok",
        "message": "Challenge solved!",
        "solution": {
            "url": "https://indexer.example/browse",
            "status": 200,
            "headers": {},
            "response": "<html>cleared</html>",
            "cookies": [{
                "domain": ".indexer.example",
                "expiry": 1_767_312_000,
                "httpOnly": true,
                "name": "cf_clearance",
                "path": "/",
                "sameSite": "None",
                "secure": true,
                "value": "cleared"
            }],
            "userAgent": "Mozilla/5.0 (X11; Linux x86_64)"
        },
        "startTimestamp": 1,
        "endTimestamp": 2,
        "version": "3.5.2"
    })
    .to_string()
}

/// An address out on the internet that every name stands for.
const PUBLIC: [u8; 4] = [93, 184, 215, 14];

fn solver(fake: &Arc<Fake>) -> FlareSolverr {
    FlareSolverr::new(
        fake.clone(),
        BASE,
        Resolving::standing_for(&[IpAddr::from(PUBLIC)]),
    )
}

fn standing_for(fake: &Arc<Fake>, address: &str) -> FlareSolverr {
    let address: IpAddr = address.parse().unwrap_or(IpAddr::from([127, 0, 0, 1]));
    FlareSolverr::new(fake.clone(), BASE, Resolving::standing_for(&[address]))
}

fn disallowed_saying(answer: &Result<Solved, Failure>, said: &str) -> bool {
    matches!(answer, Err(Failure::Disallowed { service, detail }) if service == "FlareSolverr" && detail.contains(said))
}

#[tokio::test]
async fn a_page_is_fetched_through_its_api_with_the_cookies_that_cleared_it() {
    let fake = Fake::by_route(vec![(Method::Post, "/v1", Answer::reply(200, solved()))]);

    let page = solver(&fake).solve(SITE).await.ok();

    assert_eq!(
        page,
        Some(Solved {
            status: 200,
            url: "https://indexer.example/browse".to_owned(),
            user_agent: "Mozilla/5.0 (X11; Linux x86_64)".to_owned(),
            cookies: vec![Cookie {
                name: "cf_clearance".to_owned(),
                value: "cleared".to_owned(),
                domain: ".indexer.example".to_owned(),
            }],
            page: "<html>cleared</html>".to_owned(),
        })
    );
    let asked = fake.request();
    assert_eq!(
        asked.as_ref().map(|asked| asked.url.as_str()),
        Some("http://flaresolverr:8191/v1")
    );
    let body: Value = asked
        .and_then(|asked| asked.body)
        .and_then(|body| serde_json::from_str(&body).ok())
        .unwrap_or_default();
    assert_eq!(
        body,
        json!({
            "cmd": "request.get",
            "url": SITE,
            "maxTimeout": SOLVE_WITHIN.as_millis(),
        })
    );
}

#[tokio::test]
async fn only_an_http_or_https_address_naming_a_host_by_name_is_fetched() {
    let fake = Fake::always(Answer::reply(200, solved()));

    for (url, said) in [
        ("file:///etc/passwd", "only an http or https address"),
        ("FILE:///etc/passwd", "only an http or https address"),
        ("gopher://indexer.example/", "only an http or https address"),
        ("chrome://settings", "only an http or https address"),
        ("javascript:alert(1)", "only an http or https address"),
        ("data:text/html,<p>", "only an http or https address"),
        ("about:blank", "only an http or https address"),
        ("", "cannot be read"),
        ("http://", "cannot be read"),
        ("https://user:pass@indexer.example/", "a user or a password"),
        ("https://user@indexer.example/", "a user or a password"),
        ("http://127.0.0.1/", "by number"),
        ("http://[::1]/", "by number"),
        ("http://10.0.0.1/", "by number"),
        ("http://192.168.1.1/", "by number"),
        ("http://169.254.169.254/latest/meta-data/", "by number"),
        ("http://0x7f000001/", "by number"),
        ("http://2130706433/", "by number"),
        ("http://0177.0.0.1/", "by number"),
        ("http://127.1/", "by number"),
        ("http://[::ffff:127.0.0.1]/", "by number"),
        ("http://93.184.215.14/", "by number"),
    ] {
        let answer = solver(&fake).solve(url).await;
        assert!(disallowed_saying(&answer, said), "{url}: {answer:?}");
    }
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn a_name_standing_for_this_machine_or_a_network_of_its_own_is_never_fetched() {
    let fake = Fake::always(Answer::reply(200, solved()));

    for address in [
        "127.0.0.1",
        "::1",
        "10.1.2.3",
        "172.16.0.1",
        "192.168.1.1",
        "169.254.169.254",
        "100.64.0.1",
        "fd00::1",
        "fe80::1",
        "::ffff:10.0.0.1",
        "0.0.0.0",
        "255.255.255.255",
        "224.0.0.1",
    ] {
        let answer = standing_for(&fake, address)
            .solve("https://indexer.example/")
            .await;
        assert!(
            disallowed_saying(&answer, "not the internet"),
            "{address}: {answer:?}"
        );
    }
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn localhost_is_never_fetched_however_it_resolves() {
    let fake = Fake::always(Answer::reply(200, solved()));
    let solver = FlareSolverr::new(
        fake.clone(),
        BASE,
        Arc::new(lemonfiber_outward::resolve::Lookup),
    );

    let answer = solver.solve("http://localhost:8989/").await;

    assert!(disallowed_saying(&answer, "not the internet"), "{answer:?}");
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn a_name_that_stands_for_nothing_is_never_fetched() {
    let fake = Fake::always(Answer::reply(200, solved()));
    let unresolved = FlareSolverr::new(fake.clone(), BASE, Resolving::failing("no such name"));
    let nowhere = FlareSolverr::new(fake.clone(), BASE, Resolving::standing_for(&[]));

    let failing = unresolved.solve("https://indexer.example/").await;
    let empty = nowhere.solve("https://indexer.example/").await;

    assert!(
        disallowed_saying(&failing, "could not be resolved"),
        "{failing:?}"
    );
    assert!(
        disallowed_saying(&empty, "stands for no address"),
        "{empty:?}"
    );
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn a_refusal_names_the_reason_and_never_the_address() {
    let fake = Fake::always(Answer::reply(200, solved()));

    let answer = standing_for(&fake, "10.0.0.1")
        .solve("https://secret-host.example/private?token=abc")
        .await;

    let said = format!("{answer:?}");
    assert!(
        !said.contains("secret-host") && !said.contains("token"),
        "{said}"
    );
}

#[tokio::test]
async fn the_address_handed_over_is_the_one_the_check_read() {
    let fake = Fake::always(Answer::reply(200, solved()));

    let answer = solver(&fake)
        .solve("HTTPS://Indexer.Example:443/a/../b")
        .await;

    assert!(answer.is_ok(), "{answer:?}");
    let body: Value = fake
        .request()
        .and_then(|asked| asked.body)
        .and_then(|body| serde_json::from_str(&body).ok())
        .unwrap_or_default();
    assert_eq!(body.get("url"), Some(&json!("https://indexer.example/b")));
}

#[tokio::test]
async fn a_challenge_it_could_not_clear_is_refused_in_its_own_words() {
    let fake = Fake::always(Answer::reply(
        500,
        json!({"status": "error", "message": "Error: Challenge not solved within 50s"}).to_string(),
    ));

    let refused = solver(&fake).solve(SITE).await;

    assert!(
        matches!(&refused, Err(Failure::Refused { service, detail })
            if service == SERVICE && detail.contains("Challenge not solved")),
        "{refused:?}"
    );
}

#[tokio::test]
async fn an_answer_that_is_not_ok_or_carries_no_page_is_refused() {
    for answer in [
        json!({"status": "warning", "message": "Cloudflare has blocked this request"}),
        json!({"status": "ok", "message": "no solution"}),
    ] {
        let fake = Fake::always(Answer::reply(200, answer.to_string()));
        assert!(matches!(
            solver(&fake).solve(SITE).await,
            Err(Failure::Refused { .. })
        ));
    }
}

#[tokio::test]
async fn nothing_answering_is_unavailable() {
    let fake = Fake::silent();

    assert!(matches!(
        solver(&fake).solve(SITE).await,
        Err(Failure::Unavailable { .. })
    ));
    assert!(matches!(
        solver(&fake).answers().await,
        Err(Failure::Unavailable { .. })
    ));
}

#[tokio::test]
async fn it_answers_when_its_greeting_says_it_is_ready() {
    let ready = Fake::by_route(vec![(
        Method::Get,
        "/",
        Answer::reply(
            200,
            json!({"msg": "FlareSolverr is ready!", "version": "3.5.2", "userAgent": "Mozilla/5.0"})
                .to_string(),
        ),
    )]);
    let starting = Fake::always(Answer::reply(200, json!({"msg": "Starting"}).to_string()));
    let other = Fake::always(Answer::reply(200, "<html></html>"));

    assert_eq!(solver(&ready).answers().await.ok(), Some(true));
    assert_eq!(solver(&starting).answers().await.ok(), Some(false));
    assert!(matches!(
        solver(&other).answers().await,
        Err(Failure::Refused { .. })
    ));
    assert_eq!(
        ready.request().map(|asked| asked.url),
        Some("http://flaresolverr:8191/".to_owned())
    );
}
