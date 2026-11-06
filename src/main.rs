mod parser;
mod server;

use server::Backend;
use tower_lsp::{LspService, Server};
use tokio::io::{stdin, stdout};

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::build(|client| Backend::new(client)).finish();
    Server::new(stdin(), stdout(), socket).serve(service).await;
}
