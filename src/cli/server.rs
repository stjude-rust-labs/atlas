use std::net::SocketAddr;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct ServerConfig {
    /// The PostgreSQL database connection URL.
    #[clap(long, env)]
    pub database_url: String,

    /// The socket address the server binds to.
    #[clap(long, env = "BIND_ADDRESS", default_value = "127.0.0.1:3000")]
    pub bind: SocketAddr,
}
