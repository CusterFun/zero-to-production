//! src/main.rs

use std::net::TcpListener;

use zero2prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Panic if we can't read configuratio
    let configuration = get_configuration().expect("Failed to get configuration");
    // We have removed the hard-coded `8000` - it's now coming from our settings!
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let address = TcpListener::bind(address)?;
    run(address)?.await
}
