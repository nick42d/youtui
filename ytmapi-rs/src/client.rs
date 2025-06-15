//! This module contains the basic HTTP client used in this library.
use crate::{Error, Result};
use serde::Serialize;
use std::borrow::Cow;

/// Basic HTTP client using TLS wrapping a `reqwest::Client`,
/// with the minimum required features to call YouTube Music queries.
/// Clone is low cost, internals of `reqwest::Client` are wrapped in an Arc.
#[derive(Debug, Clone)]
pub struct Client {
    inner: reqwest::Client,
}
/// Body that can be sent as a POST query using our client.
pub enum Body {
    FromString(String),
    FromFile(tokio::fs::File),
}
impl From<Body> for reqwest::Body {
    fn from(value: Body) -> Self {
        match value {
            Body::FromString(s) => reqwest::Body::from(s),
            Body::FromFile(f) => reqwest::Body::from(f),
        }
    }
}
/// Represents a basic reponse from our basic HTTP client.
pub struct QueryResponse {
    pub text: String,
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
}
impl QueryResponse {
    async fn try_from_reqwest_response(response: reqwest::Response) -> Result<Self> {
        let status_code = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(header, value)| -> Result<_> {
                let header = header.to_string();
                let value = value
                    .to_str()
                    .map_err(|_| Error::web(format!("Error parsing response header: {:?}", value)))?
                    .to_owned();
                Ok((header, value))
            })
            .collect::<Result<Vec<_>>>()?;
        let text = response.text().await?;
        Ok(QueryResponse {
            text,
            status_code,
            headers,
        })
    }
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
    /// Run a POST query, with url, body, key/kalue params and headers.
    pub async fn post_query<'a, I>(
        &self,
        url: impl AsRef<str>,
        headers: impl IntoIterator<IntoIter = I>,
        body: Body,
        params: &(impl Serialize + ?Sized),
    ) -> Result<QueryResponse>
    where
        I: Iterator<Item = (&'a str, Cow<'a, str>)>,
    {
        let mut request_builder = self.inner.post(url.as_ref()).body(body).query(params);
        for (header, value) in headers {
            request_builder = request_builder.header(header, value.as_ref());
        }
        let response = request_builder.send().await?;
        QueryResponse::try_from_reqwest_response(response).await
    }
    /// Run a POST query, with url, body serialisable to json, key/kalue params
    /// and headers.
    pub async fn post_json_query<'a, I>(
        &self,
        url: impl AsRef<str>,
        headers: impl IntoIterator<IntoIter = I>,
        body_json: &(impl Serialize + ?Sized),
        params: &(impl Serialize + ?Sized),
    ) -> Result<QueryResponse>
    where
        I: Iterator<Item = (&'a str, Cow<'a, str>)>,
    {
        let mut request_builder = self.inner.post(url.as_ref()).json(body_json).query(params);
        for (header, value) in headers {
            request_builder = request_builder.header(header, value.as_ref());
        }
        let response = request_builder.send().await?;
        QueryResponse::try_from_reqwest_response(response).await
    }
    /// Run a GET query, with url, key/value params and headers.
    pub async fn get_query<'a, I>(
        &self,
        url: impl AsRef<str>,
        headers: impl IntoIterator<IntoIter = I>,
        params: &(impl Serialize + ?Sized),
    ) -> Result<QueryResponse>
    where
        I: Iterator<Item = (&'a str, Cow<'a, str>)>,
    {
        let mut request_builder = self.inner.get(url.as_ref()).query(params);
        for (header, value) in headers {
            request_builder = request_builder.header(header, value.as_ref());
        }
        let response = request_builder.send().await?;
        QueryResponse::try_from_reqwest_response(response).await
    }
}
