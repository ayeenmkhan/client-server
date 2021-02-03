extern crate futures;
extern crate h2;
extern crate http;
extern crate tokio;

use h2::client;
use tokio::task;
use futures::*;
use http::*;

use tokio::net::TcpStream;

pub fn main() {
    let addr = "127.0.0.1:5928".parse().unwrap();

    tokio::run(
        // Establish TCP connection to the server.
        TcpStream::connect(&addr)
            .map_err(|_| {
                panic!("failed to establish TCP connection")
            })
            .and_then(|tcp| client::handshake(tcp))
            .and_then(|(h2, connection)| {
                let connection = connection
                    .map_err(|_| panic!("HTTP/2.0 connection failed"));

                // Spawn a new task to drive the connection state
                task::std::thread::spawn(connection)
                // tokio::spawn(connection);

                // Wait until the `SendRequest` handle has available
                // capacity.
                h2.ready()
            })
            .and_then(|mut h2| {
                // Prepare the HTTP request to send to the server.
                let request = Request::builder()
                    .method(Method::GET)
                    .uri("https://www.example.com/")
                    .body(())
                    .unwrap();

                // Send the request. The second tuple item allows the caller
                // to stream a request body.
                let (response, _) = h2.send_request(request, true).unwrap();

                response.and_then(|response| {
                    let (head, mut body) = response.into_parts();

                    println!("Received response: {:?}", head);

                    // The `release_capacity` handle allows the caller to manage
                    // flow control.
                    //
                    // Whenever data is received, the caller is responsible for
                    // releasing capacity back to the server once it has freed
                    // the data from memory.
                    let mut release_capacity = body.release_capacity().clone();

                    body.for_each(move |chunk| {
                        println!("RX: {:?}", chunk);

                        // Let the server send more data.
                        let _ = release_capacity.release_capacity(chunk.len());

                        Ok(())
                    })
                })
            })
            .map_err(|e| panic!("failed to perform HTTP/2.0 request: {:?}", e))
    )
}