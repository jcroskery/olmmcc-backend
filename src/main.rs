use threadpool::ThreadPool;

use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

const BUFFER_SIZE: usize = 128;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:3000").unwrap();
    let pool = ThreadPool::new(4);
    for stream in listener.incoming() {
        pool.execute(move || {
            handle_connection(stream.unwrap());
        });
    }
}
fn handle_connection(mut stream: TcpStream) {
    let mut buffer: Vec<char> = Vec::new();
    let mut eof = false;
    while !eof {
        let mut peek_buf = [0; BUFFER_SIZE + 1];
        stream.peek(&mut peek_buf).unwrap();
        if peek_buf[BUFFER_SIZE] == 0 {
            eof = true;
        }

        let mut read_buf = [0; BUFFER_SIZE];
        stream.read(&mut read_buf).unwrap();
        for x in read_buf.into_iter() {
            if *x == 0 {
                eof = true;
                break;
            }
            buffer.push(*x as char);
        }
    }
    let request: String = buffer.iter().collect();
    println!("{}", request);

    let response = "HTTP/1.1 200 OK\r\n\r\nHello, world!";

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
