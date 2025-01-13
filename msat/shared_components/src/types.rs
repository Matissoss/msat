/// ===========================================
///                 Types.rs
///     Contains types used in msat
/// ===========================================
use serde::{Serialize, Deserialize};
use std::net::IpAddr;
use std::collections::HashMap;
use crate::backend::RequestType as Request;
use crate::utils::format_lessonh;
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
    pub password           : String,
    pub language           : Language,
    pub http_server        : HttpServerConfig,
    pub application_server : AppServerConfig
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HttpServerConfig{
    pub http_port: u16,
    pub max_connections: u16,
    pub max_timeout_seconds : u16,
    pub tcp_ip  : Option<IpAddr>
}
#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AppServerConfig{
    pub port : u16,
    pub max_connections: u16,
    pub max_timeout_seconds : u16,
    pub tcp_ip : Option<IpAddr>
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
    WrongHeader,
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

#[allow(unused)]
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
            Self::WrongHeader => {
                writeln!(f, "400: Client provided wrong header")
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
                "msat/500-Internal-Server-Error&msg=Server+couldn't+communicate+with+database".to_string()
            }
            Self::UnknownRequestError => {
                "msat/400-Bad-Request&msg=Server+doesn't+implement+this+request".to_string()
            }
            Self::LengthError => {
                "msat/400-Bad-Request&msg=Client+provided+wrong+amount+of+arguments".to_string()
            }
            Self::NoDataFoundError => {
                "msat/500-Internal-Server-Error&msg=Server+couldn't+provide+any+data".to_string()
            }
            Self::ParseIntError(s) => {
                format!("msat/400-Bad-Request&msg=Client+provided+string:+\"{}\"+which+couldn't+be+parsed+to+16-bit+integer", s)
            }
        }
    }
}
impl SendToClient for ConnectionError{
    fn to_response(input: Self) -> String {
        match input{
            Self::ResponseError => {
                "msat/400-Bad-Request&msg=Server+couldn't+provide+response+to+client".to_string()
            }
            Self::Other => {
                "msat/0-Unknown&msg=Other".to_string()
            }
            Self::NoVersion => {
                "msat/400-Bad-Request&msg=Client+didn't+provide+version+in+request".to_string()
            }
            Self::CannotRead => {
                "msat/500-Internal-Server-Request&msg=Server+couldn't+read+request+sent+by+client".to_string()
            }
            Self::NoPassword => {
                "msat/400-Bad-Request&msg=Client+didn't+provide+password+in+request".to_string()
            }
            Self::WrongVersion => {
                "msat/505-Version-not-supported&msg=Client+provided+version+that+is+different+from+server".to_string()
            }
            Self::WritingError => {
                "msat/500-Internal-Server-Request&msg=Server+couldn't+send+response+to+client".to_string()
            }
            Self::WrongPassword => {
                "msat/400-Bad-Request&msg=Client+provided+wrong+password+in+POST+request".to_string()
            }
            Self::RequestParseError => {
                "msat/400-Bad-Request&msg=Server+couldn't+parse+request+sent+by+client".to_string()
            }
            Self::WrongHeader => {
                "msat/400-Bad-Request&msg=Client+provided+wrong+header".to_string()
            }
        }
    }
}

#[derive(Clone)]
pub struct ParsedRequest{
    pub request: Request,
    pub content: HashMap<String, String>,
    pub request_number: u8
}

impl From<(Request, HashMap<String, String>, u8)> for ParsedRequest{
    fn from(value: (Request, HashMap<String, String>, u8)) -> Self {
        let (req, con, req_num) = value;
        return ParsedRequest{
            request: req,
            content: con,
            request_number: req_num
        };
    }
}

pub struct Lesson{
    pub week_day     : u8,
    pub class_id     : u16,
    pub lesson_hour  : u8,
    pub teacher_id   : u16,
    pub subject_id   : u16,
    pub classroom_id : u16
}
pub struct Class{
    pub class_id   : u16,
    pub class_name : String
}
pub struct LessonHour{
    pub lesson_num : u8,
    pub start_time : u16,
    pub end_time   : u16,
}
pub struct Teacher{
    pub teacher_id : u16,
    pub first_name : String,
    pub last_name  : String
}
pub struct Classroom{
    pub classroom_id   : u16,
    pub classroom_name : String
}
pub struct Subject{
    pub subject_id   : u16,
    pub subject_name : String
}
pub struct Duty{
    pub break_num   : u8,
    pub teacher_id  : u16,
    pub break_place : String,
    pub week_day    : u8
}
pub struct BreakHours{
    pub break_num  : u8,
    pub start_time : u16,
    pub end_time   : u16
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Language{
    #[default]
    Unspecified,
    Polish,
    English
}

impl Language{
    pub fn english_or(&self, english: &str, polish: &str) -> String{
        match self{
            Self::Polish => {
                return polish.to_string()
            }
            Self::English => {
                return english.to_string()
            }
            Self::Unspecified => {
                polish.to_string()
            }
        }
    }
}

pub enum Orb<T, Y>{
    Data(T),
    Alt(Y)
}

#[allow(warnings)]
pub trait msatToString{
    fn msat_to_string(&self) -> String;
}

type Hours = (u16, u16);
impl msatToString for Hours{
    fn msat_to_string(&self) -> String {
        let (start_time, end_time) = self;
        let (stime_str , endt_str) = (format_lessonh(*start_time), format_lessonh(*end_time));
        return format!("{} - {}", stime_str, endt_str);
    }
}
