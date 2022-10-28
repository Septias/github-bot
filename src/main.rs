use std::sync::{Arc, Mutex};

use web_server::{RequestHandler, Response};

struct Server {
    storage: String,
}

impl Server {
    fn new() -> Self {
        Self {
            storage: String::new(),
        }
    }
    fn handler(
        &mut self,
        request: web_server::Request,
        mut response: web_server::Response,
    ) -> Response {
        self.storage += &request.get_body();
        self.storage.clone().into()
    }
}

fn main() {
    let server = Arc::new(Mutex::new(Server::new()));
    web_server::new()
        .get(
            "/",
            Box::new(move |a, b| server.lock().unwrap().handler(a, b)),
        )
        .launch(8080);
}
