use sqlite::{Connection, Error, State};

pub(crate) struct Zone {
    id: i64,           // Numeric id for each zone
    origin: String,    // Origin for the zone
    time_to_live: i64, // Default time to live for all domains in the zone
}

const TABLE_DEFINITION: &str = "
    create table if not exists zones (
        id integer primary key autoincrement,
        origin text not null,
        time_to_live integer
    );
";

impl Zone {
    pub fn migrate(connection: &Connection) -> Result<(), Error> {
        connection.execute(TABLE_DEFINITION)
    }
}
