use threadpool::ThreadPool;

use std::net::TcpListener;

const NUM_THREADS: usize = 4;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:3000").unwrap();
    let pool = ThreadPool::new(NUM_THREADS);
    for stream in listener.incoming() {
        pool.execute(move || {
            olmmcc::handle_connection(stream.unwrap());
        });
    }
}
