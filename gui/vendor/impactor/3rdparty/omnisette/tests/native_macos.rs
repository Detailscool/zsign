#![cfg(target_os = "macos")]
use omnisette::aos_kit::AOSKitAnisetteProvider;
use omnisette::anisette_headers_provider::AnisetteHeadersProvider;

#[tokio::test]
async fn native_provider_reports_unavailable_data_without_panicking() {
    if let Ok(mut provider) = AOSKitAnisetteProvider::new() {
        match provider.get_authentication_headers().await {
            Ok(headers) => {
                assert!(!headers.get("X-Apple-I-MD").unwrap_or(&String::new()).is_empty());
                assert!(!headers.get("X-Apple-I-MD-M").unwrap_or(&String::new()).is_empty());
                println!("native Anisette is available");
            },
            Err(_) => println!("native Anisette is unavailable on this host"),
        }
    }
}
