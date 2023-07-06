mod lib_graph;

// Standard library imports
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// External crate imports
use lazy_static::lazy_static;
#[allow(unused_imports)]
use pgx::{
    *,
    prelude::*,
    pg_sys::{Oid, Datum, BuiltinOid},
    spi::{SpiTupleTable, SpiClient, OwnedPreparedStatement, Error},
};
use rand::Rng;


// Current crate (`crate::`) imports
use crate::lib_graph::{MeritRank, MeritRankError, MyGraph, NodeId};

// pgx specific macro
pg_module_magic!();

// type alias
type SpiTuple = spi::SpiHeapTupleData;


// Constants
const TYPE_STRING: &[&str] = &[
    "source::text",
    "destination::text",
    "weight::float8",
];

const BEGIN: &str = "BEGIN";

const CREATE_TABLE: &str = "
CREATE TABLE IF NOT EXISTS graph (
    source VARCHAR(32),
    destination VARCHAR(32),
    weight NUMERIC(10, 5)
)";

const SELECT_EXISTS: &str = "SELECT EXISTS(SELECT 1 FROM graph LIMIT 1)";

const INSERT_SQL: &str = "INSERT INTO graph (source, destination, weight) VALUES ($1, $2, $3)";

const COMMIT: &str = "COMMIT";

const SELECT_QUERY: &str = "SELECT source, destination, weight FROM graph;";


// Define a new error type for better error handling
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("Edge creation error: {0}")]
    EdgeCreationError(String),

    #[error("Node creation error: {0}")]
    NodeCreationError(String),

    #[error("Table creation error: {0}")]
    TableCreationError(String),

    #[error("Transaction initiation error: {0}")]
    TransactionBeginError(String),

    #[error("Statement preparation error: {0}")]
    StatementPreparationError(String),

    #[error("Transaction commit error: {0}")]
    TransactionCommitError(String),

    #[error("Node not found error: {0}")]
    NodeNotFoundError(String),

    #[error("Data extraction error: {0}")]
    DataExtractionError(String),

    #[error("Weight extraction error: {0}")]
    WeightExtractionError(String),

    #[error("Records extraction error: {0}")]
    RecordsExtractionError(String),

    #[error("Fetch records error: {0}")]
    FetchRecordsError(String),

    #[error("Graph generation error: {0}")]
    GraphGenerationError(String),

    #[error("Graph write error: {0}")]
    WriteGraphError(String),

    #[error("Graph read error: {0}")]
    CheckRecordsError(String),

    #[error("Node name not found error: {0}")]
    NodeNameNotFoundError(String),

    // fail to choose node
    #[error("Node selection error: {0}")]
    NodeSelectionError(String),

    // The `From<pgx::spi::Error> for GraphError` implementation is
    // automatically done by the `ThisError` derive macro because of
    // the `#[from]` attribute below.
    #[error(transparent)]
    SpiError(#[from] pgx::spi::Error),

    #[error(transparent)]
    MeritRankError(#[from] MeritRankError),

    #[error("Mutex lock error: {0}")]
    MutexLockError(String),

    // TODO: Include any other custom error types here
}


// Singleton instance
lazy_static! {
    pub static ref GRAPH: Arc<Mutex<GraphSingleton>> = Arc::new(Mutex::new(GraphSingleton::new()));
}

// GraphSingleton structure
pub struct GraphSingleton {
    graph: MyGraph,
    node_names: HashMap<String, NodeId>,
}

impl GraphSingleton {
    /// Constructor
    pub fn new() -> GraphSingleton {
        GraphSingleton {
            graph: MyGraph::new(),
            node_names: HashMap::new(),
        }
    }

    pub fn get_rank() -> Result<MeritRank, GraphError> {
        match GRAPH.lock() {
            Ok(mut graph) => {
                let merit_rank = MeritRank::new(graph.graph.clone())?;
                Ok(merit_rank)
            }
            Err(e) => Err(GraphError::MutexLockError(format!("Mutex lock error: {}", e))),
        }
    }

    // Node-related methods

    /// Creates a new node with the given name and returns its ID.
    /// If the node already exists, it returns the ID of the existing node.
    ///
    /// # Arguments
    ///
    /// * `node_name` - The name of the node to create or retrieve.
    ///
    /// # Errors
    ///
    /// Returns a `GraphError::MutexLockError` if the mutex lock fails.
    pub fn add_node(node_name: &str) -> Result<NodeId, GraphError> {
        match GRAPH.lock() {
            Ok(mut graph) => graph.get_node_id(node_name),
            Err(e) => Err(GraphError::MutexLockError(format!("Mutex lock error: {}", e))),
        }
    }

    // This method remains largely the same, it's already well structured
    fn get_node_id(&mut self, node_name: &str) -> Result<NodeId, GraphError> {
        if let Some(&node_id) = self.node_names.get(node_name) {
            Ok(node_id)
        } else {
            let new_node_id = self.graph.node_count() + 1;
            let node_id = NodeId::UInt(new_node_id);
            self.node_names.insert(node_name.to_string(), node_id);
            self.graph.add_node(node_id.into());
            Ok(node_id)
        }
    }

    /// Returns the name of the node with the given ID.
    fn node_name_to_id(node_name: &str) -> Result<NodeId, GraphError> {
        match GRAPH.lock() {
            Ok(mut graph) => {
                if let Some(&node_id) = graph.node_names.get(node_name) {
                    Ok(node_id)
                } else {
                    Err(GraphError::NodeNotFoundError(format!("Node not found: {}", node_name)))
                }
            }
            Err(e) => Err(GraphError::MutexLockError(format!("Mutex lock error: {}", e))),
        }
    }

    /// Returns the ID of the node with the given name.
    fn node_id_to_name(node_id: NodeId) -> Result<String, GraphError> {
        match GRAPH.lock() {
            Ok(mut graph) => {
                for (name, id) in graph.node_names.iter() {
                    if *id == node_id {
                        return Ok(name.to_string());
                    }
                }
                Err(GraphError::NodeNotFoundError(format!("Node not found: {}", node_id)))
            }
            Err(e) => Err(GraphError::MutexLockError(format!("Mutex lock error: {}", e))),
        }
    }

    // Edge-related methods

    fn add_edge(&mut self, source: NodeId, target: NodeId, weight: f64) -> Result<(), GraphError> {
        match self.graph.add_edge(source, target, weight) {
            Ok(_) => {
                Spi::connect(|mut client| {
                    let edge = (source, target, weight);
                    self.insert_edge_into_graph(&edge, &mut client)?;
                    Ok::<(), GraphError>(())
                })?;
                Ok(())
            }
            Err(_err) => {
                Err(GraphError::EdgeCreationError("Error adding edge".to_string()))
            }
        }
    }

    // Database-related methods

    /// Creates the graph table in the database if it doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns `GraphError::TableCreationError` if an error occurs while creating the table.
    pub fn create_graph_table() -> Result<(), GraphError> {
        Spi::run(CREATE_TABLE).map_err(|err| {
            eprintln!("Error creating table: {}", err);
            GraphError::TableCreationError("Error creating table".to_string())
        })
    }

    /// Checks if records exist in the graph table.
    ///
    /// # Errors
    ///
    /// Returns `GraphError::SpiError` if an error occurs while accessing the database.
    pub fn records_exist() -> Result<bool, GraphError> {
        match Spi::get_one::<bool>(SELECT_EXISTS) {
            Ok(Some(true)) => Ok(true),
            Ok(_) => Ok(false),
            Err(err) => Err(GraphError::SpiError(err)),
        }
    }

    /// Generates a graph with the given number of nodes and probability coefficients
    ///
    /// # Parameters
    ///
    /// - `num_nodes`: The number of nodes to generate
    /// - `border_probability`: The probability of creating an edge between nodes
    pub fn generate_graph(&mut self, num_nodes: usize, border_probability: f64) -> Result<(), GraphError> {
        let mut rng = rand::thread_rng();
        let node_names: Vec<String> = self.generate_node_names(num_nodes);

        let start = std::time::Instant::now();

        for i in 0..num_nodes {
            let source = self.get_node_id(&node_names[i])?;
            for j in 0..num_nodes {
                if i != j && rng.gen::<f64>() < border_probability {
                    let target = self.get_node_id(&node_names[j])?;
                    let weight = rng.gen_range(0.0..=1.0);
                    println!("Adding edge from {} to {} with weight {}", node_names[i], node_names[j], weight);
                    self.add_edge(source, target, weight)?;
                }
            }
        }

        let stop = std::time::Instant::now();
        println!("Time to generate graph: {:?}", stop.duration_since(start));
        println!("Graph has {} nodes and {} edges", self.graph.node_count(), self.graph.get_edges().len());

        Ok(())
    }

    /// Generates a random string of the given length
    ///
    /// # Parameters
    ///
    /// - `length`: The length of the string to generate
    fn generate_random_string(&mut self, length: usize) -> String {
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| {
                let ascii_value = if rng.gen::<bool>() {
                    rng.gen_range(b'a'..=b'z')
                } else {
                    rng.gen_range(b'A'..=b'Z')
                };
                ascii_value as char
            })
            .collect()
    }

    /// Generates unique node names
    ///
    /// # Parameters
    ///
    /// - `num_nodes`: The number of node names to generate
    fn generate_node_names(&mut self, num_nodes: usize) -> Vec<String> {
        (0..num_nodes)
            .map(|_| self.generate_random_string(8))
            .collect()
    }

    /// Method to write the graph to the database
    pub fn write_graph_to_database(&self) -> Result<(), GraphError> {
        Spi::connect(|mut client| {
            self.create_graph_table_if_not_exists(&mut client)?;
            println!("Created graph table");
            self.begin_transaction(&mut client)?;
            println!("Began transaction");

            for edge in self.graph.get_edges() {
                self.insert_edge_into_graph(&edge, &mut client)?;
            }

            self.commit_transaction(&mut client)?;
            Ok(())
        })
    }

    /// Creates the graph table in the database if it does not exist
    fn create_graph_table_if_not_exists(&self, client: &mut SpiClient) -> Result<(), GraphError> {
        client.update(CREATE_TABLE, None, None)
            .map_err(|_| GraphError::TableCreationError("Error creating table".to_string()))?;
        Ok(())
    }

    /// Begins a database transaction
    fn begin_transaction(&self, client: &mut SpiClient) -> Result<(), GraphError> {
        client.update(BEGIN, None, None)
            .map_err(|e| GraphError::TransactionBeginError(e.to_string()))?;
        Ok(())
    }

    /// Prepares an SQL insert statement
    fn prepare_insert_statement(client: &mut SpiClient) -> Result<OwnedPreparedStatement, GraphError> {
        let param_types = Some(vec![
            BuiltinOid::TEXTOID.into(),
            BuiltinOid::TEXTOID.into(),
            BuiltinOid::FLOAT8OID.into(),
        ]);

        client.prepare(INSERT_SQL, param_types)
            .map(|stmt| stmt.keep())
            .map_err(|_| GraphError::StatementPreparationError("Error preparing insert statement".to_string()))
    }

    fn insert_into_graph(edge: &(NodeId, NodeId, f64), client: &mut SpiClient) -> Result<(), GraphError> {
        // Obtain immutable access to the GRAPH singleton
        let graph = GRAPH.lock().unwrap();

        // Insert the edge into the graph
        graph.insert_edge_into_graph(edge, client)
        // Ok(())
    }

    /// Inserts an edge into the graph database
    fn insert_edge_into_graph(&self, edge: &(NodeId, NodeId, f64), client: &mut SpiClient) -> Result<(), GraphError> {
        let (source, destination, weight) = edge;
        let node_ids: HashMap<NodeId, String> = self.node_names.iter().map(|(name, id)| (*id, name.clone())).collect();

        // Get the node names from the node ids
        let source_name = node_ids.get(source)
            .ok_or_else(|| GraphError::NodeNotFoundError("Source node not found".to_string()))?.clone();
        let destination_name = node_ids.get(destination)
            .ok_or_else(|| GraphError::NodeNotFoundError("Destination node not found".to_string()))?.clone();

        // Convert String to Datum
        let source_datum = source_name.into_datum();
        let destination_datum = destination_name.into_datum();
        let weight_datum = weight.into_datum();

        // Execute the insert statement
        let params = Some(vec![source_datum, destination_datum, weight_datum]);

        let stmt: OwnedPreparedStatement = Self::prepare_insert_statement(client)?;

        client.update(stmt, None, params)
            .map_err(|_| GraphError::EdgeCreationError("Error inserting edge into graph".to_string()))?;
        Ok(())
    }

    /// Commits a database transaction
    fn commit_transaction(&self, client: &mut SpiClient) -> Result<(), GraphError> {
        client.update(COMMIT, None, None)
            .map_err(|_| GraphError::TableCreationError("Error committing transaction".to_string()))?;
        Ok(())
    }

    /// Fetches records from the graph table.
    ///
    /// This method is responsible for fetching records from the graph table.
    /// It establishes a connection to the SPI client, prepares and executes the SELECT query,
    /// and extracts the records from the returned rows.
    pub fn fetch_records(&mut self) -> Result<Vec<(NodeId, NodeId, f64)>, GraphError> {
        Spi::connect(|client| {
            let prepared_stmt = client.prepare(SELECT_QUERY, None)
                .map_err(|_| GraphError::StatementPreparationError("Error preparing SELECT statement".to_string()))?;

            let rows = client.select(&prepared_stmt, None, None)
                .map_err(|_| GraphError::DataExtractionError("Error selecting rows".to_string()))?;

            // Function to extract records from the rows and return them
            self.extract_records_from_rows(rows)
        })
    }

    /// Extracts records from rows.
    ///
    /// This method iterates through the provided rows, extracts the required data
    /// from each row and stores them in a vector as records.
    fn extract_records_from_rows(&mut self, rows: SpiTupleTable) -> Result<Vec<(NodeId, NodeId, f64)>, GraphError> {
        let mut records = Vec::new();

        for row in rows {
            let (source, destination, weight) = self.extract_data_from_row(&row)
                .map_err(|_| GraphError::RecordsExtractionError("Error extracting records".to_string()))?;

            records.push((source, destination, weight));
            println!("ROW source: {}, destination: {}, weight: {}", source, destination, weight)
        }
        println!("extract_records_from_rows worked");
        Ok(records)
    }

    /// Extracts data from a row.
    ///
    /// This method extracts the source, destination, and weight data from a given row.
    fn extract_data_from_row(&mut self, row: &SpiTuple) -> Result<(NodeId, NodeId, f64), GraphError> {
        let source = self.extract_node_id_from_row(&row, 0)
            .map_err(|_| GraphError::DataExtractionError("Failed to extract source value".to_string()))?;

        let destination = self.extract_node_id_from_row(&row, 1)
            .map_err(|_| GraphError::DataExtractionError("Failed to extract destination value".to_string()))?;

        let weight = Self::extract_weight_from_row(&row, 2)
            .map_err(|_| GraphError::WeightExtractionError("Failed to extract weight value".to_string()))?;

        Ok((source, destination, weight))
    }

    /// Extracts a node id from a row.
    ///
    /// This method extracts a node id from a given row using the provided index.
    fn extract_node_id_from_row(&mut self, row: &SpiTuple, index: usize) -> Result<NodeId, GraphError> {
        match row.get(index) {
            Ok(Some(value)) => self.get_node_id(value),
            _ => Err(GraphError::DataExtractionError("Failed to extract node id".to_string())),
        }
    }

    /// Helper function to extract a weight from a row
    fn extract_weight_from_row(row: &SpiTuple, index: usize) -> Result<f64, GraphError> {
        match row.get(index) {
            Ok(Some(value)) => Ok(value),
            _ => Err(GraphError::WeightExtractionError("Failed to extract weight value".to_string())),
        }
    }
}


// The postgres external function to return a greeting message.
#[pg_extern]
fn hello_hello_world() -> &'static str {
    "Hello, hello_world"
}

// The postgres external function to create a graph table.
#[pg_extern]
fn create_graph_table() -> Result<(), GraphError> {
    GraphSingleton::create_graph_table()
}

// Function to test insert nodes and an edge into the graph.
fn insert_into_graph(node_name1: &str, node_name2: &str, weight: f64) -> Result<(), GraphError> {
    Spi::connect(|mut client| {
        let node1 = GraphSingleton::add_node(node_name1)?;
        let node2 = GraphSingleton::add_node(node_name2)?;
        let edge = (node1, node2, weight);

        println!("source: {}, target: {}", node1, node2);
        println!("edge: {:?}", edge);

        // Insert the edge into the graph
        GraphSingleton::insert_into_graph(&edge, &mut client)
            .map_err(|_| GraphError::EdgeCreationError("Error inserting edge into graph".to_string()))?;

        println!("Inserted edge ({}, {}, {})", node1, node2, weight);
        Ok(())
    })
}

/// Function to initialize the GRAPH singleton
fn init_graph() -> Result<(), GraphError> {
    // Create the graph table if it doesn't exist
    GraphSingleton::create_graph_table().map_err(|e| {
        println!("Error creating table: {}", e);
        GraphError::TableCreationError("Error creating table".to_string())
    })?;

    println!("Table created successfully.");

    let mut graph = GRAPH.lock().unwrap();

    // Check if records exist in the graph table
    match GraphSingleton::records_exist() {
        Ok(true) => {
            println!("Records already exist in the graph table.");

            // Records exist, fetch them
            match graph.fetch_records() {
                Ok(records) => {
                    // Process the fetched records
                    for record in records {
                        let (source, destination, weight) = record;
                        println!("Source: {:?}, Destination: {:?}, Weight: {}", source, destination, weight);

                        // Add the edge to the graph
                        graph.add_edge(source, destination, weight)?;
                    }
                    Ok(())
                }
                Err(e) => {
                    println!("Error fetching records: {}", e);
                    Err(GraphError::FetchRecordsError("Error fetching records".to_string()))
                }
            }
        }
        Ok(false) => {
            println!("No records found. Creating records.");

            // Generate a graph
            graph.generate_graph(42, 0.13).map_err(|e| {
                println!("Error generating graph: {:?}", e);
                GraphError::GraphGenerationError("Error generating graph".to_string())
            })?;

            println!("Graph generated successfully.");

            // Write the graph to the database
            graph.write_graph_to_database().map_err(|e| {
                println!("Error writing graph to database: {}", e);
                GraphError::WriteGraphError("Error writing graph to database".to_string())
            })?;

            println!("Graph written to database successfully.");
            Ok(())
        }
        Err(e) => {
            println!("Error checking records: {}", e);
            Err(GraphError::CheckRecordsError("Error checking records".to_string()))
        }
    }
}

#[pg_extern]
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

use serde::{Serialize, Deserialize};

#[derive(PostgresType, Serialize, Deserialize, Debug)]
pub struct NodeRating {
    node: String,
    rating: f64,
}

impl PartialEq for NodeRating {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl NodeRating {
    fn new(node: String, rating: f64) -> Self {
        Self { node, rating }
    }
}

// impl FromDatum for NodeRating {
//     const GET_TYPOID: bool = false;
//
//     unsafe fn from_datum(
//         datum: pg_sys::Datum,
//         is_null: bool,
//         _typoid: pg_sys::Oid,
//     ) -> Option<Self> where Self: Sized {
//         if is_null {
//             None
//         } else {
//             let tup = try_into::<(String, f64)>(datum, pg_sys::TEXTOID)?;
//             match tup {
//                 Some((node, rating)) => Some(Self { node, rating }),
//                 None => None,
//             }
//         }
//     }
//
//     unsafe fn from_polymorphic_datum(
//         datum: pg_sys::Datum,
//         is_null: bool,
//         _typoid: pg_sys::Oid,
//     ) -> Option<Self> {
//         Self::from_datum(datum, is_null)
//     }
//
//     unsafe fn from_datum_in_memory_context(
//         memory_context: PgMemoryContexts,
//         datum: pg_sys::Datum,
//         is_null: bool,
//         typoid: pg_sys::Oid,
//     ) -> Option<Self> {
//         memory_context.switch_to(|_| Self::from_polymorphic_datum(datum, is_null, typoid))
//     }
// }

#[pg_extern(immutable, parallel_safe)]
fn calculate_ratings(
    ego: &str, // The "ego" node for which we are calculating ratings
    steps: i32, // The number of calculation steps to perform
    limit: Option<i32>, // An optional limit on the number of ratings to return
) -> Vec<NodeRating> { // Return a vector of NodeRating objects

    // Convert the ego string into a NodeId
    let ego_id = GraphSingleton::node_name_to_id(ego).unwrap();

    // Initialize a new graph and merit rank object
    let mut merit_rank = match GraphSingleton::get_rank() {
        Ok(rank) => rank,
        Err(e) => {
            println!("Error getting rank: {}", e);
            return Vec::new(); // Return an empty Vec if there was an error
        }
    };

    // Attempt to calculate merit ranks
    match merit_rank.calculate(ego_id, 10000) {
        Ok(_) => println!("MeritRank calculated successfully."),
        Err(e) => println!("Error calculating MeritRank: {}", e),
    };

    // Convert the limit value to usize if necessary
    let limit_usize = limit.map(|l| l as usize);

    // Get ranks and handle potential error
    let ranks: HashMap<NodeId, f64> = match merit_rank.get_ranks(ego_id, limit_usize) {
        Ok(ranks) => ranks,
        Err(e) => {
            println!("Error getting ranks: {}", e);
            HashMap::new() // Return an empty HashMap if there was an error
        }
    };

    // Convert the ranks HashMap into a Vec of NodeRating objects
    let mut rank_vec: Vec<NodeRating> = Vec::new();
    for (node_id, rank) in ranks {
        // Assuming that we have a function to convert NodeId back to node name
        let node_name = GraphSingleton::node_id_to_name(node_id).unwrap();
        rank_vec.push(NodeRating::new(node_name, rank));
    }

    rank_vec // Return the vector of NodeRating objects
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    // use spi::{SpiTupleTable};
    use pgx::prelude::*;
    use std::str::FromStr;
    // use pgx::spi::Error;
    use pgx::spi::Error::InvalidPosition;
    // use pgx::spi::SpiHeapTupleData;
    use crate::{GraphError, NodeRating};
    // use crate::GraphSingleton;
    use rand::prelude::SliceRandom;

    type PgNum = Numeric<10, 5>;


    #[pg_test]
    fn test_hello_hello_world() {
        let result = crate::hello_hello_world();
        assert_eq!("Hello, hello_world", result);
        println!("Test hello_hello_world passed.");
    }

    // #[pg_test]
    fn test_hello_hello_world_spi() {
        let result: String = Spi::get_one("SELECT hello_hello_world();").unwrap().unwrap_or_default();
        assert_eq!("Hello, hello_world", result);
        println!("Test hello_hello_world_spi passed.");
    }

    #[pg_test]
    fn test_check_table() -> Result<(), GraphError> {
        // Create the table
        Spi::run("SELECT create_graph_table();")?;
        println!("Table created successfully.");

        // Check the table existence
        let result: Option<String> = Spi::get_one("SELECT tablename::text FROM pg_tables WHERE tablename = 'graph';")?;
        assert_eq!(result, Some("graph".to_string()));
        println!("Table exists.");

        // Insert a row into the table
        Spi::run("INSERT INTO graph (source, destination, weight) VALUES ('node1', 'node2', 1);")?;
        println!("Record inserted into table.");

        // Check if the row has been inserted
        let source: Option<String> = Spi::get_one("SELECT source::text FROM graph WHERE source = 'node1' AND destination = 'node2';")?;
        let destination: Option<String> = Spi::get_one("SELECT destination::text FROM graph WHERE source = 'node1' AND destination = 'node2';")?;
        let weight: Option<f64> = Spi::get_one("SELECT weight::float8 FROM graph WHERE source = 'node1' AND destination = 'node2';").unwrap();
        assert_eq!(source, Some("node1".to_string()));
        assert_eq!(destination, Some("node2".to_string()));
        assert_eq!(weight, Some(1.0));
        println!("Record correctly retrieved from table.");

        // Delete the row from the table
        Spi::run("DELETE FROM graph WHERE source = 'node1' AND destination = 'node2';")?;
        println!("Record deleted from table.");

        // Check if the row has been deleted
        let deleted_source = Spi::get_one::<String>("SELECT source FROM graph WHERE source = 'node1' AND destination = 'node2';");
        let deleted_destination = Spi::get_one::<String>("SELECT destination::text FROM graph WHERE source = 'node1' AND destination = 'node2';");
        let deleted_weight = Spi::get_one::<f64>("SELECT weight::float8 FROM graph WHERE source = 'node1' AND destination = 'node2';");

        // assert_eq!(deleted_source, None);
        match deleted_source {
            Ok(value) => assert_eq!(value, None), // Expecting the row to be deleted
            Err(_) => (), // Ignore the error because we expect the row to be missing
        }
        println!("Record successfully deleted from table.");

        assert_eq!(deleted_destination, Err(InvalidPosition));
        assert_eq!(deleted_weight, Err(InvalidPosition));
        Ok(())
    }


    // #[pg_test]
    fn test_create_table() -> Result<(), GraphError> {
        Spi::run("SELECT create_graph_table();")?;
        println!("Test create_table passed.");
        Ok(())
    }

    fn test_insert_into_graph() -> Result<(), GraphError> {
        Spi::run("INSERT INTO graph (source, destination, weight) VALUES ('node1', 'node2', 1.23);")?;
        println!("Test insert_into_graph passed.");
        Ok(())
    }

    // #[pg_test]
    fn test_select_graph_cast() {
        let row: (Option<String>, Option<String>, Option<String>) =
            Spi::get_three("SELECT source, destination, weight::text FROM graph WHERE source = 'node1' AND destination = 'node2';").unwrap();
        assert!(row.0.is_some() && row.1.is_some() && row.2.is_some());
        let (source, destination, weight) = row;
        assert_eq!(source.unwrap(), "node1");
        assert_eq!(destination.unwrap(), "node2");
        assert_eq!(weight.unwrap(), "1.23000");
        println!("Test select_graph_cast passed.");
    }

    fn test_select_graph_uncast() -> Result<(), GraphError> {
        let row: (Option<String>, Option<String>, Option<PgNum>) =
            Spi::get_three("SELECT source, destination, weight FROM graph WHERE source = 'node1' AND destination = 'node2';").unwrap();
        assert!(row.0.is_some() && row.1.is_some() && row.2.is_some());
        let (source, destination, weight) = row;
        assert_eq!(source.unwrap(), "node1");
        assert_eq!(destination.unwrap(), "node2");
        assert_eq!(weight.unwrap(), PgNum::from_str("1.23").unwrap());
        println!("Test select_graph_uncast passed.");
        Ok(())
    }

    fn test_trigger_on_insert() -> Result<(), GraphError> {
        // Assuming you have a function called `insert_and_get_graph` that triggers on insert and returns the updated graph
        let _ = Spi::run("SELECT insert_and_trigger('node3', 'node4', 2.34);").unwrap();

        // Check the graph's state
        let select_sql = "SELECT source, destination, weight FROM graph WHERE source = 'node3' AND destination = 'node4';";
        let row: (Option<String>, Option<String>, Option<PgNum>) = Spi::get_three(select_sql).unwrap();
        assert!(row.0.is_some() && row.1.is_some() && row.2.is_some());
        let (source, destination, weight) = row;
        assert_eq!(source.unwrap(), "node3");
        assert_eq!(destination.unwrap(), "node4");
        assert_eq!(weight.unwrap(), PgNum::from_str("2.34").unwrap());
        println!("Test trigger_on_insert passed.");
        Ok(())
    }

    #[pg_test]
    fn test_all() -> Result<(), GraphError> {
        test_create_table()?;
        test_insert_into_graph()?;
        test_select_graph_uncast()?;
        test_trigger_on_insert()?;
        println!("Test all passed.");
        Ok(())
    }

    #[pg_test]
    fn test_init_graph() -> Result<(), GraphError> {
        // let result: Option<String> = Spi::get_one("SELECT init_graph();").unwrap();
        let _result = crate::init_graph();
        println!("Graph initialized.");

        let _result = crate::insert_into_graph("node1", "node2", 1.23);
        let _result = crate::insert_into_graph("node1", "node3", 0.89);
        println!("Nodes inserted into graph.");

        wait_until_graph_size(42);
        println!("Graph size reached.");

        let get_unique_source_values = get_unique_source_values()?;
        println!("Unique source values: {:?}", get_unique_source_values);


        let ego_node = select_ego_node()?;
        println!("Ego node selected: {}", ego_node);
        calculate_and_display_ratings(&ego_node)?;
        println!("Test ratings_calculation passed.");

        println!("Calculate rating via Select query");
        fetch_ratings_via_select(&ego_node)?;


        // let result = crate::hello_hello_world();
        // assert_eq!("Hello, hello_world", result);
        println!("Test init_graph passed.");
        Ok(())
    }

    fn wait_until_graph_size(reached_size: usize) {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            println!("Waiting for graph to reach size {}...", reached_size);
            let result = Spi::get_one::<i64>("SELECT COUNT(DISTINCT source) FROM graph;");
            match result {
                Ok(count_option) => {
                    let count: i64 = count_option.unwrap_or_default();
                    if count >= reached_size as i64 {
                        break;
                    }
                    println!("Current graph size: {}", count);
                }
                Err(e) => {
                    if e.to_string().contains("relation \"graph\" does not exist") {
                        println!("Graph table not yet created");
                    } else {
                        // you might want to re-throw the error or handle it differently here
                        panic!("Unexpected error: {}", e);
                    }
                }
            }
        }
    }

    fn get_unique_source_values() -> Result<Vec<String>, GraphError> {
        let select_query = "SELECT DISTINCT source FROM graph";
        let mut source_values = Vec::new();

        Spi::connect(|client| {
            let result_set = client.select(select_query, None, None)?;

            // println!("Number of rows returned: {}", result_set.len());

            for row in result_set {
                let source: Option<String> = row.get(1).unwrap_or(None);

                // println!("Source value: {:?}", source);

                if let Some(source_value) = source {
                    source_values.push(source_value);
                }
            }

            Ok::<(), GraphError>(())
        })?;

        // println!("Unique source values: {:?}", source_values);

        Ok(source_values)
    }

    fn select_ego_node() -> Result<String, GraphError> {
        let mut nodes = Vec::new();
        Spi::connect(|client| {
            let sql = "SELECT source FROM graph GROUP BY source HAVING COUNT(*) >= 5;";
            let select_rows = client.select(sql, None, None).unwrap();
            // .map_err(|_| Err(GraphError::NodeNameNotFoundError("Error occurred while selecting rows".to_string())));

            println!("Selecting ego node...");

            for row in select_rows {
                let node_option: Option<String> = match row.get(1) {
                    Ok(node) => node,
                    Err(err) => {
                        println!("Error occurred while retrieving node: {}", err);
                        return Err(GraphError::NodeNameNotFoundError(format!("Error occurred while retrieving node: {}", err)));
                    }
                };
                if let Some(node) = node_option {
                    nodes.push(node);
                } else {
                    return Err(GraphError::NodeNameNotFoundError("Node option is none".to_string()));
                }
            }

            Ok(())
        })?;

        println!("Ego selected count: {}", nodes.len());

        let ego_node = nodes
            .choose(&mut rand::thread_rng())
            .ok_or_else(|| GraphError::NodeSelectionError("Error occurred while choosing node".to_string()))?.clone();

        Ok(ego_node)
    }

    fn calculate_and_display_ratings(ego_node: &str) -> Result<(), GraphError> {
        // Calculate the ratings and store them in a vector
        println!("Calculating ratings for ego node: {}", ego_node);
        let ratings = crate::calculate_ratings(ego_node, 10000, Some(50));

        // Iterate over the ratings and print each one
        for crate::NodeRating { node, rating } in ratings {
            println!("Node: {}, Rating: {}", node, rating);
        }

        Ok(())
    }

    fn fetch_ratings_via_select(ego_node: &str) -> Result<(), GraphError> {
        // Prepare the select query
        let select_query = format!("SELECT calculate_ratings('{}', 10000, 50);", ego_node);

        // Connect to the database and execute the query
        Spi::connect(|client| {
            let result_set = client.select(&select_query, None, None)?;

            // Iterate over the results and print each one
            for row in result_set {
                // let node: Option<String> = row.get(1)?;
                // let rating: Option<f64> = row.get(2)?;
                let node_ratings: Option<Vec<NodeRating>> = row.get(1)?;

                if let Some(ratings) = node_ratings {
                    for rating in ratings {
                        println!("Node rating: {:?}", rating);
                    }
                }
            }

            Ok(())
        })
    }

    // #[pg_test]
    fn test_ratings_calculation() -> Result<(), GraphError> {
        wait_until_graph_size(45);
        println!("Graph size reached.");

        let get_unique_source_values = get_unique_source_values()?;
        println!("Unique source values: {:?}", get_unique_source_values);

        let ego_node = select_ego_node()?;
        println!("Ego node selected: {}", ego_node);
        calculate_and_display_ratings(&ego_node)?;
        println!("Test ratings_calculation passed.");
        Ok(())
    }

    // #[pg_test]
    // fn test_select_graph() {
    //     let select_query = "SELECT source, destination, weight FROM graph WHERE source = 'node1' AND destination = 'node2';";
    //
    //     let result = Spi::connect(|client| {
    //         let mut cursor = client.select(select_query, None, None).unwrap();
    //         let row = cursor.next().unwrap().unwrap();
    //         let source: Option<String> = row.get("source");
    //         let destination: Option<String> = row.get("destination");
    //         let weight: Option<f64> = row.get("weight");
    //
    //         (source, destination, weight)
    //     }).unwrap();
    //
    //     assert_eq!(result, (Some("node1".to_string()), Some("node2".to_string()), Some(10.0)));
    // }
}

/// This module is required by `cargo pgx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}

// let stmt = GraphSingleton::prepare_insert_statement(&mut client)?;

// // Convert String to Datum
// let source_datum = node_name1.into_datum();
// let destination_datum = node_name2.into_datum();
// let weight_datum = weight.into_datum();
//
// // Execute the insert statement with the parameters
// let params = Some(vec![source_datum, destination_datum, weight_datum]);
// client.update(stmt, None, params)?;