use crate::config::AuthType;
use ytmapi_rs::error::ErrorKind;

// OK, this is a rabbit hole as to why this api needs a custom error type.
// 1. We want to be able store the Result of API creation in a shared cell
//    (needs to be Clone)
// 2. We can't return ytmapi_rs::Error as it is not Clone as it can contain
//    std::io::Error.
// 3. anyhow::Error is also not Clone
// 4. We can't just wrap the error in Arc<anyhow::Error> - can't be converted
//    back to anyhow::Error.
// 5. Therefore, we use this error type which is Clone - converting non-Clone
//    variants to Strign for type erasure.
// 6. The only variant we need to know more than the String representation is
//    the OAuthTokenExpired error, since it's used for retries.
#[derive(Clone, Debug)]
pub enum DynamicApiError {
    OAuthTokenExpired {
        token_hash: u64,
    },
    WrongAuthToken {
        current_authtype: AuthType,
        query_name_string: &'static str,
    },
    StreamSourceNotSupported,
    Other(String),
}

impl DynamicApiError {
    pub fn new_wrong_auth_token<Q>(current_authtype: AuthType) -> Self {
        DynamicApiError::WrongAuthToken {
            current_authtype,
            query_name_string: std::any::type_name::<Q>(),
        }
    }
}
impl std::error::Error for DynamicApiError {}
impl std::fmt::Display for DynamicApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynamicApiError::OAuthTokenExpired { token_hash: _ } => {
                write!(f, "OAuth token has expired")
            }
            DynamicApiError::Other(msg) => write!(f, "{msg}"),
            DynamicApiError::WrongAuthToken {
                current_authtype,
                query_name_string,
            } => {
                let expected_authtype = match current_authtype {
                    AuthType::Browser => AuthType::OAuth,
                    AuthType::OAuth => AuthType::Browser,
                };
                write!(
                    f,
                    "Query <{}> not supported on auth type {:?}. Expected auth type: {:?}",
                    query_name_string, current_authtype, expected_authtype
                )
            }
            DynamicApiError::StreamSourceNotSupported => 
                write!(f, "It's not currently possible to get source files for each result of a stream, since the source files get consumed to obtain continuation params"),
        }
    }
}
impl From<ytmapi_rs::Error> for DynamicApiError {
    fn from(value: ytmapi_rs::Error) -> Self {
        match value.into_kind() {
            ErrorKind::OAuthTokenExpired { token_hash } => {
                DynamicApiError::OAuthTokenExpired { token_hash }
            }
            other => DynamicApiError::Other(other.to_string()),
        }
    }
}
