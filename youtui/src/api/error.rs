use crate::config::AuthType;

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
// 7. Providing this wrapper type allows it to be converted to an anyhow::Error.
#[derive(Clone, Debug)]
pub struct DynamicApiError(String);

pub fn wrong_auth_token_error_message<Q>(
    current_authtype: AuthType,
    expected_authtypes: &[AuthType],
) -> String {
    format!(
        "Query <{}> not supported on auth type {:?}. Expected auth type: {:?}",
        std::any::type_name::<Q>(),
        current_authtype,
        expected_authtypes
    )
}

impl std::error::Error for DynamicApiError {}
impl std::fmt::Display for DynamicApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error recieved when creating API: <{}>", self.0)
    }
}
impl From<ytmapi_rs::Error> for DynamicApiError {
    fn from(value: ytmapi_rs::Error) -> Self {
        DynamicApiError(value.to_string())
    }
}
