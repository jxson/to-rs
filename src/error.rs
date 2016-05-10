use log;

pub type ToResult<T> = Result<T, ToError>;

#[derive(Debug)]
pub enum ToError {
    Log(log::SetLoggerError),
}

// TODO: add custom displays for these errors.
// * SEE: https://jadpole.github.io/rust/many-error-types
// * SEE: http://lucumr.pocoo.org/2014/11/6/error-handling-in-rust/
impl From<log::SetLoggerError> for ToError {
    fn from(err: log::SetLoggerError) -> ToError {
        ToError::Log(err)
    }
}
