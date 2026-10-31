//! What this adapter says it is, and the kit that serves its contract.

use std::sync::Arc;

use lemonfiber_adapter_kit::{Kit, Readiness};
use lemonfiber_contract::adapter::{About, Release};
use lemonfiber_contract::capabilities::indexer;
use lemonfiber_ports::service::Solving as _;

use crate::flaresolverr::FlareSolverr;

/// Where FlareSolverr answers, by its service id and the port it listens on.
pub const ADDRESS: &str = "http://flaresolverr:8191";

/// The upstream this adapter fronts, as its project names itself.
pub const UPSTREAM: &str = "FlareSolverr";

/// The FlareSolverr release this adapter's recordings were taken from.
pub const RELEASE: &str = "v3.5.2";

/// The digest of the image that release's recordings were taken from.
pub const DIGEST: &str = "sha256:c80ae007ce2ccdcd217a12426e4f039ef763ff90738c808d38810c3e59323767";

/// What this adapter says it is: the contract it speaks and the release it supports.
#[must_use]
pub fn about() -> About {
    About {
        speaks: vec![lemonfiber_contract::spoken(
            indexer::proxy::CAPABILITY,
            indexer::proxy::MAJOR,
        )],
        upstream: UPSTREAM.to_owned(),
        releases: vec![Release {
            version: RELEASE.to_owned(),
            digest: DIGEST.to_owned(),
        }],
    }
}

/// The contract, served from `solver` under `key`, ready while FlareSolverr says it is.
#[must_use]
pub fn kit(key: String, solver: Arc<FlareSolverr>) -> Kit {
    let asked = Arc::clone(&solver);
    let ready: Readiness = Arc::new(move || {
        let asked = Arc::clone(&asked);
        Box::pin(async move { asked.answers().await.unwrap_or(false) })
    });
    Kit::new(key, about())
        .ready_when(ready)
        .serving(indexer::proxy::served(solver))
}

#[cfg(test)]
mod tests;
