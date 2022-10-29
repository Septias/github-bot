use anyhow::bail;
use json::JsonValue;
use std::{
    string::ParseError,
    sync::{Arc, Mutex},
};
use thiserror::Error;
use web_server::{HttpMethod, Request, Response};

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

#[derive(Error, Debug, PartialEq, Eq)]
enum Error {
    #[error("Unable to parse Request")]
    ParseError,

    #[error("Not Covered")]
    NotCovered,
    /* #[error(transparent)]
    Other(#[from] anyhow::Error), */
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    IssueOpened { user: String },
    IssueClosed { user: String },
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

fn parse_issue(payload: JsonValue) -> Result<Event, Error> {
    println!("{:?}", payload["action"]);
    if let JsonValue::Short(action) = &payload["action"] {
        match action.as_str() {
            "closed" => Ok(Event::IssueClosed {
                user: payload["sender"]["login"].to_string(),
            }),
            "opened" => Ok(Event::IssueOpened {
                user: payload["sender"]["login"].to_string(),
            }),
            _ => Err(Error::NotCovered),
        }
    } else {
        Err(Error::ParseError)
    }
}

// this should never return an error
fn try_parse(req_body: &str) -> Result<JsonValue, Error> {
    json::parse(req_body).or_else(|_| Err(Error::ParseError))
}

fn receive_webhoook(req: Request) -> Result<Event, Error> {
    match req.params.get("X-GitHub-Event") {
        Some(event_type) if event_type == "pull_request" => todo!(),
        Some(event_type) if event_type == "issue" => parse_issue(try_parse(&req.get_body())?),
        Some(_) => Err(Error::NotCovered),
        None => Err(Error::ParseError),
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

#[cfg(test)]
mod tests {
    use super::Event;

    use super::*;

    #[test]
    fn test_issue_closed() {
        let mock = json::parse(include_str!("../mock/issue_close.json")).unwrap();
        assert_eq!(
            parse_issue(mock),
            Ok(Event::IssueClosed {
                user: String::from("Septias")
            })
        );
    }
    #[test]
    fn test_issue_opened() {
        let mock = json::parse(include_str!("../mock/issue_open.json")).unwrap();
        assert_eq!(
            parse_issue(mock),
            Ok(Event::IssueOpened {
                user: String::from("Septias")
            })
        );
    }
}
