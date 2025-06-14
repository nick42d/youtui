//! This module contains the basic HTTP client used in this library.
use crate::utils::constants::{USER_AGENT, YTM_URL};
use crate::Result;
use chrono::format;
use futures::channel::oneshot::Receiver;
use serde::Serialize;
use serde_json::json;
use std::borrow::Cow;

/// Basic HTTP client using TLS wrapping a `reqwest::Client`,
/// with the minimum required features to call YouTube Music queries.
/// Clone is low cost, internals of `reqwest::Client` are wrapped in an Arc.
#[derive(Debug, Clone)]
pub struct Client {
    inner: reqwest::Client,
}
pub enum Body<'a> {
    FromString(String),
    FromFileRef(&'a tokio::fs::File),
}
pub struct QueryResponse {
    pub text: String,
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
}
impl Body<'_> {
    async fn try_into_reqwest_body(self) -> std::io::Result<reqwest::Body> {
        match self {
            Body::FromString(s) => Ok(reqwest::Body::from(s)),
            Body::FromFileRef(f) => Ok(reqwest::Body::from(f.try_clone().await?)),
        }
    }
}
impl QueryResponse {
    async fn try_from_reqwest_response(response: reqwest::Response) -> Result<Self> {
        let status_code = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(
                |(header, value)| -> std::result::Result<_, reqwest::header::ToStrError> {
                    let header = header.to_string();
                    let value = value.to_str()?.to_owned();
                    Ok((header, value))
                },
            )
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
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
    /// Run a POST query, with url, body and headers.
    /// Result is returned as a String.
    pub async fn post_query<'a, 'b, I>(
        &self,
        url: impl AsRef<str>,
        headers: impl IntoIterator<IntoIter = I>,
        body: Body<'b>,
        params: &(impl Serialize + ?Sized),
    ) -> Result<QueryResponse>
    where
        I: Iterator<Item = (&'a str, Cow<'a, str>)>,
    {
        let mut request_builder = self
            .inner
            .post(url.as_ref())
            .body(body.try_into_reqwest_body().await.unwrap())
            .query(params);
        for (header, value) in headers {
            request_builder = request_builder.header(header, value.as_ref());
        }
        let response = request_builder.send().await?;
        QueryResponse::try_from_reqwest_response(response).await
    }
    /// Run a POST query, with url, body serialisable to json and headers.
    /// Result is returned as a String.
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
    /// Result is returned as a String.
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
