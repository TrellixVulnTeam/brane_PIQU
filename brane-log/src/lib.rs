#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate log;
#[macro_use]
extern crate juniper;

use scylla::Session;
use tokio::sync::watch::Receiver;
use std::sync::Arc;

pub mod ingestion;
pub mod interface;
pub mod schema;

pub struct Context {
    pub scylla: Arc<Session>,
    pub events_rx: Receiver<schema::Event>,
}

pub use schema::Schema;
