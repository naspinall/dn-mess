pub(crate) mod a_records;
pub(crate) mod zones;

use sqlite::{Connection, Error};
use tokio::sync::RwLock;

struct DatabaseConnection {
    connection: RwLock<Connection>,
}

pub fn run_migrations(connection: &Connection) -> Result<(), Error> {
    // List of all migrations, mind the order for foreign key issues
    let migrations = [zones::Zone::migrate, a_records::ARecord::migrate];

    // Run all the migrations one by one
    for migration in migrations.iter() {
        migration(connection)?;
    }

    Ok(())
}

// Async Wrapper for a database connection
// POC WIP etc etc
impl DatabaseConnection {
    pub fn open() -> Result<DatabaseConnection, Error> {
        let connection = sqlite::open(":memory:")?;
        Ok(DatabaseConnection {
            connection: RwLock::new(connection),
        })
    }

    pub async fn execute<T: AsRef<str>>(&self, statement: T) -> Result<(), Error> {
        // Get a write lock on the rows
        let connection = self.connection.write().await;

        // Run execute
        connection.execute(statement)
    }
}
