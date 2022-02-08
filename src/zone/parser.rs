// Remove all whitespace
// Remove all comments
// Parse as normal

use core::fmt;
use std::{
    net::{Ipv4Addr, Ipv6Addr},
    str::SplitWhitespace,
};
use tokio::fs;

struct ZoneFile {
    name: String,
    time_to_live: usize,
    records: Vec<RecordData>,
}

#[derive(Debug)]
pub enum ZoneParserError {
    EmptyFile,
    NoOrigin,
    NoTimeToLive,
    NoDomain,
    NoClass,
    InvalidClass,
    NoType,
    InvalidType,
    DataParsing,
}

enum RecordData {
    ARecord(Ipv4Addr),
    AAAARecord(Ipv6Addr),
    MXRecord(usize, String),
    CNameRecord(String),
    NSRecord(String),
}

impl std::error::Error for ZoneParserError {}

impl fmt::Display for ZoneParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZoneParserError::EmptyFile => write!(f, "Empty file"),
            ZoneParserError::NoOrigin => write!(f, "No origin provided"),
            ZoneParserError::NoTimeToLive => write!(f, "No time to live provided"),
            ZoneParserError::NoDomain => write!(f, "No domain provided"),
            ZoneParserError::NoClass => write!(f, "No class provided"),
            ZoneParserError::InvalidClass => write!(f, "Invalid class provided"),
            ZoneParserError::NoType => write!(f, "type provided"),
            ZoneParserError::InvalidType => write!(f, "Invalid class provided"),
            ZoneParserError::DataParsing => write!(f, "Error parsing data"),
        }
    }
}

type ParserResult<T> = Result<T, Box<dyn std::error::Error>>;

fn parse(bytes: Vec<u8>) -> ParserResult<ZoneFile> {
    // Bytes to string
    let file = String::from_utf8(bytes)?;
    let file = file.trim();

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
    let name = line
        .next()
        .ok_or_else(|| ZoneParserError::NoOrigin)?
        .to_string();

    let mut line = lines.next().ok_or_else(|| ZoneParserError::EmptyFile)?;

    // Parse time to live
    match line.next() {
        Some("$TTL") => true,
        _ => return Err(Box::new(ZoneParserError::NoTimeToLive)),
    };

    // Parsing the zone name
    let time_to_live: usize = line
        .next()
        .ok_or_else(|| ZoneParserError::NoTimeToLive)?
        .parse()?;

    // Use to keep track of the previous domain, used when finding '@' symbols that refer to the last domain
    let mut previous_domain: Option<&str> = None;

    let mut records = Vec::new();

    // Parse the rest of the file
    for mut line in lines {
        let mut domain = line.next().ok_or_else(|| ZoneParserError::NoDomain)?;

        if domain == "@" {
            // Set to the last domain if it exists
            domain = previous_domain.ok_or_else(|| ZoneParserError::NoDomain)?;
        }

        // Parse the class and the record type
        // Only want to parse "IN" values
        match line.next().ok_or_else(|| ZoneParserError::NoClass)? {
            "IN" => {}
            _ => return Err(Box::new(ZoneParserError::InvalidClass)),
        };

        let record_type = line.next().ok_or_else(|| ZoneParserError::NoType)?;

        // Parse the data and reject a bad type
        let data = match record_type {
            "A" => RecordData::ARecord(parse_a_record_data(&mut line)?),
            "AAAA" => RecordData::AAAARecord(parse_aaaa_record_data(&mut line)?),
            "CNAME" => RecordData::CNameRecord(parse_name_record_data(&mut line, &name)?),
            "NS" => RecordData::NSRecord(parse_name_record_data(&mut line, &name)?),
            "MX" => {
                let (priority, domain) = parse_mx_record_data(&mut line)?;
                RecordData::MXRecord(priority, domain)
            }
            _ => return Err(Box::new(ZoneParserError::InvalidType)),
        };

        // Add record
        records.push(data);

        // Set previous domain
        previous_domain = Some(domain);
    }

    Ok(ZoneFile {
        name,
        time_to_live,
        records,
    })
}

fn parse_a_record_data(line: &mut SplitWhitespace) -> ParserResult<Ipv4Addr> {
    // Get only one column
    let address_string = line.next().ok_or_else(|| ZoneParserError::DataParsing)?;

    // Parse into an ip address
    let address: Ipv4Addr = address_string.parse()?;

    Ok(address)
}

fn parse_aaaa_record_data(line: &mut SplitWhitespace) -> ParserResult<Ipv6Addr> {
    // Get only one column
    let address_string = line.next().ok_or_else(|| ZoneParserError::DataParsing)?;

    // Parse into an ip address
    let address: Ipv6Addr = address_string.parse()?;

    Ok(address)
}

fn parse_mx_record_data(line: &mut SplitWhitespace) -> ParserResult<(usize, String)> {
    // Get only one column
    let priority = line
        .next()
        .ok_or_else(|| ZoneParserError::DataParsing)?
        .parse()?;

    // Get only one column
    let domain = line
        .next()
        .ok_or_else(|| ZoneParserError::DataParsing)?
        .to_string();

    Ok((priority, domain))
}

fn parse_name_record_data(line: &mut SplitWhitespace, root_domain: &str) -> ParserResult<String> {
    // Get only one column
    let name = line.next().ok_or_else(|| ZoneParserError::DataParsing)?;

    // Add the root domain if we are missing the period
    if !name.ends_with('.') {
        return Ok(name.to_string() + "." + root_domain);
    }

    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_zone_file() {
        let file = "
            $ORIGIN example.com.     ; designates the start of this zone file in the namespace
            $TTL 3600                ; default expiration time (in seconds) of all RRs without their own TTL value
            example.com.  IN  MX    10 mail.example.com.  ; mail.example.com is the mailserver for example.com
            example.com.  IN  CNAME example.com. ; More comments
            @             IN  A     127.0.0.1
            @             IN  AAAA  2001:db8:10::2        ; IPv6 address for ns.example.com
            example.com.  IN  NS    ns.somewhere ; ns.somewhere.example is a backup nameserver for example.com
        ";

        let results = parse(file.as_bytes().to_vec()).unwrap();

        assert_eq!(results.name, "example.com.");
        assert_eq!(results.time_to_live, 3600);

        let mx_record = results.records.get(0).unwrap();
        match mx_record {
            RecordData::MXRecord(priority, domain) => {
                assert_eq!(domain, "mail.example.com.");
                assert_eq!(*priority, 10)
            }
            _ => panic!("Bad match"),
        }
        let cname_record = results.records.get(1).unwrap();
        match cname_record {
            RecordData::CNameRecord(domain) => assert_eq!(domain, "example.com."),
            _ => panic!("Bad match"),
        }
        let a_record = results.records.get(2).unwrap();
        match a_record {
            RecordData::ARecord(record) => {
                assert_eq!("127.0.0.1".parse::<Ipv4Addr>().unwrap(), *record)
            }
            _ => panic!("Bad match"),
        }
        let aaaa_record = results.records.get(3).unwrap();
        match aaaa_record {
            RecordData::AAAARecord(record) => {
                assert_eq!("2001:db8:10::2".parse::<Ipv6Addr>().unwrap(), *record)
            }
            _ => panic!("Bad match"),
        }

        let ns_record = results.records.get(4).unwrap();
        match ns_record {
            RecordData::NSRecord(record) => {
                assert_eq!(record, "ns.somewhere.example.com.")
            }
            _ => panic!("Bad match"),
        }
    }
}
