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

enum Event {
    IssueCreated { user: String },
}

impl Server {
    fn new() -> Self {
        Self {
            storage: String::new(),
        }
    }
    fn get(&mut self, request: web_server::Request) -> Response {
        self.storage.clone().into()
    }

    fn post(&mut self, request: web_server::Request) -> Response {
        let t = &format!(
            "< {}: {}>\n",
            make_str(request.get_method()),
            request.get_body()
        );
        self.storage += t;
        "Ok".into()
    }
}

pub fn start_server() {
    let server = Arc::new(Mutex::new(Server::new()));
    web_server::new()
        .get("/", {
            let server = server.clone();
            Box::new(move |a, _| server.lock().unwrap().get(a))
        })
        .post("/receive", {
            let server = server.clone();
            Box::new(move |a, _| server.lock().unwrap().post(a))
        })
        .launch(8080);
}
