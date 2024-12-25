// For server testing -> test if it works

use std::net::TcpStream;
use std::io::{Read,Write};
fn main() {
    // POST's 
    // Explaination: POST 10 1 1 2 3 4 5, means: Request_type is POST, uses version 10, type of
    // POST request equals 1, and rest
    // are arguments.
    send_request("POST 10 1 password=test 1 2 3 4 5 6 ");
    // Teacher
    send_request("POST 10 2 password=test 1 Teacher Hello ");
    // 0800 and 0900 are: 8:00 and 9:00
    send_request("POST 10 3 password=test 1 1500 1600 ");
    // USE _ instead of ' '
    send_request("POST 10 4 password=test 1 Edukacja_dla_bezpieczenstwa ");
    send_request("POST 10 5 password=test 1 Polish_room ");
    send_request("POST 10 6 password=test 1 5 1 1 ");
    send_request("POST 10 7 password=test 1 Klasa_8 ");
    // GET's
    // This request is mostly automated, only requiring teacher id
    send_request("GET 10 1 1 ");
    // This request doesn' require any argument - it is automated
    send_request("GET 10 2 ");
    // This request is also partially automated, only requiring teacher id
    send_request("GET 10 3 1 ");
    // Same
    send_request("GET 10 4 1 ");
    // Same
    send_request("GET 10 5 1 ");
    // This one is fully automated
    send_request("GET 10 6 ");
    send_request("GET 10 7 1 ");
    send_request("GET 10 8 1 ");
    send_request("GET 10 9 1 ");
}

fn send_request(request: &str){
    println!("---");
    let mut stream : TcpStream = TcpStream::connect("127.0.0.1:8888").unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = [0u8; 1024];
    stream.read(&mut response).unwrap();
    println!("{}", request);
    println!("{}", String::from_utf8_lossy(&response));
    println!("SUCCESS");
}
