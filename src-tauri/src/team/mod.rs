//! Team collections: a collection mirrored to a Git repository, one file per request, synced
//! with the Git installed on the device. The database stays the source of truth.

pub(crate) mod access;
mod accounts;
pub(crate) mod apply;
pub mod commands;
pub(crate) mod files;
mod git;
pub(crate) mod groups;
mod history;
mod merge;
mod models;
mod present;
mod repo;
mod report;
pub mod sync;
