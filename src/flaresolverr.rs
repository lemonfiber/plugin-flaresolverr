//! FlareSolverr, asked through its API: a page fetched past its challenge by the headless
//! browser it drives, and whether it answers.
//!
//! Every address is screened before the browser is handed it: `http` or `https`, with no
//! user or password, naming its host by a name rather than a number, and a name every
//! address of which `lemonfiber_outward` passes. What is handed over is the address as
//! the screen read it. The screen is a first refusal, not the protection: the browser
//! follows redirects and resolves the name again, and what holds it off every private
//! address and the host's own is the fence FlareSolverr runs inside.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use lemonfiber_adapter_kit::endpoint::{describe, Endpoint};
use lemonfiber_outward::{literal, outward, Inward};
use lemonfiber_ports::http::{Http, Method, Request, JSON};
use lemonfiber_ports::resolve::Resolver;
use lemonfiber_ports::service::{Cookie, Failure, Solved, Solving};
use serde::Deserialize;
use url::Url;

/// What every failure is blamed on.
pub const SERVICE: &str = "FlareSolverr";

/// Where FlareSolverr is asked to fetch a page.
const SOLVE: &str = "/v1";

/// Where FlareSolverr says it is ready.
const GREETING: &str = "/";

/// What FlareSolverr says at its greeting once it is ready.
const READY: &str = "FlareSolverr is ready!";

/// The command that fetches a page.
const GET: &str = "request.get";

/// What FlareSolverr says of a page it fetched.
const SOLVED: &str = "ok";

/// The header a request's body type is named in.
const CONTENT_TYPE: &str = "Content-Type";

/// How long FlareSolverr is given to fetch a page, inside the contract's deadline.
pub const SOLVE_WITHIN: Duration = Duration::from_secs(50);

/// How long the adapter waits for FlareSolverr's answer, a little past what it gives it.
pub const ANSWER_WITHIN: Duration = Duration::from_secs(55);

/// The schemes a page is fetched over.
const FETCHED_OVER: [&str; 2] = ["http", "https"];

/// One FlareSolverr, asked over its API at its address, handed only addresses the screen
/// passes as `resolver` says what a name stands for.
pub struct FlareSolverr {
    endpoint: Endpoint,
    resolver: Arc<dyn Resolver>,
}

#[derive(Deserialize)]
struct Answered {
    status: String,
    #[serde(default)]
    solution: Option<Solution>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Solution {
    url: String,
    status: u16,
    #[serde(default)]
    cookies: Vec<Kept>,
    user_agent: String,
    response: String,
}

#[derive(Deserialize)]
struct Kept {
    name: String,
    value: String,
    domain: String,
}

#[derive(Deserialize)]
struct Greeting {
    msg: String,
}

impl FlareSolverr {
    /// The FlareSolverr at `address`, asked over `http`, every page's host resolved by
    /// `resolver`.
    pub fn new(
        http: Arc<dyn Http>,
        address: impl Into<String>,
        resolver: Arc<dyn Resolver>,
    ) -> Self {
        Self {
            endpoint: Endpoint::new(http, address, SERVICE),
            resolver,
        }
    }

    /// `url` as the screen read it, where it passes as an address out on the internet, or
    /// why it does not, naming the reason and never the address.
    async fn outside(&self, url: &str) -> Result<String, &'static str> {
        let read = Url::parse(url).map_err(|_| "the address cannot be read")?;
        if !FETCHED_OVER.contains(&read.scheme()) {
            return Err("only an http or https address is fetched");
        }
        if !read.username().is_empty() || read.password().is_some() {
            return Err("an address carrying a user or a password is not fetched");
        }
        let host = read.host_str().unwrap_or_default();
        let port = read.port_or_known_default().unwrap_or_default();
        if literal(host).is_some() {
            return Err("an address naming its host by number is not fetched");
        }
        outward(self.resolver.as_ref(), host, port)
            .await
            .map_err(|why| match why {
                Inward::Unresolved(_) => "the host could not be resolved",
                Inward::Nameless | Inward::Nowhere => "the host stands for no address",
                Inward::Internal(_) => {
                    "the host stands for this machine or a network of its own, not the internet"
                }
            })?;
        Ok(read.into())
    }
}

#[async_trait]
impl Solving for FlareSolverr {
    async fn solve(&self, url: &str) -> Result<Solved, Failure> {
        let url = self.outside(url).await.map_err(|why| Failure::Disallowed {
            service: SERVICE.to_owned(),
            detail: why.to_owned(),
        })?;
        let body = serde_json::json!({
            "cmd": GET,
            "url": url,
            "maxTimeout": SOLVE_WITHIN.as_millis(),
        })
        .to_string();
        let response = self
            .endpoint
            .send(&Request {
                method: Method::Post,
                url: self.endpoint.url(SOLVE),
                headers: vec![(CONTENT_TYPE.to_owned(), JSON.to_owned())],
                body: Some(body),
                pinned: None,
            })
            .await?;
        let answered: Answered = self.endpoint.decode(&response, "the page fetched")?;
        match answered.solution {
            Some(solution) if answered.status == SOLVED => Ok(Solved {
                status: solution.status,
                url: solution.url,
                user_agent: solution.user_agent,
                cookies: solution
                    .cookies
                    .into_iter()
                    .map(|kept| Cookie {
                        name: kept.name,
                        value: kept.value,
                        domain: kept.domain,
                    })
                    .collect(),
                page: solution.response,
            }),
            _ => Err(self.endpoint.refused(&describe(&response))),
        }
    }

    async fn answers(&self) -> Result<bool, Failure> {
        let response = self
            .endpoint
            .send(&Request {
                method: Method::Get,
                url: self.endpoint.url(GREETING),
                headers: Vec::new(),
                body: None,
                pinned: None,
            })
            .await?;
        let greeting: Greeting = self.endpoint.decode(&response, "its greeting")?;
        Ok(greeting.msg == READY)
    }
}

#[cfg(test)]
mod tests;
