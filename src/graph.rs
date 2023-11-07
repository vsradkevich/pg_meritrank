// Standard library imports
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// External crate imports
use lazy_static::lazy_static;
// use petgraph::visit::Walker;

// Library for PostgreSQL extensions
use pgx::*;
// use pgx::pg_sys::Datum;
// use pgx::prelude::*;

// Current crate (`crate::`) imports
pub use crate::error::GraphManipulationError;
// #[allow(unused_imports)]
// use crate::logger::Logger;

// Current crate (`crate::`) imports
pub use crate::lib_graph::NodeId;
use crate::lib_graph::{MeritRank, MyGraph};

// Singleton instance
lazy_static! {
    pub static ref GRAPH: Arc<Mutex<GraphSingleton>> = Arc::new(Mutex::new(GraphSingleton::new()));
}

#[allow(dead_code)]
// GraphSingleton structure
pub struct GraphSingleton {
    graph: MyGraph,
    node_names: HashMap<String, NodeId>,
}

#[allow(dead_code)]
impl GraphSingleton {
    /// Constructor
    pub fn new() -> GraphSingleton {
        GraphSingleton {
            graph: MyGraph::new(),
            node_names: HashMap::new(),
        }
    }

    /// Get MeritRank object
    pub fn get_rank() -> Result<MeritRank, GraphManipulationError> {
        match GRAPH.lock() {
            Ok(graph) => {
                let merit_rank = MeritRank::new(graph.graph.clone())?;
                Ok(merit_rank)
            }
            Err(e) => Err(GraphManipulationError::MutexLockFailure(format!(
                "Mutex lock error: {}",
                e
            ))),
        }
    }

    /// Borrow Node Names
    pub fn borrow_node_names(&self) -> &HashMap<String, NodeId> {
        &self.node_names
    }

    /// Borrow Graph
    pub fn borrow_graph(&self) -> &MyGraph {
        &self.graph
    }

    /// Borrow Graph Mut
    pub fn borrow_graph_mut(&mut self) -> &mut MyGraph {
        &mut self.graph
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
    /// Returns a `GraphManipulationError::MutexLockFailure()` if the mutex lock fails.
    pub fn add_node(node_name: &str) -> Result<NodeId, GraphManipulationError> {
        match GRAPH.lock() {
            Ok(mut graph) => graph.get_node_id(node_name),
            Err(e) => Err(GraphManipulationError::MutexLockFailure(format!(
                "Mutex lock error: {}",
                e
            ))),
        }
    }

    // This method remains largely the same, it's already well structured
    pub fn get_node_id(&mut self, node_name: &str) -> Result<NodeId, GraphManipulationError> {
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
    pub fn node_name_to_id(node_name: &str) -> Result<NodeId, GraphManipulationError> {
        match GRAPH.lock() {
            Ok(graph) => {
                if let Some(&node_id) = graph.node_names.get(node_name) {
                    Ok(node_id)
                } else {
                    Err(GraphManipulationError::NodeNotFound(format!(
                        "Node not found: {}",
                        node_name
                    )))
                }
            }
            Err(e) => Err(GraphManipulationError::MutexLockFailure(format!(
                "Mutex lock error: {}",
                e
            ))),
        }
    }

    /// Returns the ID of the node with the given name.
    pub fn node_id_to_name(node_id: NodeId) -> Result<String, GraphManipulationError> {
        match GRAPH.lock() {
            Ok(graph) => {
                for (name, id) in graph.node_names.iter() {
                    if *id == node_id {
                        return Ok(name.to_string());
                    }
                }
                Err(GraphManipulationError::NodeNotFound(format!(
                    "Node not found: {}",
                    node_id
                )))
            }
            Err(e) => Err(GraphManipulationError::MutexLockFailure(format!(
                "Mutex lock error: {}",
                e
            ))),
        }
    }

    pub fn clear_graph() -> Result<(), GraphManipulationError> {
        match GRAPH.lock() {
            Ok(mut graph) => {
                graph.graph.clear();
                graph.node_names.clear();
                Ok(())
            }
            Err(e) => Err(GraphManipulationError::MutexLockFailure(format!(
                "Mutex lock error: {}",
                e
            ))),
        }
    }
}

#[pg_extern]
pub fn meritrank_add(
    subject: &str,
    object: &str,
    amount: f64,
) -> Result<(), GraphManipulationError> {
    match GRAPH.lock() {
        Ok(mut graph) => {
            let subject_id = graph.get_node_id(subject)?;
            let object_id = graph.get_node_id(object)?;

            graph
                .borrow_graph_mut()
                .add_edge(subject_id.into(), object_id.into(), amount)?;
            Ok(())
        }
        Err(e) => Err(GraphManipulationError::MutexLockFailure(format!(
            "Mutex lock error: {}",
            e
        ))),
    }
}

#[pg_extern]
pub fn meritrank_calculate(
    subject: &str,
    object: &str,
    iterations: i32,
) -> Result<f64, GraphManipulationError> {
    // Convert the subject string into a NodeId
    let subject_id = GraphSingleton::node_name_to_id(subject)?;

    // Initialize a new graph and merit rank object
    let mut merit_rank = GraphSingleton::get_rank()?;

    // Attempt to calculate merit ranks
    merit_rank.calculate(subject_id, iterations as usize)?;

    // Get ranks and handle potential error
    let peer_scores = merit_rank.get_ranks(subject_id, None)?;

    // Find the rank for our object
    let object_id = GraphSingleton::node_name_to_id(object)?;

    // Convert Vec<(NodeId, f64)> to HashMap<NodeId, f64> if needed, or find directly in the Vec
    let rank = peer_scores.into_iter()
        .find(|(node_id, _)| node_id == &object_id)
        .map(|(_, rank)| rank)
        .ok_or_else(|| GraphManipulationError::NodeNotFound(format!(
            "Rank not found for node: {}",
            object
        )))?;

    Ok(rank)
}

#[pg_extern]
pub fn meritrank_delete(subject: &str, object: &str) -> Result<(), GraphManipulationError> {
    match GRAPH.lock() {
        Ok(mut graph) => {
            let subject_id = graph.get_node_id(subject)?;
            let object_id = graph.get_node_id(object)?;

            graph
                .borrow_graph_mut()
                .remove_edge(subject_id.into(), object_id.into());
            Ok(())
        }
        Err(e) => Err(GraphManipulationError::MutexLockFailure(format!(
            "Mutex lock error: {}",
            e
        ))),
    }
}

#[pg_extern]
pub fn meritrank_clear() -> Result<(), GraphManipulationError> {
    GraphSingleton::clear_graph()
}

// TODO: Finish implementing this

// #[allow(unused_imports)]
// use crate::edge::GraphEdge;

// #[pg_extern]
// pub fn meritrank_update_graph(edges: AnyArray) -> Result<(), GraphManipulationError> {
//     let graph_edges_datum: Datum = edges.datum();
//
//     let array_datum: Array<Datum>;
//
//     // Try to convert Datum to Array<Datum>
//     unsafe {
//         array_datum = match <Array<Datum> as FromDatum>::from_datum(graph_edges_datum, false) {
//             Some(array_datum) => array_datum,
//             None => return Err(GraphManipulationError::DataExtractionFailure(
//                 "Failed to deserialize graph edges".to_string(),
//             )),
//         };
//     }
//
//     println!("Array datum length: {}", array_datum.len());
//
//     // Now, let's iterate through the array and print each element.
//     for (index, datum) in array_datum.iter().enumerate() {
//         match datum {
//             Some(datum) => {
//                 println!("Element {}: {:?}", index, datum.value())
//
//
//                 // // We're expecting Datum to be a pointer to GraphEdge, let's cast it
//                 // let graph_edge_ptr: *mut GraphEdge = datum.cast_mut_ptr();
//                 // if !graph_edge_ptr.is_null() {
//                 //     let graph_edge: *mut GraphEdge = graph_edge_ptr;
//                 //     println!("Element {}: {:?}", index, graph_edge);
//                 //     unsafe {
//                 //         println!("Element source: {:?}", *graph_edge);
//                 //         // println!("Element destination: {:?}", (*graph_edge).destination);
//                 //         // println!("Element weight: {:?}", (*graph_edge).weight);
//                 //     }
//                 // } else {
//                 //     println!("Element {}: Null pointer", index);
//                 // }
//             }
//             None => println!("Element {}: None", index),
//         }
//     }
//
//     Ok(())
// }
