use failure::Fail;
use rusoto_lambda::{GetFunctionConfigurationError, UpdateFunctionConfigurationError};
use rusoto_core::RusotoError;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "{}", _0)]
    GetConfig(#[cause] RusotoError<GetFunctionConfigurationError>),
    #[fail(display = "{}", _0)]
    UpdateConfig(#[cause] RusotoError<UpdateFunctionConfigurationError>),
}

impl From<RusotoError<GetFunctionConfigurationError>> for Error {
    fn from(err: RusotoError<GetFunctionConfigurationError>) -> Self {
        Error::GetConfig(err)
    }
}

impl From<RusotoError<UpdateFunctionConfigurationError>> for Error {
    fn from(err: RusotoError<UpdateFunctionConfigurationError>) -> Self {
        Error::UpdateConfig(err)
    }
}
