//! This module contains the basic HTTP client used in this library.
use crate::Result;
use serde::Serialize;
use std::borrow::Cow;

/// Basic HTTP client using TLS wrapping a `reqwest::Client`,
/// with the minimum required features to call YouTube Music queries.
/// Clone is low cost, internals of `reqwest::Client` are wrapped in an Arc.
#[derive(Debug, Clone)]
pub struct Client {
    inner: reqwest::Client,
}

impl Client {
    /// Utilises reqwest's default tls choice for the enabled set of options.
    pub fn new() -> Result<Self> {
        let inner = reqwest::Client::builder().build()?;
        Ok(Self { inner })
    }
    #[cfg(feature = "rustls-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
    /// Force the use of rustls-tls
    pub fn new_rustls_tls() -> Result<Self> {
        let inner = reqwest::Client::builder().use_rustls_tls().build()?;
        Ok(Self { inner })
    }
    #[cfg(feature = "native-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    /// Force the use of native-tls
    pub fn new_native_tls() -> Result<Self> {
        let inner = reqwest::Client::builder().use_native_tls().build()?;
        Ok(Self { inner })
    }
    #[cfg(feature = "reqwest")]
    #[cfg_attr(docsrs, doc(cfg(feature = "reqwest")))]
    /// Re-use a pre-existing reqwest::Client.
    pub fn new_from_reqwest_client(client: reqwest::Client) -> Self {
        Self { inner: client }
    }
    /// Run a POST query, with url, body representing a file handle and headers.
    /// Result is returned as a String.
    pub async fn post_file_query<'a, I>(
        &self,
        url: impl AsRef<str>,
        headers: impl IntoIterator<IntoIter = I>,
        body_file: tokio::fs::File,
        params: &(impl Serialize + ?Sized),
    ) -> Result<String>
    where
        I: Iterator<Item = (&'a str, Cow<'a, str>)>,
    {
        let mut request_builder = self.inner.post(url.as_ref()).body(body_file).query(params);
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
    /// Run a POST query, with url, body serialisable to json and headers.
    /// Result is returned as a String.
    pub async fn post_json_query<'a, I>(
        &self,
        url: impl AsRef<str>,
        headers: impl IntoIterator<IntoIter = I>,
        body_json: &(impl Serialize + ?Sized),
        params: &(impl Serialize + ?Sized),
    ) -> Result<String>
    where
        I: Iterator<Item = (&'a str, Cow<'a, str>)>,
    {
        let mut request_builder = self.inner.post(url.as_ref()).json(body_json).query(params);
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
    /// Run a GET query, with url, key/value params and headers.
    /// Result is returned as a String.
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
