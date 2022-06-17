use core::fmt;

#[derive(Debug)]
pub enum RecurseError {
    NoNameServerError,
    NoARecordError,
}

impl std::error::Error for RecurseError {}

impl fmt::Display for RecurseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecurseError::NoNameServerError => write!(f, "No NS record provided"),
            RecurseError::NoARecordError => write!(f, "No A record provided"),
        }
    }
}
