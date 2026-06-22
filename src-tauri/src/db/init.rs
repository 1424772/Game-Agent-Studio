use rusqlite::{Connection, Result};

use super::migrations;

pub fn initialize_database(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    migrations::run_migrations(&conn)?;
    Ok(conn)
}
