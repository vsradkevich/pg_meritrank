// use pgx::*;

use crate::graph::{GraphManipulationError, GraphSingleton, GRAPH};

#[allow(dead_code)]
impl GraphSingleton {
    /// Function to initialize the GRAPH singleton
    pub fn init_graph() -> Result<(), GraphManipulationError> {
        // Create the graph table if it doesn't exist
        GraphSingleton::create_graph_table().map_err(|e| {
            println!("Error creating table: {}", e);
            GraphManipulationError::TableCreationFailure("Error creating table".to_string())
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
                            println!(
                                "Source: {:?}, Destination: {:?}, Weight: {}",
                                source, destination, weight
                            );

                            // Add the edge to the graph
                            graph.add_edge(source, destination, weight)?;
                        }
                        Ok(())
                    }
                    Err(e) => {
                        println!("Error fetching records: {}", e);
                        Err(GraphManipulationError::FetchRecordsFailure(
                            "Error fetching records".to_string(),
                        ))
                    }
                }
            }
            Ok(false) => {
                println!("No records found. Creating records.");

                // Generate a graph
                graph.generate_graph(42, 0.13).map_err(|e| {
                    println!("Error generating graph: {:?}", e);
                    GraphManipulationError::GraphGenerationFailure(
                        "Error generating graph".to_string(),
                    )
                })?;

                println!("Graph generated successfully.");

                // Write the graph to the database
                graph.write_graph_to_database().map_err(|e| {
                    println!("Error writing graph to database: {}", e);
                    GraphManipulationError::WeightExtractionFailure(
                        "Error writing graph to database".to_string(),
                    )
                })?;

                println!("Graph written to database successfully.");
                Ok(())
            }
            Err(e) => {
                println!("Error checking records: {}", e);
                Err(e)
            }
        }
    }
}
