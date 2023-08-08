use pgx::*;

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use crate::graph::{GraphManipulationError, GraphSingleton};
    use crate::pg_test;
    use crate::rating::NodeRating;
    use pgx::prelude::*;
    use pgx::spi::Error::InvalidPosition;
    use rand::prelude::SliceRandom;
    use std::str::FromStr;

    type PgNum = Numeric<10, 5>;

    #[pg_test]
    fn table_creation_and_manipulation() -> Result<(), GraphManipulationError> {
        // Create the table
        Spi::run("SELECT create_graph_table();")?;
        println!("Table created successfully.");

        // Check the table existence
        let result: Option<String> =
            Spi::get_one("SELECT tablename::text FROM pg_tables WHERE tablename = 'graph';")?;
        assert_eq!(result, Some("graph".to_string()));
        println!("Table exists.");

        // Insert a row into the table
        Spi::run("INSERT INTO graph (source, destination, weight) VALUES ('node1', 'node2', 1);")?;
        println!("Record inserted into table.");

        // Check if the row has been inserted
        let source: Option<String> = Spi::get_one(
            "SELECT source::text FROM graph WHERE source = 'node1' AND destination = 'node2';",
        )?;
        let destination: Option<String> = Spi::get_one(
            "SELECT destination::text FROM graph WHERE source = 'node1' AND destination = 'node2';",
        )?;
        let weight: Option<f64> = Spi::get_one(
            "SELECT weight::float8 FROM graph WHERE source = 'node1' AND destination = 'node2';",
        )
        .unwrap();
        assert_eq!(source, Some("node1".to_string()));
        assert_eq!(destination, Some("node2".to_string()));
        assert_eq!(weight, Some(1.0));
        println!("Record correctly retrieved from table.");

        // Delete the row from the table
        Spi::run("DELETE FROM graph WHERE source = 'node1' AND destination = 'node2';")?;
        println!("Record deleted from table.");

        // Check if the row has been deleted
        let deleted_source = Spi::get_one::<String>(
            "SELECT source FROM graph WHERE source = 'node1' AND destination = 'node2';",
        );
        let deleted_destination = Spi::get_one::<String>(
            "SELECT destination::text FROM graph WHERE source = 'node1' AND destination = 'node2';",
        );
        let deleted_weight = Spi::get_one::<f64>(
            "SELECT weight::float8 FROM graph WHERE source = 'node1' AND destination = 'node2';",
        );

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

    #[pg_test]
    fn graph_cast() {
        GraphSingleton::create_graph_table().unwrap();
        let _ = Spi::run(
            "INSERT INTO graph (source, destination, weight) VALUES ('node1', 'node2', 1.23);",
        )
        .unwrap();
        let row: (Option<String>, Option<String>, Option<String>) =
            Spi::get_three("SELECT source, destination, weight::text FROM graph WHERE source = 'node1' AND destination = 'node2';").unwrap();
        assert!(row.0.is_some() && row.1.is_some() && row.2.is_some());
        let (source, destination, weight) = row;
        assert_eq!(source.unwrap(), "node1");
        assert_eq!(destination.unwrap(), "node2");
        assert_eq!(weight.unwrap(), "1.23000");
        println!("Test select_graph_cast passed.");
    }

    // #[pg_test]
    fn graph_uncast() -> Result<(), GraphManipulationError> {
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

    // #[pg_test]
    fn insert_trigger() -> Result<(), GraphManipulationError> {
        // Assuming you have a function called `insert_and_get_graph` that triggers on insert and returns the updated graph
        let _ = Spi::run("SELECT insert_and_trigger('node3', 'node4', 2.34);").unwrap();

        // Check the graph's state
        let select_sql = "SELECT source, destination, weight FROM graph WHERE source = 'node3' AND destination = 'node4';";
        let row: (Option<String>, Option<String>, Option<PgNum>) =
            Spi::get_three(select_sql).unwrap();
        assert!(row.0.is_some() && row.1.is_some() && row.2.is_some());
        let (source, destination, weight) = row;
        assert_eq!(source.unwrap(), "node3");
        assert_eq!(destination.unwrap(), "node4");
        assert_eq!(weight.unwrap(), PgNum::from_str("2.34").unwrap());
        println!("Test trigger_on_insert passed.");
        Ok(())
    }

    #[pg_test]
    fn all() -> Result<(), GraphManipulationError> {
        test_create_table()?;
        test_insert_into_graph()?;
        graph_uncast()?;
        insert_trigger()?;
        println!("Test all passed.");
        Ok(())
    }

    // private helper functions...

    fn test_create_table() -> Result<(), GraphManipulationError> {
        Spi::run("SELECT create_graph_table();")?;
        println!("Test create_table passed.");
        Ok(())
    }

    fn test_insert_into_graph() -> Result<(), GraphManipulationError> {
        Spi::run(
            "INSERT INTO graph (source, destination, weight) VALUES ('node1', 'node2', 1.23);",
        )?;
        println!("Test insert_into_graph passed.");
        Ok(())
    }

    #[pg_test]
    fn test_init_graph() -> Result<(), GraphManipulationError> {
        // let result: Option<String> = Spi::get_one("SELECT init_graph();").unwrap();
        let _result = GraphSingleton::init_graph();
        println!("Graph initialized.");

        let _result = crate::generate::insert_into_graph("node1", "node2", 1.23);
        let _result = crate::generate::insert_into_graph("node1", "node3", 0.89);
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

    fn get_unique_source_values() -> Result<Vec<String>, GraphManipulationError> {
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

            Ok::<(), GraphManipulationError>(())
        })?;

        // println!("Unique source values: {:?}", source_values);

        Ok(source_values)
    }

    fn select_ego_node() -> Result<String, GraphManipulationError> {
        let mut nodes = Vec::new();
        Spi::connect(|client| {
            let sql = "SELECT source FROM graph GROUP BY source HAVING COUNT(*) >= 5;";
            let select_rows = client.select(sql, None, None).unwrap();
            // .map_err(|_| Err(GraphManipulationError::NodeNameNotFoundFailure("Error occurred while selecting rows".to_string())));

            println!("Selecting ego node...");

            for row in select_rows {
                let node_option: Option<String> = match row.get(1) {
                    Ok(node) => node,
                    Err(err) => {
                        println!("Error occurred while retrieving node: {}", err);
                        return Err(GraphManipulationError::NodeNameNotFound(format!(
                            "Error occurred while retrieving node: {}",
                            err
                        )));
                    }
                };
                if let Some(node) = node_option {
                    nodes.push(node);
                } else {
                    return Err(GraphManipulationError::NodeNameNotFound(
                        "Node option is none".to_string(),
                    ));
                }
            }

            Ok(())
        })?;

        println!("Ego selected count: {}", nodes.len());

        let ego_node = nodes
            .choose(&mut rand::thread_rng())
            .ok_or_else(|| {
                GraphManipulationError::NodeSelectionFailure(
                    "Error occurred while choosing node".to_string(),
                )
            })?
            .clone();

        Ok(ego_node)
    }

    fn calculate_and_display_ratings(ego_node: &str) -> Result<(), GraphManipulationError> {
        // Calculate the ratings and store them in a vector
        println!("Calculating ratings for ego node: {}", ego_node);
        let ratings = crate::rating::calculate_ratings(ego_node, 10000, Some(50));

        // Iterate over the ratings and print each one
        for NodeRating { node, rating } in ratings {
            println!("Node: {}, Rating: {}", node, rating);
        }

        Ok(())
    }

    fn fetch_ratings_via_select(ego_node: &str) -> Result<(), GraphManipulationError> {
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
}
