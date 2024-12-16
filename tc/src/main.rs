use std::net::{TcpStream, TcpListener};
use std::io::{Read,Write};
fn main() {
    let mut stream : TcpStream = TcpStream::connect("127.0.0.1:8000").unwrap();
    let mut input : String = String::from("");
    std::io::stdin().read_line(&mut input).unwrap();
    stream.write_all(input.as_bytes()).unwrap();
    let mut buf = [0u8;1024];
    let length = stream.read(&mut buf).unwrap();
    println!("{}", String::from_utf8_lossy(&buf));
}
