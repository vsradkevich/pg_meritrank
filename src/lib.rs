// Importing modules for the library
// mod edge; // This module contains edge related operations and data structures
mod error; // This module contains error types and handling logic
mod graph; // This module is for graph related operations
// #[cfg(feature = "shared")]
// mod shared; // This module contains shared data structures
mod lib_graph; // This module contains graph related operations and data structures
mod tests;

use pgx::*;

#[allow(unused_imports)]
use graph::{GraphManipulationError, GraphSingleton}; // Importing types from the `graph` module

// pgx specific macros
pg_module_magic!();

// The postgres external function to return a greeting message.
#[pg_extern]
/// Returns a static greeting message.
fn hello_hello_world() -> &'static str {
    "Hello, hello_world"
}

#[pg_extern]
/// Inserts a record in the `graph` table and triggers a SPI run.
///
/// # Arguments
///
/// * `source` - A string slice holding the source node's name.
/// * `destination` - A string slice holding the destination node's name.
/// * `weight` - A float64 holding the weight of the edge.
fn insert_and_trigger(source: &str, destination: &str, weight: f64) {
    let insert_sql = format!(
        "INSERT INTO graph (source, destination, weight) VALUES ('{}', '{}', {});",
        source, destination, weight
    );
    match Spi::run(&insert_sql) {
        Ok(_) => println!("Inserted record into graph table successfully."),
        Err(err) => println!("Error inserting record into graph table: {}", err),
    }
}

/// This module is required by `cargo pgx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    use std::fs;

    use std::env;

    // Dotenv for .env file
    use dotenv::dotenv;

    /// Setup function to perform one-off initialization when the pg_test framework starts
    pub fn setup(_options: Vec<&str>) {
        dotenv().ok();

        // Read initial_structure.sql and implement_triggers.sql
        // let dir_list = fs::read_dir("").unwrap();
        // print!("dir_list: {:?}", dir_list);
        let initial_structure_sql_file =
            env::var("INITIAL_STRUCTURE_SQL").expect("Could not read initial_structure.sql");

        let implement_triggers_sql_file =
            env::var("IMPLEMENT_TRIGGERS_SQL").expect("Could not read initial_structure.sql");

        let _initial_structure_sql = fs::read_to_string(initial_structure_sql_file);
        let _implement_triggers_sql = fs::read_to_string(implement_triggers_sql_file);

        // apply initial structure to database
        // let result = Spi::run(initial_structure_sql.unwrap().as_ref()); // Convert to &str using as_ref()
        // assert!(result.is_ok());

        // let database_url = env::var("DATABASE_URL")?;
        // let pool = sqlx::PgPool::connect(&database_url).await?;
        //
        // sqlx::query(&initial_structure_sql).execute(&pool).await?;
        // sqlx::query(&implement_triggers_sql).execute(&pool).await?;
    }

    /// Returns any postgresql.conf settings that are required for your tests
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec!["search_path = public"]
    }
}
