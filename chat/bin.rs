mod client;
mod core;
mod server;

use core::http::version::Version;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    dbg!(Version::V1_1);
    let mut args = std::env::args();
    match (args.nth(1).as_ref().map(String::as_str), args.next()) {
        (Some("client"), None) => Ok(client::main()?),
        (Some("server"), None) => Ok(server::main()?),
        _ => Err("Usage: a-chat [client|server]".into()),
    }
}
