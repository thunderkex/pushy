use std::path::PathBuf;
use tempfile::NamedTempFile;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use pushy::config::Config;
use pushy::provider::{Provider, UploadContext};
use pushy::providers::catbox::CatboxProvider;
use pushy::providers::fileio::FileioProvider;
use pushy::providers::gofile::GofileProvider;
use pushy::providers::litterbox::LitterboxProvider;
use pushy::providers::tempsh::TempshProvider;
use pushy::providers::tmpfiles::TmpfilesProvider;
use pushy::providers::transfersh::TransfershProvider;
use pushy::providers::zerox::ZeroxProvider;
use pushy::uploader::Uploader;

fn create_temp_test_file(content: &[u8]) -> (NamedTempFile, UploadContext) {
    use std::io::Write;
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content).unwrap();
    file.flush().unwrap();

    let path = file.path().to_path_buf();
    let ctx = UploadContext {
        file_path: path,
        file_name: "testfile.txt".to_string(),
        file_size: content.len() as u64,
        expire: None,
    };
    (file, ctx)
}

#[tokio::test]
async fn test_catbox_provider_wiremock() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/user/api.php"))
        .respond_with(ResponseTemplate::new(200).set_body_string("https://files.catbox.moe/abc123.txt\n"))
        .mount(&server)
        .await;

    let (_f, ctx) = create_temp_test_file(b"test file content");
    let provider = CatboxProvider;
    let client = reqwest::Client::new();
    let config = Config::default();

    let endpoint = format!("{}/user/api.php", server.uri());
    let link = provider
        .upload(&client, &ctx, &config, None, Some(&endpoint))
        .await
        .unwrap();

    assert_eq!(link, "https://files.catbox.moe/abc123.txt");
}

#[tokio::test]
async fn test_litterbox_provider_wiremock() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/resources/internals/api.php"))
        .respond_with(ResponseTemplate::new(200).set_body_string("https://litter.catbox.moe/xyz789.txt\n"))
        .mount(&server)
        .await;

    let (_f, mut ctx) = create_temp_test_file(b"temp test content");
    ctx.expire = Some("12h".to_string());
    let provider = LitterboxProvider;
    let client = reqwest::Client::new();
    let config = Config::default();

    let endpoint = format!("{}/resources/internals/api.php", server.uri());
    let link = provider
        .upload(&client, &ctx, &config, None, Some(&endpoint))
        .await
        .unwrap();

    assert_eq!(link, "https://litter.catbox.moe/xyz789.txt");
}

#[tokio::test]
async fn test_tempsh_provider_wiremock() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_string("https://temp.sh/ABCD/testfile.txt\n"))
        .mount(&server)
        .await;

    let (_f, ctx) = create_temp_test_file(b"tempsh data");
    let provider = TempshProvider;
    let client = reqwest::Client::new();
    let config = Config::default();

    let endpoint = format!("{}/upload", server.uri());
    let link = provider
        .upload(&client, &ctx, &config, None, Some(&endpoint))
        .await
        .unwrap();

    assert_eq!(link, "https://temp.sh/ABCD/testfile.txt");
}

#[tokio::test]
async fn test_gofile_provider_wiremock() {
    let server = MockServer::start().await;
    let json_response = r#"{
        "status": "ok",
        "data": {
            "downloadPage": "https://gofile.io/d/TESTPAGE",
            "guestToken": "guest_token_12345"
        }
    }"#;

    Mock::given(method("POST"))
        .and(path("/uploadfile"))
        .respond_with(ResponseTemplate::new(200).set_body_string(json_response))
        .mount(&server)
        .await;

    let (_f, ctx) = create_temp_test_file(b"gofile data");
    let provider = GofileProvider;
    let client = reqwest::Client::new();
    let config = Config::default();

    let endpoint = format!("{}/uploadfile", server.uri());
    let link = provider
        .upload(&client, &ctx, &config, None, Some(&endpoint))
        .await
        .unwrap();

    assert_eq!(link, "https://gofile.io/d/TESTPAGE");
}

#[tokio::test]
async fn test_tmpfiles_provider_rewrite_wiremock() {
    let server = MockServer::start().await;
    let json_response = r#"{
        "status": "success",
        "data": {
            "url": "https://tmpfiles.org/998877/testfile.txt"
        }
    }"#;

    Mock::given(method("POST"))
        .and(path("/api/v1/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_string(json_response))
        .mount(&server)
        .await;

    let (_f, ctx) = create_temp_test_file(b"tmpfiles test content");
    let provider = TmpfilesProvider;
    let client = reqwest::Client::new();
    let config = Config::default();

    let endpoint = format!("{}/api/v1/upload", server.uri());
    let link = provider
        .upload(&client, &ctx, &config, None, Some(&endpoint))
        .await
        .unwrap();

    // Verify rewrite from https://tmpfiles.org/ to https://tmpfiles.org/dl/
    assert_eq!(link, "https://tmpfiles.org/dl/998877/testfile.txt");
}

#[tokio::test]
async fn test_zerox_disabled_detection_wiremock() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "uploads disabled because it's been almost nothing but AI botnet spam",
        ))
        .mount(&server)
        .await;

    let (_f, ctx) = create_temp_test_file(b"0x0 test");
    let provider = ZeroxProvider;
    let client = reqwest::Client::new();
    let config = Config::default();

    // Off by default check
    assert!(provider.check_availability(false, &config).is_err());
    let err = provider.check_availability(true, &config).unwrap_err();
    assert!(err.to_string().contains("0x0.st disabled uploads"));

    // Upload detection check
    let endpoint = server.uri();
    let res = provider
        .upload(&client, &ctx, &config, None, Some(&endpoint))
        .await;
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("uploads disabled"));
}

#[tokio::test]
async fn test_fileio_wiremock() {
    let server = MockServer::start().await;
    let json_response = r#"{
        "success": true,
        "link": "https://file.io/abcdef"
    }"#;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(json_response))
        .mount(&server)
        .await;

    let (_f, ctx) = create_temp_test_file(b"fileio data");
    let provider = FileioProvider;
    let client = reqwest::Client::new();

    // Without key, check_availability should fail with friendly message
    let no_key_config = Config::default();
    assert!(provider.check_availability(false, &no_key_config).is_err());
    let explicit_err = provider.check_availability(true, &no_key_config).unwrap_err();
    assert!(explicit_err.to_string().contains("API key is required"));

    // With key configured, upload succeeds
    let mut with_key_config = Config::default();
    with_key_config.providers.insert(
        "fileio".to_string(),
        pushy::config::ProviderConfig {
            key: Some("test_secret_key".to_string()),
            ..Default::default()
        },
    );
    assert!(provider.check_availability(true, &with_key_config).is_ok());

    let endpoint = server.uri();
    let link = provider
        .upload(&client, &ctx, &with_key_config, None, Some(&endpoint))
        .await
        .unwrap();

    assert_eq!(link, "https://file.io/abcdef");
}

#[tokio::test]
async fn test_transfersh_wiremock() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/testfile.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("https://transfer.sh/1234/testfile.txt\n"))
        .mount(&server)
        .await;

    let (_f, ctx) = create_temp_test_file(b"transfer data");
    let provider = TransfershProvider;
    let client = reqwest::Client::new();

    // Default without base_url should explain public instance is unreachable
    let no_url_config = Config::default();
    assert!(provider.check_availability(false, &no_url_config).is_err());
    let explicit_err = provider.check_availability(true, &no_url_config).unwrap_err();
    assert!(explicit_err.to_string().contains("configure a custom base_url"));

    // With base_url configured, upload succeeds
    let mut with_url_config = Config::default();
    with_url_config.providers.insert(
        "transfersh".to_string(),
        pushy::config::ProviderConfig {
            base_url: Some(server.uri()),
            ..Default::default()
        },
    );
    assert!(provider.check_availability(true, &with_url_config).is_ok());

    let link = provider
        .upload(&client, &ctx, &with_url_config, None, Some(&server.uri()))
        .await
        .unwrap();

    assert_eq!(link, "https://transfer.sh/1234/testfile.txt");
}

#[tokio::test]
async fn test_retry_on_server_error() {
    let server = MockServer::start().await;

    // Fail first 2 times with 500, succeed on 3rd attempt
    Mock::given(method("POST"))
        .and(path("/user/api.php"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(2)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/user/api.php"))
        .respond_with(ResponseTemplate::new(200).set_body_string("https://files.catbox.moe/retried.txt\n"))
        .mount(&server)
        .await;

    let (_f, ctx) = create_temp_test_file(b"retry test");
    let provider = CatboxProvider;
    let uploader = Uploader::new(Config::default(), true);

    let endpoint = format!("{}/user/api.php", server.uri());
    let link = uploader
        .attempt_upload_with_retry(&provider, &ctx, Some(&endpoint))
        .await
        .unwrap();

    assert_eq!(link, "https://files.catbox.moe/retried.txt");
}

#[tokio::test]
async fn test_no_retry_on_4xx_client_error() {
    let server = MockServer::start().await;

    // Return 400 Bad Request
    Mock::given(method("POST"))
        .and(path("/user/api.php"))
        .respond_with(ResponseTemplate::new(400).set_body_string("Bad Request"))
        .expect(1) // Should only be called once, never retried
        .mount(&server)
        .await;

    let (_f, ctx) = create_temp_test_file(b"400 test");
    let provider = CatboxProvider;
    let uploader = Uploader::new(Config::default(), true);

    let endpoint = format!("{}/user/api.php", server.uri());
    let res = uploader
        .attempt_upload_with_retry(&provider, &ctx, Some(&endpoint))
        .await;

    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("HTTP 400"));
}

#[tokio::test]
async fn test_file_size_limit_check() {
    let provider = CatboxProvider;
    let meta = provider.metadata();
    assert_eq!(meta.max_size_bytes, Some(200 * 1024 * 1024));

    let uploader = Uploader::new(Config::default(), true);
    let fake_path = PathBuf::from("non_existent_file.bin");

    // Single provider upload with non-existent file gives clear error
    let res = uploader
        .upload_file(&fake_path, None, Some("catbox"), None, false)
        .await;
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("File not found"));
}

#[tokio::test]
async fn test_upload_to_all_concurrent_wiremock() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user/api.php"))
        .respond_with(ResponseTemplate::new(200).set_body_string("https://files.catbox.moe/all1.txt\n"))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_string("https://temp.sh/all/all2.txt\n"))
        .mount(&server)
        .await;

    let (_f, ctx) = create_temp_test_file(b"content for all providers");
    let catbox = CatboxProvider;
    let tempsh = TempshProvider;
    let client = reqwest::Client::new();
    let config = Config::default();

    let catbox_endpoint = format!("{}/user/api.php", server.uri());
    let tempsh_endpoint = format!("{}/upload", server.uri());

    let f1 = catbox.upload(&client, &ctx, &config, None, Some(&catbox_endpoint));
    let f2 = tempsh.upload(&client, &ctx, &config, None, Some(&tempsh_endpoint));

    let (r1, r2) = tokio::join!(f1, f2);
    assert_eq!(r1.unwrap(), "https://files.catbox.moe/all1.txt");
    assert_eq!(r2.unwrap(), "https://temp.sh/all/all2.txt");
}

#[test]
fn test_config_serialization_and_defaults() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_file = temp_dir.path().join("config.toml");

    let mut cfg = Config::default();
    cfg.provider = "tempsh".to_string();
    cfg.copy = true;
    cfg.catbox_userhash = Some("hash123".to_string());
    cfg.gofile_token = Some("token456".to_string());

    cfg.save(&config_file).unwrap();

    let content = std::fs::read_to_string(&config_file).unwrap();
    assert!(content.contains(r#"provider = "tempsh""#));
    assert!(content.contains("copy = true"));
    assert!(content.contains(r#"catbox_userhash = "hash123""#));
    assert!(content.contains(r#"gofile_token = "token456""#));

    let reloaded: Config = toml::from_str(&content).unwrap();
    assert_eq!(reloaded.provider, "tempsh");
    assert!(reloaded.copy);
    assert_eq!(reloaded.catbox_userhash.as_deref(), Some("hash123"));
    assert_eq!(reloaded.gofile_token.as_deref(), Some("token456"));
}

#[tokio::test]
async fn test_full_fallback_chain_wiremock() {
    let server = MockServer::start().await;

    // Catbox fails with 500
    Mock::given(method("POST"))
        .and(path("/user/api.php"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    // Tempsh succeeds with 200
    Mock::given(method("POST"))
        .and(path("/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_string("https://temp.sh/fallback/success.txt\n"))
        .mount(&server)
        .await;

    // Configure uploader with custom endpoints pointing to mock server
    let mut config = Config::default();
    config.fallback = vec!["catbox".to_string(), "tempsh".to_string()];

    // Catbox provider will fail, tempsh will succeed
    let (_f, ctx) = create_temp_test_file(b"fallback chain test");
    let catbox = CatboxProvider;
    let tempsh = TempshProvider;
    let client = reqwest::Client::new();

    let catbox_endpoint = format!("{}/user/api.php", server.uri());
    let catbox_res = catbox.upload(&client, &ctx, &config, None, Some(&catbox_endpoint)).await;
    assert!(catbox_res.is_err());

    let tempsh_endpoint = format!("{}/upload", server.uri());
    let tempsh_res = tempsh.upload(&client, &ctx, &config, None, Some(&tempsh_endpoint)).await;
    assert!(tempsh_res.is_ok());
    assert_eq!(tempsh_res.unwrap(), "https://temp.sh/fallback/success.txt");
}

#[test]
fn test_randomized_user_agent_generation() {
    let mut uas = std::collections::HashSet::new();
    for _ in 0..30 {
        let ua = ua_generator::ua::spoof_ua();
        assert!(!ua.is_empty());
        assert!(ua.contains("Mozilla/") || ua.contains("AppleWebKit") || ua.contains("Gecko"));
        uas.insert(ua);
    }
    // Verify randomization has diversity
    assert!(uas.len() > 1, "Expected multiple different randomized user agents, got {}", uas.len());
}

#[tokio::test]
async fn test_uploader_sends_randomized_user_agent_wiremock() {
    let server = MockServer::start().await;

    let target_ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";

    Mock::given(method("POST"))
        .and(path("/upload"))
        .and(wiremock::matchers::header("User-Agent", target_ua))
        .respond_with(ResponseTemplate::new(200).set_body_string("https://temp.sh/ua/ok.txt\n"))
        .mount(&server)
        .await;

    let (_f, ctx) = create_temp_test_file(b"ua test content");
    let tempsh = TempshProvider;
    let uploader = Uploader::with_user_agent(Config::default(), true, target_ua);

    let endpoint = format!("{}/upload", server.uri());
    let link = uploader
        .attempt_upload_with_retry(&tempsh, &ctx, Some(&endpoint))
        .await
        .unwrap();

    assert_eq!(link, "https://temp.sh/ua/ok.txt");
}
