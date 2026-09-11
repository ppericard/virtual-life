//! Start with `demo`, then read `engine::World::step`, then `runner`.
//! Only the optional web adapter imports HTTP/serialization code.

pub mod automaton;
pub mod demo;
pub mod engine;
pub mod experiment;
pub mod launch;
pub mod runner;

#[cfg(feature = "web")]
pub mod web;
