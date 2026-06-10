mod atom;
mod database;
pub mod error;
//mod old_encoder;
//pub mod old_engine;
mod relation;
mod rule;
pub mod term;

pub mod cause;
mod encoder;
mod engine;
pub mod renderers;
pub mod tuple;

//pub use old_engine::DatalogEngine;
pub use engine::DatalogEngine;
