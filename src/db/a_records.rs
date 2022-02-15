use sqlite::{Connection, Error, State};

pub(crate) struct ARecord {
    id: i64,
    name: String,
}

const TABLE_DEFINITION: &str = "
    create table if not exists a_records (
        id integer primary key autoincrement,
        name text not null,
        time_to_live integer not null,
        ip_address integer not null,
        zone_id integer not null,
        foreign key(zone_id) references zones(id)
    );
";

impl ARecord {
    pub fn migrate(connection: &Connection) -> Result<(), Error> {
        connection.execute(TABLE_DEFINITION)
    }

    pub fn insert_domain(connection: &Connection, name: &str) -> Result<(), Error> {
        connection.execute("insert into domains (name) values (\'google.com\');")
    }

    pub fn get_domains(connection: &Connection) -> Result<Vec<ARecord>, Error> {
        let mut statement = connection.prepare("select * from domains;")?;

        let mut a_records = vec![];

        while let State::Row = statement.next()? {
            a_records.push(ARecord {
                id: statement.read::<i64>(0)?,
                name: statement.read::<String>(1)?,
            });
        }

        Ok(a_records)
    }
}
