//! Builder implementation for YtMusic, to allow more complicated construction.
//! ## Example
//! Basic usage with a pre-created cookie file forcing use of rustls-tls
//! ```no_run
//! #[tokio::main]
//! pub async fn main() -> Result<(), ytmapi_rs::Error> {
//!     let cookie_path = std::path::Path::new("./cookie.txt");
//!     let yt = ytmapi_rs::builder::YtMusicBuilder::new_rustls_tls()
//!         .with_browser_token_cookie_file(cookie_path)
//!         .build()
//!         .await?;
//!     yt.get_search_suggestions("Beatles").await?;
//!     let result = yt.get_search_suggestions("Beatles").await?;
//!     println!("{:?}", result);
//!     Ok(())
//! }
//! ```
use crate::{
    auth::{BrowserToken, OAuthToken},
    client::Client,
    Result, YtMusic,
};
use std::path::Path;

#[derive(Default)]
pub enum ClientOptions {
    #[default]
    Default,
    #[cfg(feature = "rustls-tls")]
    RustlsTls,
    #[cfg(feature = "native-tls")]
    NativeTls,
    Existing(Client),
}

/// Helper struct for YtMusicBuilder.
pub struct NoToken;
/// Helper struct for YtMusicBuilder.
pub struct FromCookie(String);
/// Helper struct for YtMusicBuilder.
pub struct FromCookieFile<T>(T);

/// Builder to build more complex YtMusic.
pub struct YtMusicBuilder<T> {
    client_options: ClientOptions,
    token: T,
}

impl<T> YtMusicBuilder<T> {
    pub fn with_client(mut self, client: Client) -> Self {
        self.client_options = ClientOptions::Existing(client);
        self
    }
    #[cfg(feature = "rustls-tls")]
    pub fn with_rustls_tls(mut self) -> Self {
        self.client_options = ClientOptions::RustlsTls;
        self
    }
    #[cfg(feature = "rustls-tls")]
    pub fn with_default_tls(mut self) -> Self {
        self.client_options = ClientOptions::Default;
        self
    }
    #[cfg(feature = "native-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    pub fn with_native_tls(mut self) -> Self {
        self.client_options = ClientOptions::NativeTls;
        self
    }
    pub fn with_browser_token(self, token: BrowserToken) -> YtMusicBuilder<BrowserToken> {
        let YtMusicBuilder {
            client_options,
            token: _,
        } = self;
        YtMusicBuilder {
            client_options,
            token,
        }
    }
    // TODO: Improve how this handles building client.
    pub fn with_browser_token_cookie(self, cookie: String) -> YtMusicBuilder<FromCookie> {
        let YtMusicBuilder {
            client_options,
            token: _,
        } = self;
        let token = FromCookie(cookie);
        YtMusicBuilder {
            client_options,
            token,
        }
    }
    // TODO: Improve how this handles building client.
    pub fn with_browser_token_cookie_file<P: AsRef<Path>>(
        self,
        cookie_file: P,
    ) -> YtMusicBuilder<FromCookieFile<P>> {
        let YtMusicBuilder {
            client_options,
            token: _,
        } = self;
        let token = FromCookieFile(cookie_file);
        YtMusicBuilder {
            client_options,
            token,
        }
    }
    pub fn with_oauth_token(self, token: OAuthToken) -> YtMusicBuilder<OAuthToken> {
        let YtMusicBuilder {
            client_options,
            token: _,
        } = self;
        YtMusicBuilder {
            client_options,
            token,
        }
    }
}
impl YtMusicBuilder<FromCookie> {
    pub async fn build(self) -> Result<YtMusic<BrowserToken>> {
        let YtMusicBuilder {
            client_options,
            token: FromCookie(cookie),
        } = self;
        let client = build_client(client_options)?;
        let token = BrowserToken::from_str(cookie.as_ref(), &client).await?;
        Ok(YtMusic { client, token })
    }
}
impl<P: AsRef<Path>> YtMusicBuilder<FromCookieFile<P>> {
    pub async fn build(self) -> Result<YtMusic<BrowserToken>> {
        let YtMusicBuilder {
            client_options,
            token: FromCookieFile(cookie_file),
        } = self;
        let client = build_client(client_options)?;
        let token = BrowserToken::from_cookie_file(cookie_file, &client).await?;
        Ok(YtMusic { client, token })
    }
}
impl YtMusicBuilder<NoToken> {
    // This lint is a little confusing in this case, as we do not want different
    // default implementations for YtMusicBuilder<T> depending on T. There
    // should only be one way to construct a YtMusicBuilder with T = NoToken.
    #[allow(clippy::new_without_default)]
    pub fn new() -> YtMusicBuilder<NoToken> {
        YtMusicBuilder {
            client_options: ClientOptions::Default,
            token: NoToken,
        }
    }
    pub fn new_with_client(client: Client) -> YtMusicBuilder<NoToken> {
        YtMusicBuilder {
            client_options: ClientOptions::Existing(client),
            token: NoToken,
        }
    }
    #[cfg(feature = "rustls-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
    pub fn new_rustls_tls() -> YtMusicBuilder<NoToken> {
        YtMusicBuilder {
            client_options: ClientOptions::RustlsTls,
            token: NoToken,
        }
    }
    #[cfg(feature = "native-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    pub fn new_native_tls() -> Self {
        YtMusicBuilder {
            client_options: ClientOptions::NativeTls,
            token: NoToken,
        }
    }
}

impl YtMusicBuilder<BrowserToken> {
    pub fn build(self) -> Result<YtMusic<BrowserToken>> {
        let YtMusicBuilder {
            client_options,
            token,
        } = self;
        let client = build_client(client_options)?;
        Ok(YtMusic { client, token })
    }
}

impl YtMusicBuilder<OAuthToken> {
    pub fn build(self) -> Result<YtMusic<OAuthToken>> {
        let YtMusicBuilder {
            client_options,
            token,
        } = self;
        let client = build_client(client_options)?;
        Ok(YtMusic { client, token })
    }
}

fn build_client(client_options: ClientOptions) -> Result<Client> {
    match client_options {
        ClientOptions::Default => Client::new(),
        #[cfg(feature = "rustls-tls")]
        ClientOptions::RustlsTls => Client::new_rustls_tls(),
        #[cfg(feature = "native-tls")]
        ClientOptions::Native => Client::new_native_tls(),
        ClientOptions::Existing(client) => Ok(client),
    }
}
