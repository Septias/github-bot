use std::sync::{Arc, Mutex};

use web_server::{HttpMethod, Response};

struct Server {
    storage: String,
}

fn make_str(httpmethod: HttpMethod) -> &'static str {
    match httpmethod {
        HttpMethod::GET => "get",
        HttpMethod::POST => "post",
        _ => "some method",
    }
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
        mut _response: web_server::Response,
    ) -> Response {
        let t = &format!(
            "< {}: {}>\n",
            make_str(request.get_method()),
            request.get_body()
        );
        println!("{}", t);
        self.storage += t;
        self.storage.clone().into()
    }
}

fn main() {
    let server = Arc::new(Mutex::new(Server::new()));
    let cl1 = server.clone();
    web_server::new()
        .get(
            "/",
            Box::new(move |a, b| server.lock().unwrap().handler(a, b)),
        )
        .post("/receive", Box::new(move |a, b| cl1.lock().unwrap().handler(a, b)))
        .launch(8080);
}
