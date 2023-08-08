// -- Constants

#[allow(dead_code)]
pub const TYPE_STRING: &[&str] = &["source::text", "destination::text", "weight::float8"];

#[allow(dead_code)]
pub const BEGIN: &str = "BEGIN";

#[allow(dead_code)]
pub const CREATE_TABLE: &str = "
CREATE TABLE IF NOT EXISTS graph (
    source VARCHAR(32),
    destination VARCHAR(32),
    weight NUMERIC(10, 5)
)";

#[allow(dead_code)]
pub const SELECT_EXISTS: &str = "SELECT EXISTS(SELECT 1 FROM graph LIMIT 1)";

#[allow(dead_code)]
pub const INSERT_SQL: &str = "INSERT INTO graph (source, destination, weight) VALUES ($1, $2, $3)";

#[allow(dead_code)]
pub const COMMIT: &str = "COMMIT";

#[allow(dead_code)]
pub const SELECT_QUERY: &str = "SELECT source, destination, weight FROM graph;";
