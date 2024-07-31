use crate::{
    auth::{BrowserToken, OAuthToken},
    client::Client,
    Result, YtMusic,
};

pub enum ClientOptions {
    Default,
    #[cfg(feature = "rustls-tls")]
    Rustls,
    #[cfg(feature = "native-tls")]
    Native,
    Existing(Client),
}

pub struct NoToken;

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
    pub fn with_oauth_token(self, token: OAuthToken) -> YtMusicBuilder<OAuthToken> {
        let YtMusicBuilder { tls, token: _ } = self;
        YtMusicBuilder { tls, token }
    }
}

impl YtMusicBuilder<NoToken> {
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
