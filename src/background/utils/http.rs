use std::sync::LazyLock;

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent(concat!("KestrelUI/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("failed to build http client")
});

pub fn client() -> &'static reqwest::Client {
    &CLIENT
}

pub fn get(url: &str) -> reqwest::RequestBuilder {
    CLIENT.get(url)
}
