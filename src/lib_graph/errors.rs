/// Errors that can occur in the MeritRank implementation.
#[derive(Debug, Clone)]
pub enum MeritRankError {
    NodeDoesNotExist,
    SelfReferenceNotAllowed,
    RandomChoiceError,
    NoPathExists,
    NodeIdParseError,
    InvalidNode,
}

use std::error::Error;
use std::fmt::{Display, Formatter, Result};

impl Display for MeritRankError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            MeritRankError::NodeDoesNotExist => write!(f, "NodeDoesNotExist"),
            MeritRankError::SelfReferenceNotAllowed => write!(f, "SelfReferenceNotAllowed"),
            MeritRankError::RandomChoiceError => write!(f, "RandomChoiceError"),
            MeritRankError::NoPathExists => write!(f, "NoPathExists"),
            MeritRankError::NodeIdParseError => write!(f, "NodeIdParseError"),
            MeritRankError::InvalidNode => write!(f, "InvalidNode"),
        }
    }
}

impl Error for MeritRankError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MeritRankError::NodeDoesNotExist => None,
            MeritRankError::SelfReferenceNotAllowed => None,
            MeritRankError::RandomChoiceError => None,
            MeritRankError::NoPathExists => None,
            MeritRankError::NodeIdParseError => None,
            MeritRankError::InvalidNode => None,
        }
    }
}
