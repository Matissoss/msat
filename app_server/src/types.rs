/// ===========================================
///                 Types.rs
///     Contains types used in main.rs 
/// ===========================================
use serde::{Serialize, Deserialize};
use std::net::IpAddr;

#[derive(Clone,Debug,Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Request{
    GET,
    POST,
    #[default]
    Other
}
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

#[derive(PartialEq,Eq,Serialize,Deserialize,Clone,Debug, Default)]
pub struct Configuration{
    pub password : String,
    pub ip_addr  : Option<IpAddr>,
}
#[allow(unused)]
#[derive(Clone,Copy,Debug,PartialEq, Eq, PartialOrd, Ord)]
pub enum ConnectionError{
    CannotRead,
    WrongVersion,
    RequestParseError,
    NoVersion,
    WrongPassword,
    NoPassword,
    WritingError,
    ResponseError,
    Other
}

#[allow(unused)]
pub enum RequestError{
    LengthError,
    DatabaseError,
    UnknownRequestError,
    NoDataFoundError,
    ParseIntError(String)
}

pub trait SendToClient{
    fn to_response(input: Self) -> String;
}

impl std::fmt::Display for RequestError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self{
            Self::LengthError => {
                writeln!(f, "400: Client provided wrong amount of arguments")
            }
            Self::DatabaseError => {
                writeln!(f, "500: Server couldn't make operation on database")
            }
            Self::NoDataFoundError => {
                writeln!(f, "500: Server couldn't find any data requested by user")
            }
            Self::ParseIntError(s) => {
                writeln!(f, "400: Client provided argument that couldn't be parsed as integer ({})", s)
            }
            Self::UnknownRequestError => {
                writeln!(f, "400: Server doesn't know how to proceed with this request")
            }
        }
    }
}

impl std::fmt::Display for ConnectionError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self{
            Self::Other => {
                writeln!(f, "Other")
            }
            Self::NoVersion => {
                writeln!(f, "400: Version wasn't provided in request.")
            }
            Self::NoPassword => {
                writeln!(f, "403: Password wasn't provided in request.")
            }
            Self::WrongVersion => {
                writeln!(f, "505: Client uses different version than server.")
            }
            Self::CannotRead => {
                writeln!(f, "500: Server was unable to read message from client.")
            }
            Self::RequestParseError => {
                writeln!(f, "400: Server was unable to parse request.")
            }
            Self::WritingError => {
                writeln!(f, "500: Server was unable to send message to client.")
            }
            Self::WrongPassword => {
                writeln!(f, "400: Client provided wrong password.")
            }
            Self::ResponseError => {
                writeln!(f, "400: Server was unable to respond to request.")
            }
        }
    }
}

impl SendToClient for RequestError{
    fn to_response(input: Self) -> String {
        match input{
            Self::DatabaseError => {
                "500 Internal Server Error: Server couldn't communicate with database".to_string()
            }
            Self::UnknownRequestError => {
                "400 Bad Request: Server doesn't implement this request".to_string()
            }
            Self::LengthError => {
                "400 Bad Request: Client provided wrong amount of arguments".to_string()
            }
            Self::NoDataFoundError => {
                "500 Internal Server Error: Server couldn't provide any data".to_string()
            }
            Self::ParseIntError(s) => {
                format!("400 Bad Request: Client provided string: \"{}\" which couldn't be parsed to 8-bit integer", s)
            }
        }
    }
}
impl SendToClient for ConnectionError{
    fn to_response(input: Self) -> String {
        match input{
            Self::ResponseError => {
                "400 Bad Request: Server couldn't provide response to client".to_string()
            }
            Self::Other => {
                "0 Unknown: Other".to_string()
            }
            Self::NoVersion => {
                "400 Bad Request: Client didn't provide version in request".to_string()
            }
            Self::CannotRead => {
                "500 Internal Server Request: Server couldn't read request sent by client".to_string()
            }
            Self::NoPassword => {
                "400 Bad Request: Client didn't provide password in request".to_string()
            }
            Self::WrongVersion => {
                "505 Version not supported: Client provided version that is different from server".to_string()
            }
            Self::WritingError => {
                "500 Internal Server Request: Server couldn't send response to client".to_string()
            }
            Self::WrongPassword => {
                "400 Bad Request: Client provided wrong password in POST request".to_string()
            }
            Self::RequestParseError => {
                "400 Bad Request: Server couldn't parse request sent by client".to_string()
            }
        }
    }
}

#[derive(Clone)]
pub struct ParsedRequest{
    pub request: Request,
    pub content: Vec<String>,
    pub password: Option<String>,
    pub request_number: u8
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
