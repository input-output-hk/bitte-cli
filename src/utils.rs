pub mod error;
pub mod nomad;
pub mod terraform;
pub mod types;

use error::Error;

#[derive(Clone)]
pub struct Instance {
    pub public_ip: String,
    pub name: String,
    pub uid: String,
    pub flake_attr: String,
    pub s3_cache: String,
}
