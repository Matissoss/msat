use core::str;
use std::{
    collections::HashMap, fs, io::{
        Read, Write
    }, net::{
        IpAddr, Ipv4Addr, TcpListener, TcpStream
    }
};
use std::sync::Arc;
use tokio::sync::Mutex;
use rusqlite::Connection as SQLite;
use serde::{Serialize,Deserialize};
mod database;
use crate::database as Database;
mod config;
use crate::config as ConfigFile;

#[derive(Clone,Debug,Default, PartialEq, Eq, PartialOrd, Ord)]
enum Request{
    GET,
    POST,
    #[default]
    Other
}

const VERSION : u16 = 10;

impl ToString for Request{
    fn to_string(&self) -> String {
        match &self{
            Self::GET => {
                "GET".to_string()
            },
            Self::POST => {
                "POST".to_string()
            },
            _ => "None".to_string()
        }
    }
}

#[derive(PartialEq, Eq,Serialize,Deserialize,Clone,Debug, Default)]
struct Configuration{
    password : String,
    ip_addr  : Option<IpAddr>,
}
#[derive(Clone,Copy,Debug,PartialEq, Eq, PartialOrd, Ord)]
enum ConnectionError{
    CannotRead,
    WrongVersion,
    RequestParseError,
    NoVersion,
    NonHex,
    WrongPassword,
    NoPassword,
    WritingError,
    ResponseError,
}

impl std::fmt::Display for ConnectionError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self{
            Self::NonHex => {
                writeln!(f, "Wrong Hex value was provided.")
            }
            Self::NoVersion => {
                writeln!(f, "Version wasn't provided in request.")
            }
            Self::NoPassword => {
                writeln!(f, "Password wasn't provided in POST request.")
            }
            Self::WrongVersion => {
                writeln!(f, "Client uses different version than server.")
            }
            Self::CannotRead => {
                writeln!(f, "Server was unable to read message from client.")
            }
            Self::RequestParseError => {
                writeln!(f, "Server was unable to parse request.")
            }
            Self::WritingError => {
                writeln!(f, "Server was unable to send message to client.")
            }
            Self::WrongPassword => {
                writeln!(f, "Client provided wrong password.")
            }
            Self::ResponseError => {
                writeln!(f, "Server was unable to respond to request.")
            }
        }
    }
}

struct ParsedRequest{
    request: Request,
    content: Vec<String>,
    password: Option<String>,
    request_number: u8
}

impl From<(Request, Vec<String>, Option<String>, u8)> for ParsedRequest{
    fn from(value: (Request, Vec<String>, Option<String>, u8)) -> Self {
        let (req, con, pas, req_num) = value;
        return ParsedRequest{
            request: req,
            content: con,
            password: pas,
            request_number: req_num
        };
    }
}

fn from_hex(input: &String) -> Result<u8, ()>{
    if input.len() != 2{
        return Err(());
    }
    else{
        let mut final_out : u8 = 0;
        for (i,c) in input.chars().enumerate(){
            match c.to_ascii_uppercase(){
                'A'|'B'|'C'|'D'|'E'|'F' => {
                    final_out += (c as u8 - 'A' as u8 + 10) * power(2-i as u8, 16);
                }
                '1'|'2'|'3'|'4'|'5'|'6'|'7'|'8'|'9'|'0' => {
                    final_out += (c as u8 - '0' as u8) * power(2-i as u8, 16);
                }
                _ => {
                    return Err(())
                }
            }
        }
        Ok(final_out)
    }
}
fn power(p: u8, n: u8) -> u8{
    let mut fin : u8 = 0;
    for _ in 0..p{
        fin *= p;
    }
    fin
}
#[tokio::main]
async fn main() {
    match Database::init().await{
        Ok(_) => {}
        Err(_) => {
            println!("Error initializing database");
            std::process::exit(-1);
        }
    }
    let mut database : Arc<Mutex<SQLite>> = Arc::new(Mutex::new(match SQLite::open("database.db"){
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error opening database: {}", e);
            std::process::exit(-1);
        }
    }));

    let shared_config = Arc::new(match ConfigFile::get().await{
        Ok(v) => {v},
        Err(_) => {None}
    });
    let ip_address = match &*shared_config{
        Some(v) => {
            match v.ip_addr{
                Some(v1) => Some(v1),
                None => {None}
            }
        }
        None => {None}
    };
    let listener : TcpListener = match TcpListener::bind
        (format!("{}:8000", ip_address.unwrap_or(IpAddr::from(Ipv4Addr::from([127,0,0,1]))))){
            Ok(v) => v,
            Err(e) => {
                if ip_address.is_none(){
                    eprintln!(
                    "data/config.toml doesn't contain any IP Address, like: `127.0.0.1`;
                    Server automatically used this address with port 8000, but it wasn't able to connect : {}", 
                    e);
                    std::process::exit(-1);
                }
                eprintln!("Error connecting to address: `{}` : {}", ip_address.unwrap(), e);
                std::process::exit(-1);
            }
    };
    start_listening(listener, database).await;
    println!("Shutdown?");
    std::process::exit(0);
}

async fn start_listening(listener: TcpListener, db: Arc<Mutex<SQLite>>){
    loop{
        for s in listener.incoming(){
            let stream : Option<TcpStream> = match s{
                Ok(v) => Some(v),
                Err(e) => {
                    eprintln!("Couldn't establish connection with TcpStream : {}", e);
                    None
                }
            };
            if stream.is_some(){
                let copied = Arc::clone(&db);
                tokio::spawn(async move{
                    match handle_connection(stream.unwrap(), copied).await{
                        Ok(_) => {}
                        Err(e) => {print!("{}", e)} 
                    }
                });
            }
            else{
                println!("Error! Stream is None");
            }
        }
    }
}

async fn handle_connection(mut stream: TcpStream, db: Arc<Mutex<SQLite>>) -> Result<(), ConnectionError>{
    let mut data_sent = [0u8; 1024];
    match stream.read(&mut data_sent){
        Ok(_) => {}
        Err(_) => {return Err(ConnectionError::CannotRead);}
    };
    let parsed_req : ParsedRequest = ParsedRequest::from(match parse_request(&String::from_utf8_lossy(&data_sent)).await{
        Ok(v) => v,
        Err(e) => {
            return Err(e);
        }
    });
    match stream.local_addr(){
        Ok(v) => {
            println!("-----\nConnection with: {}, Port:{}\n-----", 
            v.ip(), v.port())
        },
        Err(e) => {
            eprintln!("Error getting local address: {}", e);
        }
    }
    match parsed_req.request{
        Request::GET|Request::POST =>{
            if parsed_req.request == Request::POST && parsed_req.password.is_none(){
                return Err(ConnectionError::NoPassword);
            }
            match get_response(parsed_req).await{
                Ok(v) => {
                    println!("{}", v);
                    match stream.write_all(v.as_bytes()){
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error writing message: {}", e);
                            return Err(ConnectionError::WritingError);
                        }
                    }
                }
                Err(_) => {
                    return Err(ConnectionError::ResponseError);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

async fn get_response(parsed_request: ParsedRequest) -> Result<String, ()>{
    match parsed_request.request{
        Request::GET => {
            Ok(format!("GET: \n{} \n{:?}", String::from_iter(parsed_request.content),parsed_request.request_number))
        }
        Request::POST => {
            Ok(format!("POST: \n{} \n{:?} \n{}", String::from_iter(parsed_request.content), parsed_request.password,parsed_request.request_number))
        }
        _ => {Ok("-1".to_string())}
    }
}

async fn parse_request(input: &str) -> Result<(Request,Vec<String>, Option<String>,u8), ConnectionError> {
    let sliced_input = input.split_whitespace().collect::<Vec<&str>>();
    let (mut request_type,mut content,mut password, mut request_num) : (Request,Vec<String>,Option<String>,u8) = 
    (Request::Other,vec![String::default()],None,0);
    match sliced_input[0]{
        "POST" => {request_type = Request::POST},
        "GET" => {request_type = Request::GET}
        _ => {}
    }
    match sliced_input[1].parse::<u16>(){
        Ok(v) => {
            if v != VERSION{
                return Err(ConnectionError::WrongVersion);
            }
        }
        Err(_) => {
            return Err(ConnectionError::NoVersion);
        }
    }
    match from_hex(&sliced_input[2].to_string()){
        Ok(v) => {
            request_num = v;
        }
        Err(_) => {
            return Err(ConnectionError::NonHex);
        }
    }
    for word in &sliced_input[3..]{
        if word.contains("password="){
            if request_type == Request::POST{
                password = Some(word[8..].to_string());
            } 
        }
        else{
            content.push(word.to_string());
        }
    }
    Ok((request_type,content,password,request_num))
}
