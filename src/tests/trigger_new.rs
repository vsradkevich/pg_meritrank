//noinspection DuplicatedCode
#[cfg(any(test, feature = "pg_test"))]
#[crate::pg_schema]
mod tests {
    // #[allow(unused_imports)]
    // use crate::edge::GraphEdge;
    use crate::graph::{
        meritrank_add,
        meritrank_calculate,
        meritrank_delete, // meritrank_update_graph,
    };
    use dotenv::dotenv;
    use pgx::*;
    use std::env;
    use std::fs;

    /// Reads a SQL script from a file and executes it in the current Postgres session.
    ///
    /// The path to the SQL file is expected to be stored in the environment variable `env_var`.
    ///
    /// # Panics
    /// Panics if the environment variable is not set, the file cannot be read or the SQL script fails to execute.
    fn execute_sql_from_file(env_var: &str) {
        let sql_file = env::var(env_var).expect(&format!("Could not read {}", env_var));

        let sql_script =
            fs::read_to_string(sql_file).expect(&format!("Could not read SQL file {}", env_var));

        let result = Spi::run(sql_script.as_ref());
        assert!(result.is_ok());
    }

    // Test the meritrank_add function
    // #[pg_test]
    fn test_meritrank_add() {
        println!("Test meritrank_add started.");
        let result = meritrank_add("node1", "node2", 42.0);
        assert!(result.is_ok());
        println!("Test meritrank_add passed.");
    }

    // Test the meritrank_calculate function
    // #[pg_test]
    fn test_meritrank_calculate() {
        println!("Test meritrank_calculate started.");
        let result = meritrank_calculate("node1", "node2", 100);
        assert!(result.is_ok());
        let calculated_rank = result.unwrap();
        println!("Calculated rank for node1 -> node2: {}", calculated_rank);
        println!("Test meritrank_calculate passed.");
    }

    // Test the meritrank_delete function
    // #[pg_test]
    fn test_meritrank_delete() {
        println!("Test meritrank_delete started.");
        let result = meritrank_delete("node1", "node2");
        assert!(result.is_ok());
        println!("Test meritrank_delete passed.");
    }

    // Test the meritrank_update_graph function
    // #[pg_test]
    // fn test_meritrank_update_graph() {
    //     println!("Test meritrank_update_graph started.");
    //     let _edges = vec![
    //         GraphEdge::new("node1".to_string(), "node2".to_string(), 1.0),
    //         GraphEdge::new("node2".to_string(), "node3".to_string(), 2.0),
    //     ];
    //     // let result = meritrank_update_graph(edges);
    //     // assert!(result.is_ok());
    //     println!("Test meritrank_update_graph passed.");
    // }

    fn test_trigger_functionality() {
        println!("Test trigger functionality started.");
        let result = meritrank_add("node1", "node2", 42.0);
        assert!(result.is_ok());
        let result = meritrank_calculate("node1", "node2", 100);
        assert!(result.is_ok());
        let result = meritrank_delete("node1", "node2");
        assert!(result.is_ok());
        println!("Test trigger functionality passed.");
    }

    // Main test function that calls all the other tests in the correct order
    #[pg_test]
    fn run_all_tests() {
        dotenv().ok();

        // Setup
        execute_sql_from_file("INITIAL_STRUCTURE_SQL");
        execute_sql_from_file("IMPLEMENT_TRIGGERS_SQL");

        // Meritrank tests
        test_meritrank_add();
        test_meritrank_calculate();
        test_meritrank_delete();
        // test_meritrank_update_graph();

        // Run Meritrank tests in SQL
        let _add = Spi::run("SELECT meritrank_add('node1', 'node2', 42);");
        let _calculate = Spi::run("SELECT meritrank_calculate('node1', 'node2', 100);");
        let _delete = Spi::run("SELECT meritrank_delete('node1', 'node2');");
        let _update_graph = Spi::run("CALL update_graph_procedure();");

        // Test Triggers
        test_triggers();

        // Change the execution time
        let _update_graph = Spi::run("CALL update_graph_procedure();");
    }

    use rand::Rng;
    use std::collections::HashMap;

    // Definition of the main types
    type TypeName = String;
    type NodeId = usize;
    type NodeName = String;
    type Weight = i32;

    fn get_triggers_from_file(path: &str) -> HashMap<String, Vec<String>> {
        let content = fs::read_to_string(path).unwrap();
        let re =
            Regex::new(r"(?m)^CREATE\s+TRIGGER\s+(\w+)\s+AFTER\s+(\w+)\s+ON\s+(\w+\.\w+)\s*.*?;$")
                .unwrap();
        let mut triggers = HashMap::new();

        for cap in re.captures_iter(&content) {
            let trigger_type = cap[2].to_lowercase();
            let table_name = cap[3].to_string();

            if !triggers.contains_key(&trigger_type) {
                triggers.insert(trigger_type.clone(), Vec::new());
            }

            triggers
                .get_mut(&trigger_type)
                .unwrap()
                .push(table_name.clone());

            println!("Trigger type: {}, table name: {}", trigger_type, table_name);
        }

        triggers
    }

    // Graph structure containing data and a set of names for each node type
    struct Graph {
        data: HashMap<NodeName, HashMap<NodeName, Weight>>,
        node_type_names: HashMap<TypeName, Vec<NodeName>>,
    }

    impl Graph {
        // Initialize an empty graph
        fn new() -> Self {
            Graph {
                data: HashMap::new(),
                node_type_names: HashMap::new(),
            }
        }

        // Add a node type
        fn add_node_type(&mut self, table: &TypeName) {
            if !self.node_type_names.contains_key(table) {
                self.node_type_names.insert(table.clone(), Vec::new());
            }
        }

        // Add a node
        fn add_node(
            &mut self,
            table: &TypeName,
            node_id: &NodeId,
        ) -> Result<NodeName, &'static str> {
            let node_name = format!("{}_{}", table, node_id);
            if self.data.contains_key(&node_name) {
                Err("Node already exists")
            } else {
                self.data.insert(node_name.clone(), HashMap::new());
                self.node_type_names
                    .entry(table.clone())
                    .or_insert_with(Vec::new)
                    .push(node_name.clone());
                Ok(node_name)
            }
        }

        // Delete a node
        fn delete_node(&mut self, table: &TypeName, node_id: &NodeId) -> Result<(), &'static str> {
            let node_name = format!("{}_{}", table, node_id);
            if self.data.remove(&node_name).is_none() {
                Err("Node not found")
            } else {
                if let Some(nodes) = self.node_type_names.get_mut(table) {
                    nodes.retain(|x| x != &node_name);
                }

                // Delete all edges with this node
                for edges in self.data.values_mut() {
                    edges.remove(&node_name);
                }
                Ok(())
            }
        }

        // Add an edge
        fn add_edge(
            &mut self,
            node1: &NodeName,
            node2: &NodeName,
            weight: Weight,
        ) -> Result<(), &'static str> {
            if let Some(node) = self.data.get_mut(node1) {
                node.insert(node2.clone(), weight);
                Ok(())
            } else {
                Err("First node not found")
            }
        }

        // Update an edge
        fn update_edge(
            &mut self,
            node1: &NodeName,
            node2: &NodeName,
            new_weight: Weight,
        ) -> Result<(), &'static str> {
            if let Some(edges) = self.data.get_mut(node1) {
                if edges.insert(node2.clone(), new_weight).is_some() {
                    Ok(())
                } else {
                    Err("Edge not found")
                }
            } else {
                Err("First node not found")
            }
        }

        // Delete an edge
        fn delete_edge(&mut self, node1: &NodeName, node2: &NodeName) -> Result<(), &'static str> {
            if let Some(edges) = self.data.get_mut(node1) {
                if edges.remove(node2).is_some() {
                    Ok(())
                } else {
                    Err("Edge not found")
                }
            } else {
                Err("First node not found")
            }
        }

        // Get a list of all types
        fn get_node_types(&self) -> Vec<TypeName> {
            self.node_type_names.keys().cloned().collect()
        }

        // Get a list of all nodes of a certain type
        fn get_nodes_by_type(&self, type_name: &TypeName) -> Option<Vec<NodeName>> {
            self.node_type_names.get(type_name).cloned()
        }

        // Get the type of a node
        fn get_type_of_node(&self, node: &NodeName) -> Option<&TypeName> {
            self.node_type_names
                .iter()
                .find(|(_, nodes)| nodes.contains(node))
                .map(|(type_name, _)| type_name)
        }

        // Get a list of all edges for a given node
        fn get_edges(&self, node: &NodeName) -> Option<&HashMap<NodeName, i32>> {
            self.data.get(node)
        }
    }

    enum Event {
        AddNode {
            table: TypeName,
            node_id: NodeId,
        },
        AddEdge {
            table: TypeName,
            node1: NodeName,
            node2: NodeName,
            weight: Weight,
        },
        UpdateEdge {
            table: TypeName,
            node1: NodeName,
            node2: NodeName,
            weight: Weight,
        },
        DeleteNode {
            table: TypeName,
            node_id: NodeId,
        },
        DeleteEdge {
            table: TypeName,
            node1: NodeName,
            node2: NodeName,
        },
    }

    struct DataManager {
        // a graph representing the data structure
        graph: Graph,
        // a list of tables with data
        tables: HashMap<String, Table>,
    }

    struct Table {
        data: HashMap<String, i32>,
    }

    impl Table {
        fn new() -> Self {
            Table {
                data: HashMap::new(),
            }
        }
    }

    use pgx::pg_sys::BuiltinOid;
    use regex::Regex;

    impl DataManager {
        fn new() -> Self {
            dotenv().ok();
            let trigger_file_name = &env::var("IMPLEMENT_TRIGGERS_SQL").unwrap();
            let triggers = get_triggers_from_file(trigger_file_name);

            let mut graph = Graph::new();
            let mut tables = HashMap::new();

            for (_trigger_type, table_list) in triggers {
                for table_name in table_list {
                    // table_name = "public.vote_{%type%}"
                    let re = Regex::new(r"public\.vote_(?P<type>\w+)").unwrap();
                    // parse type from table_name via regex
                    let captures = re.captures(table_name.as_str()).unwrap();
                    let object_type = captures.name("type").unwrap().as_str();
                    tables.insert(object_type.to_string(), Table::new());
                    graph.add_node_type(&object_type.to_string());
                }
            }

            println!("Graph types: {:?}", graph.get_node_types());
            println!("Tables keys: {:?}", tables.keys());

            DataManager {
                graph,
                tables,
                // triggers,
            }
        }

        // Graph generation and adding to data tables
        pub fn generate_graph(&mut self, num_nodes: usize, edge_probability: f32) {
            // Node generation for each type
            for table in self.graph.get_node_types() {
                for node_id in 0..num_nodes {
                    let _node_name = self.graph.add_node(&table, &node_id).unwrap();
                    self.handle_event(Event::AddNode {
                        table: table.clone(),
                        node_id,
                    });
                }
            }

            // Edge generation between nodes
            let subjects = self.graph.get_nodes_by_type(&"user".to_string()).unwrap();

            for subject in &subjects {
                for table in self.graph.get_node_types() {
                    // there may be a condition on the presence of edges between nodes of certain types
                    // if table != "user" {

                    let objects = self.graph.get_nodes_by_type(&table).unwrap();
                    for object in objects {
                        if rand::random::<f32>() < edge_probability {
                            let weight = rand::random::<i32>().abs() % 15 + 1; // Random number from 1 to 14
                            self.handle_event(Event::AddEdge {
                                table: table.clone(),
                                node1: subject.clone(),
                                node2: object.clone(),
                                weight,
                            });
                        }
                    }
                }
            }
        }

        // Generating a random event depending on the current state of the graph
        pub fn generate_event(&self) -> Option<Event> {
            let mut rng = rand::thread_rng();
            let prob = rng.gen_range(0..100);

            // Select a random subject vertex
            let user_type = String::from("user");
            let subject_index =
                rng.gen_range(0..self.graph.get_nodes_by_type(&user_type).unwrap().len());
            let subject = &self.graph.get_nodes_by_type(&user_type).unwrap()[subject_index];

            // Data deletion (if there is something to delete) <20, data update <50, data addition <100
            if prob < 50 {
                // Check if the subject has any edges
                if let Some(edges) = self.graph.data.get(subject) {
                    if !edges.is_empty() {
                        // Select a random edge
                        let edge_index = rng.gen_range(0..edges.len());
                        let edge_node = edges.keys().nth(edge_index).unwrap().clone();

                        // Get object type
                        let object_type = self.graph.get_type_of_node(&edge_node).unwrap();

                        if prob < 20 {
                            return Some(Event::DeleteEdge {
                                table: object_type.clone(),
                                node1: subject.clone(),
                                node2: edge_node,
                            });
                        } else {
                            let weight = rand::random::<i32>().abs() % 15 + 1; // Random number from 1 to 14
                            return Some(Event::UpdateEdge {
                                table: object_type.clone(),
                                node1: subject.clone(),
                                node2: edge_node,
                                weight,
                            });
                        }
                    }
                }
            } else {
                let object_types = self.graph.get_node_types();

                // Adding edges (between existing nodes)
                let object_type_index = rng.gen_range(0..object_types.len());
                let object_type = &object_types[object_type_index];
                let objects = self.graph.get_nodes_by_type(object_type).unwrap();
                if !objects.is_empty() {
                    let object_index = rng.gen_range(0..objects.len());
                    let object = &objects[object_index];

                    // Get object type
                    let object_type = self.graph.get_type_of_node(&object).unwrap();

                    let weight = rand::random::<i32>().abs() % 15 + 1; // Random number from 1 to 14
                    return Some(Event::AddEdge {
                        table: object_type.clone(),
                        node1: subject.clone(),
                        node2: object.clone(),
                        weight,
                    });
                }
            }

            None
        }

        // Event processing
        pub fn handle_event(&mut self, event: Event) {
            match event {
                Event::AddNode { table, node_id } => {
                    // add a node to the graph and update the database
                    let _ = self.graph.add_node(&table, &node_id);

                    // TODO: add code to update the database
                    // Note: in this case there is no data for object and amount, another approach may be required

                    println!("Add node: {} {}", table, node_id);
                }
                Event::AddEdge {
                    table,
                    node1,
                    node2,
                    weight,
                } => {
                    // add an edge to the graph and update the database
                    let _ = self.graph.add_edge(&node1, &node2, weight);

                    Spi::connect(|mut client| {
                        let table_name = format!("public.vote_{}", table);
                        let query = format!(
                            "INSERT INTO {} (subject, object, amount) VALUES ($1, $2, $3)",
                            table_name
                        );

                        let prepared = client
                            .prepare(
                                &query,
                                Some(vec![
                                    PgOid::from(BuiltinOid::VARCHAROID),
                                    PgOid::from(BuiltinOid::VARCHAROID),
                                    PgOid::from(BuiltinOid::INT4OID),
                                ]),
                            )
                            .map(|stmt| stmt.keep())
                            .expect("Failed to prepare INSERT");

                        let params = Some(vec![
                            node1.clone().into_datum(),
                            node2.clone().into_datum(),
                            weight.clone().into_datum(),
                        ]);

                        if client.update(&prepared, None, params).is_err() {
                            println!("Failed to execute INSERT on {}", table);
                        }
                    });

                    println!("Add edge: {} {} {}", table, node1, node2);
                }
                Event::UpdateEdge {
                    table,
                    node1,
                    node2,
                    weight,
                } => {
                    // update an edge in the graph and update the database
                    let _ = self.graph.update_edge(&node1, &node2, weight);

                    Spi::connect(|mut client| {
                        let table_name = format!("public.vote_{}", table);
                        let query = format!(
                            "UPDATE {} SET subject = $1, object = $2, amount = $3 WHERE subject = $4 AND object = $5",
                            table_name
                        );

                        let prepared = client
                            .prepare(
                                &query,
                                Some(vec![
                                    PgOid::from(BuiltinOid::VARCHAROID),
                                    PgOid::from(BuiltinOid::VARCHAROID),
                                    PgOid::from(BuiltinOid::INT4OID),
                                    PgOid::from(BuiltinOid::VARCHAROID),
                                    PgOid::from(BuiltinOid::VARCHAROID),
                                ]),
                            )
                            .map(|stmt| stmt.keep())
                            .expect("Failed to prepare UPDATE");

                        let params = Some(vec![
                            node1.clone().into_datum(),
                            node2.clone().into_datum(),
                            weight.clone().into_datum(),
                            node1.clone().into_datum(),
                            node2.clone().into_datum(),
                        ]);

                        if client.update(&prepared, None, params).is_err() {
                            println!("Failed to execute UPDATE on {}", table);
                        }
                    });

                    println!("Update edge: {} {} {}", table, node1, node2);
                }
                Event::DeleteNode { table, node_id } => {
                    // remove a node from the graph and update the database
                    let _ = self.graph.delete_node(&table, &node_id);

                    // TODO: add code to update the database
                    // Note: in this case there is no data for object and amount, another approach may be required

                    println!("Delete node: {} {}", table, node_id);
                }
                Event::DeleteEdge {
                    table,
                    node1,
                    node2,
                } => {
                    // remove an edge from the graph and update the database
                    let _ = self.graph.delete_edge(&node1, &node2);

                    Spi::connect(|mut client| {
                        let table_name = format!("public.vote_{}", table);
                        let query = format!(
                            "DELETE FROM {} WHERE subject = $1 AND object = $2",
                            table_name
                        );

                        let prepared = client
                            .prepare(
                                &query,
                                Some(vec![
                                    PgOid::from(BuiltinOid::VARCHAROID),
                                    PgOid::from(BuiltinOid::VARCHAROID),
                                ]),
                            )
                            .map(|stmt| stmt.keep())
                            .expect("Failed to prepare DELETE");

                        let params =
                            Some(vec![node1.clone().into_datum(), node2.clone().into_datum()]);

                        if client.update(&prepared, None, params).is_err() {
                            println!("Failed to execute DELETE on {}", table);
                        }
                    });

                    println!("Delete edge: {} {} {}", table, node1, node2);
                }
            }
        }
    }

    fn test_triggers() {
        println!("Test triggers started.");
        let mut data_manager = DataManager::new();
        data_manager.generate_graph(100, 0.3);
        println!("Test triggers passed.");
    }
}
