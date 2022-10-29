#![allow(unused)]
mod handler;
mod parser;
mod server;
use parser::test;
use server::start_server;

fn main() {
    start_server()
}
