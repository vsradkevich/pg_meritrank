// Current crate (`crate::`) imports
pub use crate::error::GraphManipulationError;
use crate::graph::GraphSingleton;
use crate::lib_graph::NodeId;
#[allow(unused_imports)]
use crate::logger::Logger;
use crate::sql::*;

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

#[allow(dead_code)]
// type alias
type SpiTuple = spi::SpiHeapTupleData;

impl GraphSingleton {
    /// Fetches records from the graph table.
    ///
    /// This method is responsible for fetching records from the graph table.
    /// It establishes a connection to the SPI client, prepares and executes the SELECT query,
    /// and extracts the records from the returned rows.
    pub fn fetch_records(&mut self) -> Result<Vec<(NodeId, NodeId, f64)>, GraphManipulationError> {
        Spi::connect(|client| {
            let prepared_stmt = client.prepare(SELECT_QUERY, None).map_err(|_| {
                GraphManipulationError::StatementPreparationFailure(
                    "Error preparing SELECT statement".to_string(),
                )
            })?;

            let rows = client.select(&prepared_stmt, None, None).map_err(|_| {
                GraphManipulationError::DataExtractionFailure("Error selecting rows".to_string())
            })?;

            // Function to extract records from the rows and return them
            self.extract_records_from_rows(rows)
        })
    }

    /// Extracts records from rows.
    ///
    /// This method iterates through the provided rows, extracts the required data
    /// from each row and stores them in a vector as records.
    fn extract_records_from_rows(
        &mut self,
        rows: SpiTupleTable,
    ) -> Result<Vec<(NodeId, NodeId, f64)>, GraphManipulationError> {
        let mut records = Vec::new();

        for row in rows {
            let (source, destination, weight) = self.extract_data_from_row(&row).map_err(|_| {
                GraphManipulationError::RecordsExtractionFailure(
                    "Error extracting records".to_string(),
                )
            })?;

            records.push((source, destination, weight));
            println!(
                "ROW source: {}, destination: {}, weight: {}",
                source, destination, weight
            )
        }
        println!("extract_records_from_rows worked");
        Ok(records)
    }

    /// Extracts data from a row.
    ///
    /// This method extracts the source, destination, and weight data from a given row.
    fn extract_data_from_row(
        &mut self,
        row: &SpiTuple,
    ) -> Result<(NodeId, NodeId, f64), GraphManipulationError> {
        let source = self.extract_node_id_from_row(&row, 0).map_err(|_| {
            GraphManipulationError::DataExtractionFailure(
                "Failed to extract source value".to_string(),
            )
        })?;

        let destination = self.extract_node_id_from_row(&row, 1).map_err(|_| {
            GraphManipulationError::DataExtractionFailure(
                "Failed to extract destination value".to_string(),
            )
        })?;

        let weight = Self::extract_weight_from_row(&row, 2).map_err(|_| {
            GraphManipulationError::WeightExtractionFailure(
                "Failed to extract weight value".to_string(),
            )
        })?;

        Ok((source, destination, weight))
    }

    /// Extracts a node id from a row.
    ///
    /// This method extracts a node id from a given row using the provided index.
    fn extract_node_id_from_row(
        &mut self,
        row: &SpiTuple,
        index: usize,
    ) -> Result<NodeId, GraphManipulationError> {
        match row.get(index) {
            Ok(Some(value)) => self.get_node_id(value),
            _ => Err(GraphManipulationError::DataExtractionFailure(
                "Failed to extract node id".to_string(),
            )),
        }
    }

    /// Helper function to extract a weight from a row
    fn extract_weight_from_row(
        row: &SpiTuple,
        index: usize,
    ) -> Result<f64, GraphManipulationError> {
        match row.get(index) {
            Ok(Some(value)) => Ok(value),
            _ => Err(GraphManipulationError::WeightExtractionFailure(
                "Failed to extract weight value".to_string(),
            )),
        }
    }
}
