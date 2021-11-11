use std::fmt::{Display, Formatter};
use std::num::ParseIntError;

/// Error type which just contains a `String`
#[derive(Debug)]
pub struct Error(String);

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Error {
    #[must_use]
    /// Creates a new instance of Error
    pub fn new(info: String) -> Error {
        Error(info)
    }
}

impl From<ParseIntError> for Error {
    fn from(pie: ParseIntError) -> Self {
        Self(pie.to_string())
    }
}

impl From<jargon_args::Error> for Error {
    fn from(jae: jargon_args::Error) -> Self {
        match jae {
            jargon_args::Error::MissingArg(e) => Error::new(format!("missing argument: '{}'", e)),
            jargon_args::Error::Other(e) => Error::new(e),
        }
    }
}
