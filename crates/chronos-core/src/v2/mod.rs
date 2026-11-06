//! Pure contracts of Chronos social v2, independent of the v1 engine.
pub mod atlas;
pub mod attributes;
pub mod canonical;
pub mod catalog;
pub mod display_names;
pub mod general;
pub mod ids;
pub mod influence;
pub mod model;
pub mod naming;
pub mod offices;
pub mod population;
pub mod query;
pub mod recruit;
pub mod region;
pub mod social;
#[cfg(test)]
mod social_tests;
pub mod validate;

pub const ALGORITHM_VERSION: &str = "chronos-social-2";
