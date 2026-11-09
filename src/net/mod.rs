//! Network access: dumb fetching, an on-disk cache, and the worker thread
//! that combines them (cache → network → stale cache) and parses the result.

pub mod cache;
pub mod fetch;
pub mod worker;
