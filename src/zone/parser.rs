// Remove all whitespace
// Remove all comments
// Parse as normal

use core::fmt;
use tokio::fs;

#[derive(Debug)]
pub enum ZoneParserError {
    EmptyFile,
    NoOrigin,
    NoTimeToLive,
}

impl std::error::Error for ZoneParserError {}

impl fmt::Display for ZoneParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZoneParserError::EmptyFile => write!(f, "Empty file"),
            ZoneParserError::NoOrigin => write!(f, "No origin provided"),
            ZoneParserError::NoTimeToLive => write!(f, "No time to live provided"),
        }
    }
}

type ParserResult<T> = Result<T, Box<dyn std::error::Error>>;

async fn parse(path: &str) -> ParserResult<()> {
    // Loading file into bytes
    let file_bytes = fs::read(path).await?;

    // Bytes to string
    let file = String::from_utf8(file_bytes)?;

    // Split on newline
    let mut lines = file.lines().map(|line| {
        // Removing comments from lines, empty lines become nothing
        line.split(';').next().unwrap_or("").split_whitespace()
    });

    // First line
    let mut line = lines.next().ok_or_else(|| ZoneParserError::EmptyFile)?;

    // Parse start of the zone
    match line.next() {
        Some("$ORIGIN") => true,
        _ => return Err(Box::new(ZoneParserError::NoOrigin)),
    };

    // Parsing the zone name
    let zone_name = line.next().ok_or_else(|| ZoneParserError::NoOrigin)?;

    // Parse time to live
    match line.next() {
        Some("$TTL") => true,
        _ => return Err(Box::new(ZoneParserError::NoTimeToLive)),
    };

    // Parsing the zone name
    let time_to_live: i64 = line
        .next()
        .ok_or_else(|| ZoneParserError::NoTimeToLive)?
        .parse()?;

    Ok(())
}
