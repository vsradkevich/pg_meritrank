// Standard library imports
use std::collections::HashMap;

// Current crate (`crate::`) imports
pub use crate::error::GraphManipulationError;
use crate::graph::{GraphSingleton, GRAPH};
use crate::lib_graph::NodeId;
#[allow(unused_imports)]
use crate::logger::Logger;
use crate::sql::*;

// Random number generation
use rand::Rng;

// The postgres external function to create a graph table.
#[pg_extern]
fn create_graph_table() -> Result<(), GraphManipulationError> {
    GraphSingleton::create_graph_table()
}

// Library for PostgreSQL extensions
#[allow(unused_imports)]
use pgx::{
    // Import specific types for PostgreSQL
    pg_sys::{BuiltinOid, Datum, Oid},
    // Import pgx prelude
    prelude::*,
    // Import specific types for SPI
    spi::{Error, OwnedPreparedStatement, SpiClient, SpiTupleTable},
};

impl GraphSingleton {
    // Edge-related methods

    pub fn add_edge(
        &mut self,
        source: NodeId,
        target: NodeId,
        weight: f64,
    ) -> Result<(), GraphManipulationError> {
        match self.borrow_graph_mut().add_edge(source, target, weight) {
            Ok(_) => {
                Spi::connect(|mut client| {
                    let edge = (source, target, weight);
                    self.insert_edge_into_graph(&edge, &mut client)?;
                    Ok::<(), GraphManipulationError>(())
                })?;
                Ok(())
            }
            Err(_err) => Err(GraphManipulationError::EdgeCreationFailure(
                "Error adding edge".to_string(),
            )),
        }
    }

    // Database-related methods

    /// Creates the graph table in the database if it doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns `GraphManipulationError::TableCreationError` if an error occurs while creating the table.
    pub fn create_graph_table() -> Result<(), GraphManipulationError> {
        Spi::run(CREATE_TABLE).map_err(|err| {
            println!("Error creating table: {}", err);
            GraphManipulationError::TableCreationFailure("Error creating table".to_string())
        })
    }

    /// Creates the graph table in the database if it does not exist
    fn create_graph_table_if_not_exists(
        &self,
        client: &mut SpiClient,
    ) -> Result<(), GraphManipulationError> {
        client.update(CREATE_TABLE, None, None).map_err(|_| {
            GraphManipulationError::TableCreationFailure("Error creating table".to_string())
        })?;
        Ok(())
    }

    /// Checks if records exist in the graph table.
    ///
    /// # Errors
    ///
    /// Returns `GraphManipulationError::SpiError` if an error occurs while accessing the database.
    pub fn records_exist() -> Result<bool, GraphManipulationError> {
        match Spi::get_one::<bool>(SELECT_EXISTS) {
            Ok(Some(true)) => Ok(true),
            Ok(_) => Ok(false),
            Err(err) => Err(GraphManipulationError::SpiFailure(err)),
        }
    }

    /// Begins a database transaction
    fn begin_transaction(&self, client: &mut SpiClient) -> Result<(), GraphManipulationError> {
        client
            .update(BEGIN, None, None)
            .map_err(|e| GraphManipulationError::TransactionInitiationFailure(e.to_string()))?;
        Ok(())
    }

    fn insert_into_graph(
        edge: &(NodeId, NodeId, f64),
        client: &mut SpiClient,
    ) -> Result<(), GraphManipulationError> {
        // Obtain immutable access to the GRAPH singleton
        let graph = GRAPH.lock().unwrap();

        // Insert the edge into the graph
        graph.insert_edge_into_graph(edge, client)
        // Ok(())
    }

    /// Prepares an SQL insert statement
    fn prepare_insert_statement(
        client: &mut SpiClient,
    ) -> Result<OwnedPreparedStatement, GraphManipulationError> {
        let param_types = Some(vec![
            BuiltinOid::TEXTOID.into(),
            BuiltinOid::TEXTOID.into(),
            BuiltinOid::FLOAT8OID.into(),
        ]);

        client
            .prepare(INSERT_SQL, param_types)
            .map(|stmt| stmt.keep())
            .map_err(|_| {
                GraphManipulationError::StatementPreparationFailure(
                    "Error preparing insert statement".to_string(),
                )
            })
    }

    /// Inserts an edge into the graph database
    fn insert_edge_into_graph(
        &self,
        edge: &(NodeId, NodeId, f64),
        client: &mut SpiClient,
    ) -> Result<(), GraphManipulationError> {
        let (source, destination, weight) = edge;
        let node_ids: HashMap<NodeId, String> = self
            .borrow_node_names()
            .iter()
            .map(|(name, id)| (*id, name.clone()))
            .collect();

        // Get the node names from the node ids
        let source_name = node_ids
            .get(source)
            .ok_or_else(|| {
                GraphManipulationError::NodeNotFound("Source node not found".to_string())
            })?
            .clone();
        let destination_name = node_ids
            .get(destination)
            .ok_or_else(|| {
                GraphManipulationError::NodeNotFound("Destination node not found".to_string())
            })?
            .clone();

        // Convert String to Datum
        let source_datum = source_name.into_datum();
        let destination_datum = destination_name.into_datum();
        let weight_datum = weight.into_datum();

        // Execute the insert statement
        let params = Some(vec![source_datum, destination_datum, weight_datum]);

        let stmt: OwnedPreparedStatement = Self::prepare_insert_statement(client)?;

        client.update(stmt, None, params).map_err(|_| {
            GraphManipulationError::EdgeCreationFailure(
                "Error inserting edge into graph".to_string(),
            )
        })?;
        Ok(())
    }

    /// Generates a graph with the given number of nodes and probability coefficients
    ///
    /// # Parameters
    ///
    /// - `num_nodes`: The number of nodes to generate
    /// - `border_probability`: The probability of creating an edge between nodes
    pub fn generate_graph(
        &mut self,
        num_nodes: usize,
        border_probability: f64,
    ) -> Result<(), GraphManipulationError> {
        let mut rng = rand::thread_rng();
        let node_names: Vec<String> = self.generate_node_names(num_nodes);

        let start = std::time::Instant::now();

        for i in 0..num_nodes {
            let source = self.get_node_id(&node_names[i])?;
            for j in 0..num_nodes {
                if i != j && rng.gen::<f64>() < border_probability {
                    let target = self.get_node_id(&node_names[j])?;
                    let weight = rng.gen_range(0.0..=1.0);
                    println!(
                        "Adding edge from {} to {} with weight {}",
                        node_names[i], node_names[j], weight
                    );
                    self.add_edge(source, target, weight)?;
                }
            }
        }

        let stop = std::time::Instant::now();
        println!("Time to generate graph: {:?}", stop.duration_since(start));
        println!(
            "Graph has {} nodes and {} edges",
            self.borrow_graph().node_count(),
            self.borrow_graph().get_edges().len()
        );

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
    pub fn write_graph_to_database(&self) -> Result<(), GraphManipulationError> {
        Spi::connect(|mut client| {
            self.create_graph_table_if_not_exists(&mut client)?;
            println!("Created graph table");
            self.begin_transaction(&mut client)?;
            println!("Began transaction");

            for edge in self.borrow_graph().get_edges() {
                self.insert_edge_into_graph(&edge, &mut client)?;
            }

            self.commit_transaction(&mut client)?;
            Ok(())
        })
    }

    /// Commits a database transaction
    fn commit_transaction(&self, client: &mut SpiClient) -> Result<(), GraphManipulationError> {
        client.update(COMMIT, None, None).map_err(|_| {
            GraphManipulationError::TableCreationFailure("Error committing transaction".to_string())
        })?;
        Ok(())
    }
}

#[allow(dead_code)]
// Function to test insert nodes and an edge into the graph.
pub fn insert_into_graph(
    node_name1: &str,
    node_name2: &str,
    weight: f64,
) -> Result<(), GraphManipulationError> {
    Spi::connect(|mut client| {
        let node1 = GraphSingleton::add_node(node_name1)?;
        let node2 = GraphSingleton::add_node(node_name2)?;
        let edge = (node1, node2, weight);

        println!("source: {}, target: {}", node1, node2);
        println!("edge: {:?}", edge);

        // Insert the edge into the graph
        GraphSingleton::insert_into_graph(&edge, &mut client).map_err(|_| {
            GraphManipulationError::EdgeCreationFailure(
                "Error inserting edge into graph".to_string(),
            )
        })?;

        println!("Inserted edge ({}, {}, {})", node1, node2, weight);
        Ok(())
    })
}
