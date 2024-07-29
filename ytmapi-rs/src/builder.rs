use crate::{
    auth::{BrowserToken, OAuthToken},
    client::Client,
    Result, YtMusic,
};

pub enum Tls {
    #[cfg(feature = "default-tls")]
    Default,
    #[cfg(feature = "rustls-tls")]
    Rustls,
    #[cfg(feature = "native-tls")]
    Native,
}

pub struct NoToken;

pub struct YtMusicBuilder<T> {
    tls: Tls,
    token: T,
}

impl YtMusicBuilder<NoToken> {
    #[cfg(feature = "default-tls")]
    pub fn new() -> Self {
        YtMusicBuilder {
            tls: Tls::Default,
            token: NoToken,
        }
    }
    #[cfg(feature = "rustls-tls")]
    pub fn new_rustls_tls() -> Self {
        YtMusicBuilder {
            tls: Tls::Rustls,
            token: NoToken,
        }
    }
    #[cfg(feature = "native-tls")]
    pub fn new_native_tls() -> Self {
        YtMusicBuilder {
            tls: Tls::Native,
            token: NoToken,
        }
    }
    #[cfg(feature = "rustls-tls")]
    pub fn with_rustls_tls(mut self) -> Self {
        self.tls = Tls::Rustls;
        self
    }
    #[cfg(feature = "native-tls")]
    pub fn with_native_tls(mut self) -> Self {
        self.tls = Tls::Native;
        self
    }
    pub fn with_browser_token(self, token: BrowserToken) -> YtMusicBuilder<BrowserToken> {
        let YtMusicBuilder { tls, token: _ } = self;
        YtMusicBuilder { tls, token }
    }
    pub fn with_oauth_token(self, token: OAuthToken) -> YtMusicBuilder<OAuthToken> {
        let YtMusicBuilder { tls, token: _ } = self;
        YtMusicBuilder { tls, token }
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

fn build_client(tls: Tls) -> Result<Client> {
    match tls {
        #[cfg(feature = "default-tls")]
        Tls::Default => Client::new(),
        #[cfg(feature = "rustls-tls")]
        Tls::Rustls => Client::new_rustls_tls(),
        #[cfg(feature = "native-tls")]
        Tls::Native => Client::new_native_tls(),
    }
}
