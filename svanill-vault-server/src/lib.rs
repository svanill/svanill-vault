#[macro_use]
extern crate diesel;
extern crate serde;
#[macro_use]
extern crate serde_derive;
pub mod auth;
pub mod db;
pub mod errors;
pub mod file_server;
pub mod http;
mod rusoto_extra;
pub mod server;

#[cfg(test)]
extern crate ctor;

#[cfg(test)]
extern crate color_backtrace;
