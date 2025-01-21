use std::{
    io::{Read, Write},
    net::TcpStream
};

fn main() {
    let mut tcp_stream = TcpStream::connect("127.0.0.1:8000").unwrap();
    tcp_stream
        .write_all(b"HTTP/1.1 GET /?msat/50&method=POST+7&password=test&teacher_id=1&teacher_name=pan_tadeusz")
        .unwrap();
    let mut buf = [0u8; 2048];
    let len = tcp_stream.read(&mut buf).unwrap();
    println!("{}", String::from_utf8_lossy(&buf[0..len]));
}
