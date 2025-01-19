/// ===========================================
///                 Types.rs
///     Contains types used in msat
/// ===========================================
use serde::{
    Serialize, 
    Deserialize
};
use chrono::Datelike;
use std::net::IpAddr;
use crate::consts::*;
use crate::backend::RequestType as Request;
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
pub struct Config{
    pub password           : String,
    pub language           : Language,
    pub http_server        : HttpServerConfig,
    pub application_server : AppServerConfig
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HttpServerConfig{
    pub port: u16,
    pub ip  : IpAddr,
    pub max_connections: u16,
    pub max_timeout_seconds : u16,
}
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AppServerConfig{
    pub port : u16,
    pub ip : IpAddr,
    pub max_connections: u16,
    pub max_timeout_seconds : u16,
}

impl std::default::Default for HttpServerConfig{
    fn default() -> Self{
        return Self{
            port: 8000,
            ip: *LOCAL_IP,
            max_timeout_seconds: 10,
            max_connections: 100
        }
    }
}
impl std::default::Default for AppServerConfig{
    fn default() -> Self{
        return Self{
            port: 8888,
            max_connections: 100,
            max_timeout_seconds: 10,
            ip: *LOCAL_IP
        }
    }
}

pub enum ServerError{
    ParseIntError{arg: String},
    ParseArgError{args: Vec<String>},
    ArgsMissing  {expected: Vec<String>},
    ReadRequestError,
    UnknownRequest,
    WriteRequestError,
    HTTP{err: HTTPError},
    InvalidRequest(String),
    RequestPasswordError{entered_password: String},
    VersionNotSupported(u16),
    DatabaseError(rusqlite::Error)
}

pub enum HTTPError{
    NotFound,
    NotImplemented,
    InternalServerError,
    URITooLong(String)
}

#[allow(unused)]
pub trait SendToClient{
    fn to_response(&self) -> String;
}

#[allow(warnings)]
impl SendToClient for ServerError{
    fn to_response(&self) -> String {
        match &self{
            Self::UnknownRequest => format!("msat/501-Not-Implemented&error_msg='UnknownRequest'"),
            Self::ReadRequestError => format!("msat/500-Internal-Server-Error&error_msg='ReadError'"),
            Self::WriteRequestError => format!("msat/500-InternalServerError&error_msg='WriteError'"),
            Self::ParseIntError {arg} => format!("msat/400-Bad-Request&error_msg='ParseIntError={}'",arg),
            Self::ParseArgError {args} => format!("msat/400-Bad-Request&error_msg='Args={}'", args.join("+")),
            Self::ArgsMissing {expected} => format!("msat/400-Bad-Request&error_msg='ArgsExpected={}'", expected.join("+")),
            Self::InvalidRequest(request) => format!("msat/400-Bad-Request&error_msg='InvalidRequest='{}''", request),
            Self::HTTP {err} => format!("msat/0-http_error&error={}", err.to_response()),
            Self::RequestPasswordError {entered_password} => format!("msat/400-Bad-Request&error_msg='WrongPassword={}'", entered_password.to_string().to_single('+')),
            Self::VersionNotSupported  (version_entered) => format!("msat/400-Bad-Request&error_msg='NotSupportedVersion={}&supported={}'", version_entered, 
                SUPPORTED_VERSIONS.map(|n| n.to_string()).join("+")),
            Self::DatabaseError(_) => "msat/500-Internal-Server-Error&error_msg='DatabaseError'".to_string()
        }
    }
}

#[allow(warnings)]
impl SendToClient for HTTPError{
    fn to_response(&self) -> String{
        match &self{
            Self::NotFound => format!("404-Not-Found"),
            Self::NotImplemented => format!("501-Not-Implemented"),
            Self::InternalServerError => format!("500-Internal-Server-Error"),
            Self::URITooLong (_) => format!("414-URI-Too-Long")
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Language{
    Polish,
    #[default]
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
        }
    }
}

pub enum Weekday{
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
    Unknown
}

impl Weekday{
    fn now() -> Weekday{
        Weekday::from(chrono::Local::now().weekday() as u8)
    }
    fn to_num(val: Weekday) -> u8{
        return match val{
            Weekday::Monday    => 1,
            Weekday::Tuesday   => 2,
            Weekday::Wednesday => 3,
            Weekday::Thursday  => 4,
            Weekday::Friday    => 5,
            Weekday::Saturday  => 6,
            Weekday::Sunday    => 7,
            Weekday::Unknown   => 0
        };
    }
}

impl From<u8> for Weekday{
    fn from(value : u8) -> Self{
        return match value{
            1 => Self::Monday,
            2 => Self::Tuesday,
            3 => Self::Wednesday,
            4 => Self::Thursday,
            5 => Self::Friday,
            6 => Self::Saturday,
            7 => Self::Sunday,
            _ => Self::Unknown
        };
    } 
}

#[derive(Deserialize, Serialize, Default, Clone, Copy)]
pub struct JoinedHour{
    pub lesson_hour  : Option<u16>,
    pub start_hour   : Option<u8>,
    pub start_minute : Option<u8>,
    pub end_hour     : Option<u8>,
    pub end_minutes  : Option<u8>
}

#[derive(Deserialize, Serialize, Default)]
pub struct JoinedLesson{
    pub weekday       : Option<u8>,
    pub teacher       : Option<String>,
    pub class         : Option<String>,
    pub classroom     : Option<String>,
    pub subject       : Option<String>,
    pub lessonh       : JoinedHour,
    pub semester      : Option<String>,
    pub academic_year : Option<String>
}
#[derive(Deserialize, Serialize, Default)]
pub struct JoinedLessonRaw{
    pub weekday       : Option<u8>,
    pub teacher       : Option<u16>,
    pub class         : Option<u16>,
    pub classroom     : Option<u16>,
    pub subject       : Option<u16>,
    pub lessonh       : Option<u16>,
    pub semester      : Option<u8>,
    pub academic_year : Option<u8>
}
#[derive(Deserialize, Serialize, Default)]
pub struct JoinedDuty{
    pub weekday       : Option<u8>,
    pub semester      : Option<u8>,
    pub academic_year : Option<u8>,
    pub teacher       : Option<String>,
    pub place         : Option<String>,
    pub break_num     : JoinedHour,
}
#[derive(Deserialize, Serialize, Default)]
pub struct JoinedDutyRaw{
    pub weekday       : Option<u8> ,
    pub semester      : Option<u8> ,
    pub academic_year : Option<u8> ,
    pub teacher_id    : Option<u16>,
    pub place_id      : Option<u16>,
    pub break_num     : JoinedHour
}

pub trait MultiwordToSingleword{
    fn to_single(&self, separator: char) -> String;
}
impl MultiwordToSingleword for String{
    fn to_single(&self, separator: char) -> String {
        let words = &self.split_whitespace().collect::<Vec<&str>>();
        let mut to_return = "".to_string();
        for word in words{
            if to_return.as_str() == ""{
                to_return.push_str(&format!("{}", word));
            }
            else{
                to_return.push_str(&format!("{}{}", separator, word));
            }
        }
        return to_return
    }
}
