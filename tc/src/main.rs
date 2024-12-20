use std::net::TcpStream;
use std::io::{Read,Write};
fn main() {
    let mut stream : TcpStream = TcpStream::connect("127.0.0.1:8888").unwrap();
    let mut input : String = String::from("");
    println!("Enter your request:");
    std::io::stdin().read_line(&mut input).unwrap();
    stream.write_all(input.as_bytes()).unwrap();
    let mut buf = [0u8;1024];
    stream.read(&mut buf).unwrap();
    println!("{}", String::from_utf8_lossy(&buf));
}
