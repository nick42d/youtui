use std::path::Path;

use crate::{
    auth::{BrowserToken, OAuthToken},
    client::Client,
    Result, YtMusic,
};

#[derive(Default)]
pub enum ClientOptions {
    #[default]
    Default,
    #[cfg(feature = "rustls-tls")]
    Rustls,
    #[cfg(feature = "native-tls")]
    Native,
    Existing(Client),
}

pub struct NoToken;
pub struct FromCookie(String);
pub struct FromCookieFile<T>(T);

pub struct YtMusicBuilder<T> {
    tls: ClientOptions,
    token: T,
}

impl<T> YtMusicBuilder<T> {
    #[cfg(feature = "native-tls")]
    pub fn new_native_tls() -> Self {
        YtMusicBuilder {
            tls: ClientOptions::Native,
            token: NoToken,
        }
    }
    pub fn with_client(mut self, client: Client) -> Self {
        self.tls = ClientOptions::Existing(client);
        self
    }
    #[cfg(feature = "rustls-tls")]
    pub fn with_rustls_tls(mut self) -> Self {
        self.tls = ClientOptions::Rustls;
        self
    }
    #[cfg(feature = "native-tls")]
    pub fn with_native_tls(mut self) -> Self {
        self.tls = ClientOptions::Native;
        self
    }
    pub fn with_browser_token(self, token: BrowserToken) -> YtMusicBuilder<BrowserToken> {
        let YtMusicBuilder { tls, token: _ } = self;
        YtMusicBuilder { tls, token }
    }
    // TODO: Improve how this handles building client.
    pub fn with_browser_token_cookie(self, cookie: String) -> YtMusicBuilder<FromCookie> {
        let YtMusicBuilder { tls, token: _ } = self;
        let token = FromCookie(cookie);
        YtMusicBuilder { tls, token }
    }
    // TODO: Improve how this handles building client.
    pub fn with_browser_token_cookie_file<P: AsRef<Path>>(
        self,
        cookie_file: P,
    ) -> YtMusicBuilder<FromCookieFile<P>> {
        let YtMusicBuilder { tls, token: _ } = self;
        let token = FromCookieFile(cookie_file);
        YtMusicBuilder { tls, token }
    }
    pub fn with_oauth_token(self, token: OAuthToken) -> YtMusicBuilder<OAuthToken> {
        let YtMusicBuilder { tls, token: _ } = self;
        YtMusicBuilder { tls, token }
    }
}
impl YtMusicBuilder<FromCookie> {
    pub async fn build(self) -> Result<YtMusic<BrowserToken>> {
        let YtMusicBuilder {
            tls,
            token: FromCookie(cookie),
        } = self;
        let client = build_client(tls)?;
        let token = BrowserToken::from_str(cookie.as_ref(), &client).await?;
        Ok(YtMusic { client, token })
    }
}
impl<P: AsRef<Path>> YtMusicBuilder<FromCookieFile<P>> {
    pub async fn build(self) -> Result<YtMusic<BrowserToken>> {
        let YtMusicBuilder {
            tls,
            token: FromCookieFile(cookie_file),
        } = self;
        let client = build_client(tls)?;
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
            tls: ClientOptions::Default,
            token: NoToken,
        }
    }
    pub fn new_with_client(client: Client) -> YtMusicBuilder<NoToken> {
        YtMusicBuilder {
            tls: ClientOptions::Existing(client),
            token: NoToken,
        }
    }
    #[cfg(feature = "rustls-tls")]
    pub fn new_rustls_tls() -> YtMusicBuilder<NoToken> {
        YtMusicBuilder {
            tls: ClientOptions::Rustls,
            token: NoToken,
        }
    }
}

impl YtMusicBuilder<BrowserToken> {
    pub fn build(self) -> Result<YtMusic<BrowserToken>> {
        let YtMusicBuilder { tls, token } = self;
        let client = build_client(tls)?;
        Ok(YtMusic { client, token })
    }
}

impl YtMusicBuilder<OAuthToken> {
    pub fn build(self) -> Result<YtMusic<OAuthToken>> {
        let YtMusicBuilder { tls, token } = self;
        let client = build_client(tls)?;
        Ok(YtMusic { client, token })
    }
}

fn build_client(tls: ClientOptions) -> Result<Client> {
    match tls {
        ClientOptions::Default => Client::new(),
        #[cfg(feature = "rustls-tls")]
        ClientOptions::Rustls => Client::new_rustls_tls(),
        #[cfg(feature = "native-tls")]
        ClientOptions::Native => Client::new_native_tls(),
        ClientOptions::Existing(client) => Ok(client),
    }
}
