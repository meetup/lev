use rusoto_lambda::{GetFunctionConfigurationError, UpdateFunctionConfigurationError};

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "{}", _0)]
    GetConfig(#[cause] GetFunctionConfigurationError),
    #[fail(display = "{}", _0)]
    UpdateConfig(#[cause] UpdateFunctionConfigurationError),
}

impl From<GetFunctionConfigurationError> for Error {
    fn from(err: GetFunctionConfigurationError) -> Self {
        Error::GetConfig(err)
    }
}

impl From<UpdateFunctionConfigurationError> for Error {
    fn from(err: UpdateFunctionConfigurationError) -> Self {
        Error::UpdateConfig(err)
    }
}
