use omnisette::{AnisetteConfiguration, AnisetteHeaders, AnisetteHeadersProviderType};

#[test]
fn unavailable_local_anisette_does_not_silently_use_remote_server() {
    let path = std::env::temp_dir().join(format!("zsign-no-anisette-{}", std::process::id()));
    let result = AnisetteHeaders::get_anisette_headers_provider(
        AnisetteConfiguration::new().set_configuration_path(path),
    );
    assert!(result.is_err(), "remote service must require explicit consent");
}

#[test]
fn remote_provider_requires_an_explicit_https_address() {
    assert!(AnisetteConfiguration::new().allow_remote("http://example.com".into()).is_err());
    assert!(AnisetteConfiguration::new().allow_remote("https://user:password@example.com".into()).is_err());
    let config = AnisetteConfiguration::new()
        .set_configuration_path(std::env::temp_dir().join("zsign-missing-adi"))
        .allow_remote("https://example.com".into()).unwrap();
    let result = AnisetteHeaders::get_anisette_headers_provider(config).unwrap();
    assert!(matches!(result.provider_type, AnisetteHeadersProviderType::Remote));
}
