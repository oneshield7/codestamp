//! Orchestration: requests run one after another from Start to End, each step passing values on to
//! the next and checking its response, to test a whole sequence (register to checkout).

pub mod commands;
mod evaluate;
mod models;
mod repo;
mod run;
mod validate;
