//! This module contains the basic HTTP client that can
//! be used when implementing the Query trait.

use crate::Result;
use reqwest::header::HeaderMap;
use serde::Serialize;
use std::borrow::Cow;

/// Basic HTTP client using TLS, with the minimum required features to call
/// YouTube Music queries. Clone is low cost, internals are wrapped in an Arc.
#[derive(Debug, Clone)]
pub struct Client {
    inner: reqwest::Client,
}

impl Client {
    pub fn new() -> Result<Self> {
        let inner = reqwest::Client::builder().build()?;
        Ok(Self { inner })
    }
    #[cfg(feature = "rustls-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
    pub fn new_rustls_tls() -> Result<Self> {
        let inner = reqwest::Client::builder().use_rustls_tls().build()?;
        Ok(Self { inner })
    }
    #[cfg(feature = "native-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    pub fn new_native_tls() -> Result<Self> {
        let inner = reqwest::Client::builder().use_native_tls().build()?;
        Ok(Self { inner })
    }
    pub async fn post_query<'a, I>(
        &self,
        url: impl AsRef<str>,
        headers: impl IntoIterator<IntoIter = I>,
        body_json: &(impl Serialize + ?Sized),
    ) -> Result<String>
    where
        I: Iterator<Item = (&'a str, Cow<'a, str>)>,
    {
        let mut request_builder = self.inner.post(url.as_ref()).json(body_json);
        for (header, value) in headers {
            request_builder = request_builder.header(header, value.as_ref());
        }
        request_builder
            .send()
            .await?
            .text()
            .await
            .map_err(Into::into)
    }
    pub async fn get_query<'a, I>(
        &self,
        url: impl AsRef<str>,
        headers: impl IntoIterator<IntoIter = I>,
        params: &(impl Serialize + ?Sized),
    ) -> Result<String>
    where
        I: Iterator<Item = (&'a str, Cow<'a, str>)>,
    {
        let mut request_builder = self.inner.get(url.as_ref()).query(params);
        for (header, value) in headers {
            request_builder = request_builder.header(header, value.as_ref());
        }
        request_builder
            .send()
            .await?
            .text()
            .await
            .map_err(Into::into)
    }
}
