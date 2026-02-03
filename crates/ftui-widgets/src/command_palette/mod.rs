#![forbid(unsafe_code)]

//! Command Palette widget for instant action search.
//!
//! This module provides a fuzzy-search command palette with:
//! - Bayesian match scoring with evidence ledger
//! - Incremental scoring with query-prefix pruning
//! - Word-start, prefix, substring, and fuzzy matching
//! - Conformal rank confidence for tie-break stability
//! - Match position tracking for highlighting
//!
//! # Submodules
//!
//! - [`scorer`]: Bayesian fuzzy matcher with explainable scoring

pub mod scorer;

pub use scorer::{
    BayesianScorer, ConformalRanker, EvidenceKind, EvidenceLedger, IncrementalScorer,
    IncrementalStats, MatchResult, MatchType, RankConfidence, RankStability, RankedItem,
    RankedResults, RankingSummary,
};
