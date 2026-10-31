//! The manifest, as the core at the pinned commit reads it.

fn manifest() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/plugin.toml")).unwrap_or_default()
}

#[test]
fn the_pinned_core_refuses_nothing_in_the_manifest() {
    let read: Result<lemonfiber_plugin::Manifest, _> = toml::from_str(&manifest());
    let refused: Vec<String> = match read {
        Ok(manifest) => lemonfiber_plugin::refusals(&manifest, &[])
            .into_iter()
            .map(|violation| format!("{}: {}", violation.location, violation.message))
            .collect(),
        Err(why) => vec![why.to_string()],
    };
    assert!(refused.is_empty(), "{refused:#?}");
}

fn unfenced_proxies(manifest: &str) -> Vec<String> {
    toml::from_str::<lemonfiber_plugin::Manifest>(manifest).map_or_else(
        |why| vec![why.to_string()],
        |manifest| {
            manifest
                .services
                .into_iter()
                .filter(|service| {
                    service.provides.iter().any(|capability| {
                        capability == lemonfiber_contract::capabilities::indexer::proxy::CAPABILITY
                    })
                })
                .filter(|service| {
                    !service
                        .network
                        .as_ref()
                        .is_some_and(|network| network.fenced)
                })
                .map(|service| service.id)
                .collect()
        },
    )
}

#[test]
fn the_service_that_fetches_for_indexers_runs_fenced() {
    assert_eq!(unfenced_proxies(&manifest()), Vec::<String>::new());
}

#[test]
fn a_manifest_whose_indexer_proxy_service_is_not_fenced_is_refused() {
    let unfenced = manifest().replace("network     = { fenced = true }\n", "");
    assert_ne!(unfenced, manifest());
    assert_eq!(unfenced_proxies(&unfenced), vec!["flaresolverr".to_owned()]);
}
