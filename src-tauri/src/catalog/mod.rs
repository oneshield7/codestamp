//! Catalog: every collection listed as an API with a health score made of what Codestamp
//! already knows about it (History, Run all results and quality rules), plus owner and tags.

mod build;
pub mod commands;
mod metrics;
mod models;
mod repo;
mod rules;
mod score;
mod validate;
