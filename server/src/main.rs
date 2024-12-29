use core::str;
use std::{ 
    io::{
        Read, Write
    }, net::{
        IpAddr, Ipv4Addr, TcpListener, TcpStream
    }
};
use std::sync::Arc;
use tokio::sync::Mutex;
use rusqlite::{Connection as SQLite, OpenFlags};
use serde::{Serialize,Deserialize};
mod database;
use crate::database as Database;
mod config;
use crate::config as ConfigFile;
use chrono::{self, Datelike, Timelike};

#[derive(Clone,Debug,Default, PartialEq, Eq, PartialOrd, Ord)]
enum Request{
    GET,
    POST,
    #[default]
    Other
}

pub const VERSION : u16  = 10;
pub const SUCCESS : &str = "[     OK     ]";
pub const ERROR   : &str = "[     ERR     ]";

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
struct Configuration{
    password : String,
    ip_addr  : Option<IpAddr>,
}
#[allow(unused)]
#[derive(Clone,Copy,Debug,PartialEq, Eq, PartialOrd, Ord)]
enum ConnectionError{
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
enum RequestError{
    LengthError,
    DatabaseError,
    UnknownRequestError,
    NoDataFoundError,
    ParseIntError(String)
}

trait SendToClient{
    fn to_response(input: Self) -> String;
}

impl std::fmt::Display for RequestError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self{
            Self::LengthError => {
                writeln!(f, "{} 400: Client provided wrong amount of arguments", ERROR)
            }
            Self::DatabaseError => {
                writeln!(f, "{} 500: Server couldn't make operation on database", ERROR)
            }
            Self::NoDataFoundError => {
                writeln!(f, "{} 500: Server couldn't find any data requested by user", ERROR)
            }
            Self::ParseIntError(s) => {
                writeln!(f, "{} 400: Client provided argument that couldn't be parsed as integer", ERROR)
            }
            Self::UnknownRequestError => {
                writeln!(f, "{} 400: Server doesn't know how to proceed with this request", ERROR)
            }
        }
    }
}

impl std::fmt::Display for ConnectionError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self{
            Self::Other => {
                writeln!(f, "{} Other", ERROR)
            }
            Self::NoVersion => {
                writeln!(f, "{} 400: Version wasn't provided in request.", ERROR)
            }
            Self::NoPassword => {
                writeln!(f, "{} 403: Password wasn't provided in POST request.", ERROR)
            }
            Self::WrongVersion => {
                writeln!(f, "{} 505: Client uses different version than server.", ERROR)
            }
            Self::CannotRead => {
                writeln!(f, "{} 500: Server was unable to read message from client.", ERROR)
            }
            Self::RequestParseError => {
                writeln!(f, "{} 400: Server was unable to parse request.", ERROR)
            }
            Self::WritingError => {
                writeln!(f, "{} 500: Server was unable to send message to client.", ERROR)
            }
            Self::WrongPassword => {
                writeln!(f, "{} 400: Client provided wrong password.", ERROR)
            }
            Self::ResponseError => {
                writeln!(f, "{} 400: Server was unable to respond to request.", ERROR)
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

const CLEAR : &str = if cfg!(windows){
    "cls"
}
else{
    "clear"
};

#[tokio::main]
async fn main() {
    match std::process::Command::new(CLEAR).status(){
        Ok(_) => {
            println!("Initializing msat version {}...", VERSION);
        }
        Err(_) => {}
    };

    match Database::init().await{
        Ok(_) => {}
        Err(_) => {
            println!("{} Error initializing database", ERROR);
            std::process::exit(-1);
        }
    }
    let database : Arc<Mutex<SQLite>> = Arc::new(Mutex::new(match SQLite::open_with_flags("data/database.db",
                OpenFlags::SQLITE_OPEN_CREATE|OpenFlags::SQLITE_OPEN_READ_WRITE|OpenFlags::SQLITE_OPEN_FULL_MUTEX){
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} Error opening database: {}", ERROR,e);
            std::process::exit(-1);
        }
    }));
    match database.lock().await.execute_batch("PRAGMA journal_mode = WAL;"){
        Ok(_) => {}
        Err(e) =>{
            eprintln!("{} Error executing batch: {}", ERROR,e);
            std::process::exit(-1);
        }
    }
    match database.lock().await.busy_timeout(std::time::Duration::from_secs(4)){
        Ok(_) => {}
        Err(e) => {
            eprintln!("{} Error setting busy_timeout: {}", ERROR,e);
            std::process::exit(-1);
        }
    }

    let shared_config = Arc::new(match ConfigFile::get().await{
        Ok(v) => {v},
        Err(_) => {
            println!("{} Error getting configuration", ERROR); 
            None
        }
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
        (format!("{}:8888", ip_address.unwrap_or(IpAddr::from(Ipv4Addr::from([127,0,0,1]))))){
            Ok(v) => v,
            Err(e) => {
                if ip_address.is_none(){
                    eprintln!(
                    "{} data/config.toml doesn't contain any IP Address, like: `127.0.0.1`;
                    Server automatically used this address with port 8888, but it wasn't able to connect : {}", ERROR,
                    e);
                    std::process::exit(-1);
                }
                eprintln!("{} Error connecting to address: `{}` : {}", ERROR,ip_address.unwrap(), e);
                std::process::exit(-1);
            }
    };
    println!("Listening on {}:8888", ip_address.unwrap_or(IpAddr::from(Ipv4Addr::from([127,0,0,1]))));
    start_listening(listener, database).await;
    println!("Shutdown?");
    std::process::exit(0);
}

async fn start_listening(listener: TcpListener, db: Arc<Mutex<SQLite>>){
    loop{
        for s in listener.incoming(){
            let (mut ip_address, mut port) = (IpAddr::from(Ipv4Addr::new(127, 0, 0, 1)),0);
            let stream : Option<TcpStream> = match s{
                Ok(v) => {
                    match v.local_addr(){
                        Ok(v1) => {
                            (ip_address, port) = (v1.ip(), v1.port());
                        }
                        Err(_) => {}
                    }
                    Some(v)
                },
                Err(e) => {
                    eprintln!("{} Couldn't establish connection with TCPStream : {}", ERROR, e);
                    None
                }
            };
            
            if stream.is_some(){
                let shared_db = Arc::clone(&db);
                tokio::spawn(async move{
                    match handle_connection(stream.unwrap(), shared_db).await{
                        Ok(_) => {
                            println!("{} Successfully handled request from TCPStream {}:{}\n---", SUCCESS, ip_address, port);
                        }
                        Err(e) => {
                            println!("{}\n---", e);
                        }
                    }
                });
            }
            else{
                println!("{} TCPStream is None", ERROR);
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
    let string = match String::from_utf8(data_sent.to_vec()){
        Ok(n) => {
            println!("\"{}\"", n);
            n
        },
        Err(e) => {
            eprintln!("{}", e);
            "".to_string()
        }
    };
    let parsed_req : ParsedRequest = ParsedRequest::from(match parse_request(&string).await{
        Ok(v) => v,
        Err(e) => {
            return Err(e);
        }
    });
    match stream.local_addr(){
        Ok(v) => {
            println!("Connected with {}:{}", v.ip(), v.port());
        },
        Err(e) => {
            eprintln!("{} Error getting local address: {}", ERROR,e);
        }
    }
    match parsed_req.request{
        Request::GET|Request::POST =>{
            if parsed_req.request == Request::POST && parsed_req.password.is_none(){
                return Err(ConnectionError::NoPassword);
            }
            let response : String;
            match get_response(parsed_req.clone(), db).await{
                Ok(v) => {
                    response = v;
                }
                Err(e) => {
                    response = RequestError::to_response(e);
                }
            }
            match stream.write_all(response.as_bytes()){
                Ok(_) => {
                    println!("{} Handled Request\n===", SUCCESS);
                }
                Err(_) => {
                    return Err(ConnectionError::WritingError);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

async fn get_response(parsed_request: ParsedRequest, db: Arc<Mutex<SQLite>>) -> Result<String, RequestError>{
    match parsed_request.request{
        Request::GET => {
            match parsed_request.request_number{
                0 => {
                    return Err(RequestError::UnknownRequestError);
                }
                1 => {
                    // GET Lessons for this Day 
                    let teacher = match parsed_request.content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::ParseIntError(parsed_request.content[0].clone()));
                        } 
                    };
                    let date : u8 = chrono::Local::now().weekday() as u8;
                    let database = db.lock().await;
                    let mut prompt = match database.prepare("SELECT * FROM Lessons WHERE teacher_id = ?1 AND week_day = ?2;"){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::DatabaseError);
                        }
                    };
                    // class_id, classroom_id, subject_id, lesson_number
                    let product_iter = match prompt.query_map([teacher, date], |row|{
                        Ok((
                            quick_match(row.get::<usize,u8>(1)), //class_id,
                            quick_match(row.get::<usize,u8>(5)), // classroom_id,
                            quick_match(row.get::<usize,u8>(4)), // subject_id,
                            quick_match(row.get::<usize,u8>(2)) //lesson_hour
                        ))
                    }){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::DatabaseError);
                        }
                    };
                    for result in product_iter{
                        match result{
                            Ok((class_id, classroom_id, subject_id, lesson_number)) => {
                                if class_id.is_some()&&classroom_id.is_some()
                                    &&subject_id.is_some()&&lesson_number.is_some(){
                                    let (u_class, u_classroom, u_subject, u_lesson) = 
                                        (class_id.unwrap(), classroom_id.unwrap(), 
                                         subject_id.unwrap(), lesson_number.unwrap());
                                        return Ok(format!("200 OK {};{};{};{};", u_class,u_classroom,u_subject,u_lesson));
                                }
                                else{
                                    return Err(RequestError::NoDataFoundError);
                                }
                            }
                            Err(_) => {
                                return Err(RequestError::NoDataFoundError);
                            }
                        }
                    }
                    return Ok("204 No Content".to_string());
                }
                2 => {
                    // GET Hours for this lesson (start time and end time)
                    let current_time_hhmm = format!("{}{}",
                        format_time(chrono::Local::now().hour()), format_time(chrono::Local::now().minute()));
                    let database1 = db.lock().await;
                    let mut query = match database1.prepare("SELECT * FROM LessonHours 
                            WHERE start_time < CAST(?1 AS INTEGER) AND 
                            end_time > CAST(?2 AS INTEGER);")
                    {
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::DatabaseError);
                        } 
                    };
                    let stmt = match query.query_map([&current_time_hhmm, &current_time_hhmm],|row| {
                        Ok((
                                quick_match(row.get::<usize, String>(1)),
                                quick_match(row.get::<usize, String>(2)),
                        ))
                    }){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::DatabaseError);
                        }
                    };
                    let (mut f_end, mut f_start) = ("".to_string(), "".to_string());
                    for result in stmt{
                        match result{
                            Ok((start_time, end_time)) => {
                                if end_time.is_some()&start_time.is_some() == true{
                                    let (u_end, u_start) = (end_time.unwrap(), start_time.unwrap());
                                    (f_end, f_start) = (u_end, u_start);
                                }
                                else{
                                    return Err(RequestError::DatabaseError);
                                }
                            }
                            Err(_) => {
                                return Err(RequestError::NoDataFoundError);
                            }
                        }
                    }
                    if f_end.is_empty()&f_start.is_empty() == false{
                        return Ok(format!("200 OK {};{}", f_start, f_end));
                    }
                    return Ok("204 No Content".to_string());
                }
                3 => {
                    // GET teacher for next duty (name), takes argument of type u8 as input
                    let teacher_id = match parsed_request.content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::ParseIntError(parsed_request.content[0].clone()));
                        }
                    };
                    let current_lesson = match get_lesson_hour(Arc::clone(&db)).await{
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::DatabaseError);
                        }
                    };
                    let query = "SELECT * FROM Duties 
                        WHERE lesson_hour = ?1 AND teacher_id = ?2 AND week_day = ?3;";
                    let database = db.lock().await;
                    let mut stmt = match database.prepare(&query){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::DatabaseError)
                    };
                    let iter = match stmt.query_map([current_lesson, teacher_id, chrono::Local::now().weekday() as u8], |row|{
                        Ok(quick_match(row.get::<usize, u8>(2)))
                    }){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::DatabaseError)
                    };
                    for element in iter{
                        match element{
                            Ok(v) => {
                                match v{
                                    Some(v1) => {
                                        return Ok(format!("200 OK {}", v1));
                                    }
                                    None => {continue;}
                                }
                            }
                            Err(_) => return Err(RequestError::NoDataFoundError)
                        }
                    }
                    return Ok("204 No Content".to_string())
                }
                4 => {
                    // GET teacher for next duty (bool)
                    let teacher_id = match parsed_request.content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    match get_teacher_duty_bool(teacher_id, db).await{
                        Ok(v) =>{
                            return Ok(format!("200 OK {}", v));
                        }
                        Err(_) => return Err(RequestError::NoDataFoundError)
                    }
                }
                5 => {
                    // GET current classroom && class (as String)
                    let teacher_id = match parsed_request.content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    let lesson_hour = match get_lesson_hour(Arc::clone(&db)).await{
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::DatabaseError)
                    };
                    let query ="SELECT * FROM Lessons 
                        WHERE week_day = ?1 AND lesson_hour = ?2 AND teacher_id = ?3;";
                    let database = db.lock().await;
                    let mut stmt = match database.prepare(&query){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::DatabaseError)
                    };
                    let iter = match stmt.query_map([chrono::Local::now().weekday() as u8, lesson_hour, teacher_id], |row| {
                        Ok((
                                // classroom and class
                                quick_match(row.get::<usize, u8>(5)),
                                quick_match(row.get::<usize, u8>(1))
                        ))
                    })
                    {
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::DatabaseError)
                    };
                    for element in iter{
                        match element{
                            Ok((classroom_id, class_id)) => {
                                if classroom_id.is_some()&class_id.is_some() == true{
                                    let (u_classroom, u_class) = (classroom_id.unwrap(), class_id.unwrap());
                                    return Ok(format!("200 OK {};{}", u_class, u_classroom));
                                }
                                else{
                                    return Err(RequestError::NoDataFoundError);
                                }
                            }
                            Err(_) => return Err(RequestError::DatabaseError)
                        }
                    }
                    return Ok("204 No Content".to_string())
                }
                6 => {
                    // GET lesson hour 
                    match get_lesson_hour(db).await{
                        Ok(v) => return Ok(format!("200 OK {}", v)),
                        Err(_) => return Err(RequestError::DatabaseError)
                    };
                }
                7 => {
                    // GET classroom by id
                    let id = match parsed_request.content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    match get_classroom(id, db).await{
                        Ok(v) => return Ok(format!("200 OK {}", v)),
                        Err(_) => return Err(RequestError::DatabaseError)
                    }
                }
                8 => {
                    // GET class by id 
                    let id = match parsed_request.content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    match get_class(id, db).await{
                        Ok(v) => return Ok(format!("200 OK {}", v)),
                        Err(_) => return Err(RequestError::DatabaseError)
                    }
                }
                9 => {
                    let id = match parsed_request.content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    match get_teacher(id, db).await{
                        Ok(v) => return Ok(format!("200 OK {}", v)),
                        Err(_) => return Err(RequestError::DatabaseError)
                    }
                }
                _ => {
                    return Err(RequestError::UnknownRequestError);
                }
            };
        }
        Request::POST => {
            match parsed_request.request_number {
                0 => {
                    return Err(RequestError::UnknownRequestError);
                }
                1 => {
                    // POST Lesson - contains class, classroom, subject, teacher, lesson number
                    let (class_id, classroom_id, subject_id, teacher_id, lesson_number, week_day) :
                        (Option<u8>, Option<u8>, Option<u8>, Option<u8>, Option<u8>, Option<u8>)= 
                    (
                        quick_match(str::parse::<u8>(&parsed_request.content[0].trim())), 
                        quick_match(str::parse::<u8>(&parsed_request.content[1].trim())), 
                        quick_match(str::parse::<u8>(&parsed_request.content[2].trim())), 
                        quick_match(str::parse::<u8>(&parsed_request.content[3].trim())), 
                        quick_match(str::parse::<u8>(&parsed_request.content[4].trim())), 
                        quick_match(str::parse::<u8>(&parsed_request.content[5].trim()))
                    );
                    for index in 0..parsed_request.content.len(){
                        println!("{} : {}", index, parsed_request.content[index]);
                    }
                    if class_id.is_some()&&classroom_id.is_some()&&subject_id.is_some()&&teacher_id.is_some()
                        &&lesson_number.is_some()&&week_day.is_some()
                    {
                        let (u_class, u_classroom, u_subject, u_teacher, u_lesson, u_weekday) = 
                            (class_id.unwrap(), classroom_id.unwrap(), subject_id.unwrap(), 
                             teacher_id.unwrap(), lesson_number.unwrap(), week_day.unwrap());
                        let database = db.lock().await;
                        match database.execute("INSERT INTO Lessons 
                            (week_day, class_id, classroom_id, subject_id, teacher_id, lesson_hour) 
                            VALUES (?1,?2,?3,?4,?5,?6)
                            ON CONFLICT (class_id, lesson_hour, week_day) 
                            DO UPDATE SET classroom_id = excluded.classroom_id, subject_id = excluded.subject_id,
                            teacher_id = excluded.teacher_id;", 
                            [u_weekday, u_class, u_classroom, u_subject, u_teacher, u_lesson])
                        {
                            Ok(_) => {
                                return Ok("201 Created".to_string())
                            }
                            Err(_) => {
                                return Err(RequestError::DatabaseError);
                            }
                        };
                    }
                    else{
                        return Err(RequestError::ParseIntError(parsed_request.content[0].clone()));
                    }
                }
                2 => {
                    // POST Teacher - contains ID and full name
                    let id = match parsed_request.content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => {return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))}
                    };
                    let name = parsed_request.content[1].as_str();
                    let last_name = parsed_request.content[2].as_str();
                    let database = db.lock().await;
                    match database.execute("INSERT INTO Teachers (teacher_id, first_name, last_name) VALUES (?1, ?2, ?3)
                        ON CONFLICT (teacher_id) DO UPDATE SET first_name = excluded.first_name, last_name = excluded.last_name;", 
                        [id.to_string().as_str(), name, last_name]){
                        Ok(_) => {},
                        Err(_) => return Err(RequestError::DatabaseError)
                    };
                    return Ok("201 Created".to_string());
                }
                3 => {
                    // POST Hours - contains start hour, lesson number and end number
                    let content = &parsed_request.content;
                    let lesson_num : u8 = match content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("{} Error parsing: {}", ERROR, e);
                            return Err(RequestError::ParseIntError(content[0].clone()));
                        }
                    };
                    let (s_hour, s_minute) = match format_mmdd(&content[1]){
                        Ok(v) => v,
                        Err(_) => {
                            eprintln!("{} Error formatting to MMDD: {}", ERROR, &content[1]);
                            return Err(RequestError::ParseIntError(content[1].clone()));
                        }
                    };
                    let (e_hour, e_minute) = match format_mmdd(&content[2]){
                        Ok(v) => v,
                        Err(_) => {
                            eprintln!("{} Error formatting to MMDD: {}", ERROR, &content[2]);
                            return Err(RequestError::ParseIntError(content[2].clone()));
                        }
                    };
                        let database = db.lock().await;
                        match database.execute("INSERT INTO LessonHours (
                        lesson_num,start_time, end_time) 
                            VALUES (?1,?2,?3)
                            ON CONFLICT(lesson_num) DO UPDATE SET start_time = excluded.start_time, end_time = excluded.end_time;", 
                            [lesson_num.to_string(), format_two_digit_time(s_hour, s_minute), 
                            format_two_digit_time(e_hour, e_minute)])
                        {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("{} Error with Database: {}", ERROR, e);
                                return Err(RequestError::DatabaseError);
                            }
                        };
                    return Ok("201 Created".to_string());
                }
                4 => {
                    // POST Subjects - contains id and name
                    let id = match parsed_request.content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    let name;
                    match parsed_request.content[1].as_str(){
                        "" => return Err(RequestError::UnknownRequestError),
                        _ => {name = parsed_request.content[1].as_str()}
                    };
                    let database = db.lock().await;
                    match database.execute(&"INSERT INTO Subjects (subject_id, subject_name) VALUES (?1, ?2)
                        ON CONFLICT (subject_id) DO UPDATE SET subject_name = excluded.subject_name", &[id.to_string().as_str(), name]){
                        Ok(_) => {},
                        Err(_) => return Err(RequestError::UnknownRequestError)
                    };
                    return Ok("201 Created".to_string());
                }
                5 => {
                    // POST Classrooms - contains id and name
                    let id = match parsed_request.content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    let name;
                    match parsed_request.content[1].as_str(){
                        "" => return Err(RequestError::ParseIntError(parsed_request.content[0].clone())),
                        _ => {name = parsed_request.content[1].as_str()}
                    };
                    let database = db.lock().await;
                    match database.execute("INSERT INTO Classrooms (classroom_id, classroom_name) VALUES (?1, ?2)
                        ON CONFLICT (classroom_id) DO UPDATE SET classroom_name = excluded.classroom_name", &[id.to_string().as_str(), name]){
                        Ok(_) => {},
                        Err(_) => return Err(RequestError::DatabaseError)
                    };
                    return Ok("201 Created".to_string());
                }
                6 => {
                    // POST Duties - contains teacher id, classroom_id, day (1, 7), and break number 
                    let teacher_id = match parsed_request.content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    let weekday = match parsed_request.content[1].parse::<u8>(){
                        Ok(v) => {
                            if v <= 7 && v > 0{
                                v
                            }
                            else{
                                return Err(RequestError::ParseIntError(parsed_request.content[1].clone()));
                            }
                        }
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[1].clone()))
                    };
                    let lesson_number = match parsed_request.content[2].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[2].clone()))
                    };
                    let classroom_id = match parsed_request.content[3].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[3].clone()))
                    };
                    let database = db.lock().await;
                    match database.execute("INSERT INTO Duties (lesson_hour, teacher_id, classroom_id, week_day) VALUES (?1, ?2, ?3, ?4)
                        ON CONFLICT (lesson_hour, teacher_id, week_day) DO UPDATE SET classroom_id = excluded.classroom_id", 
                        &[lesson_number.to_string().as_str(), teacher_id.to_string().as_str(), classroom_id.to_string().as_str(), weekday.to_string().as_str()]){
                        Ok(_) => return Ok("201 Created".to_string()),
                        Err(_) => {
                            return Err(RequestError::DatabaseError)
                        }
                    }
                }
                7 => {
                    // POST Classes - contains class number (UNIQUE!) and name
                    let id = match parsed_request.content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    let name;
                    match parsed_request.content[1].as_str(){
                        "" => return Err(RequestError::UnknownRequestError),
                        _ => {name = parsed_request.content[1].as_str()}
                    };
                    let database = db.lock().await;
                    match database.execute("INSERT INTO Classes (class_id, class_name) VALUES (?1, ?2)
                        ON CONFLICT (class_id) DO UPDATE SET class_name = excluded.class_name", [id.to_string().as_str(), name]){
                        Ok(_) => {return Ok("201 Created".to_string())},
                        Err(_) =>{
                            return Err(RequestError::DatabaseError)
                        }
                    }
                }
                _ => {
                    return Err(RequestError::UnknownRequestError);
                }
            }
        }
        Request::Other => {}
    }
    Ok("418 I'm teapot".to_string())
}

fn quick_match<T, E>(statement: Result<T, E>) -> Option<T>
where T: std::fmt::Display, E: std::fmt::Display{
    match statement{
        Ok(v) => {println!("Value of quick_match: {}", v);return Some(v);},
        Err(e) => {eprintln!("Error from quick_match: {}", e);return None;}
    }
}
fn format_two_digit_time(time1: u8, time2: u8) -> String{
    if time1 < 10 && time2 < 10{
        return format!("0{}0{}", time1, time2);
    }
    else if time1 < 10 && time2 > 10{
        return format!("0{}{}", time1, time2);
    }
    else if time1 > 10 && time2 < 10{
        return format!("{}0{}", time1, time2);
    }
    else{
        return format!("{}{}", time1, time2);
    }
}
fn format_mmdd(input: &str) -> Result<(u8, u8), ()>{
    if input.len() != 4{
        return Err(());
    }
    let month = match input[0..1].parse::<u8>(){
        Ok(v) => v,
        Err(_) => {
            return Err(());
        }
    };
    let day = match input[2..3].parse::<u8>(){
        Ok(v) => v,
        Err(_) => {
            return Err(());
        }
    };
    return Ok((month, day));
}
fn format_time(time: u32) -> String{
    if time < 10{
        return format!("0{}", time);
    }
    else{
        return format!("{}", time);
    }
}
async fn parse_request(input: &str) -> Result<(Request,Vec<String>, Option<String>,u8), ConnectionError> {
    let sliced_input = input.split_whitespace().collect::<Vec<&str>>();
    let (mut request_type,mut content,mut password, mut request_num) : (Request,Vec<String>,Option<String>,u8) = 
    (Request::Other,vec![],None,0);
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
    match sliced_input[2].parse::<u8>(){
        Ok(v) => {request_num = v}
        Err(_) => {
            return Err(ConnectionError::Other);
        }
    };
    for word in &sliced_input[3..]{
        if word.contains("password=") && password.is_none(){
            if request_type == Request::POST{
                password = Some(word[9..].to_string());
            } 
        }
        else if word.contains("password=") && password.is_some(){
            println!("{} Password was provided more than once!", ERROR);
        }
        else if word.is_empty() == false{
            content.push(word.to_string());
        }
    }
    if request_type == Request::POST{
        if let Some(correct_password) = get_password().await{
            if let Some(ref input_password) = password{
                if &correct_password != input_password{
                    return Err(ConnectionError::WrongPassword)
                }
            }
            else{
                return Err(ConnectionError::WrongPassword)
            }
        }
    }
    else{
        return Err(ConnectionError::WrongPassword)
    }
    Ok((request_type,content,password,request_num))
}
async fn get_lesson_hour(db: Arc<Mutex<SQLite>>) -> Result<u8, ()>{
    let now = chrono::Local::now();
    let (month, day) : (u8, u8) = (match now.month().try_into() {Ok(v) => v,Err(_) => {return Err(())}},
    match now.day().try_into(){Ok(v)=>v,Err(_)=>{return Err(());}});
    let formatted = format_two_digit_time(month, day);
    let database = db.lock().await;
    let query = "SELECT * FROM LessonHours 
        WHERE start_time < CAST(?1 AS INTEGER) AND end_time > CAST(?2 AS INTEGER);";
    let mut stmt = match database.prepare(&query){
        Ok(v) => v,
        Err(_) => {
            return Err(());
        }
    };
    let result_iter = stmt.query_map([&formatted, &formatted],|row|{
        Ok(quick_match(row.get::<usize, u8>(0)))
    });
    match result_iter{
        Ok(iter) => {
            for element in iter{
                match element{
                    Ok(value) => {
                        match value{
                            Some(v) => return Ok(v),
                            None => return Err(())
                        }
                    }
                    Err(_) => return Err(())
                }
            }
        }
        Err(_) => {return Err(())}
    }

    return Ok(0);
}
async fn get_teacher_duty_bool(teacher_id: u8,db: Arc<Mutex<SQLite>>) -> Result<bool, ()>{
    let lesson_hour = match get_lesson_hour(Arc::clone(&db)).await{
        Ok(v) => v,
        Err(_) => return Err(())
    };
    let database = db.lock().await;
    let mut stmt = match database.prepare("SELECT * FROM Duties 
        WHERE teacher_id = ?1 AND week_day = ?2 AND lesson_hour = ?3;"){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    let item = match stmt.query_row([teacher_id,chrono::Local::now().weekday() as u8,lesson_hour], |row|{
    Ok(quick_match(row.get::<usize, u8>(1)))}){
        Ok(v) => v,
        Err(e) => {
            match e{
                rusqlite::Error::QueryReturnedNoRows => {
                    return Ok(false);
                }
                _ => {
                    return Err(());
                }
            }
        }
    };
    match item{
        Some(v) => return Ok(v == teacher_id),
        None => return Err(())
    }
}
async fn get_classroom(id: u8,db: Arc<Mutex<SQLite>>) -> Result<String, ()>{
    let query = "SELECT * FROM Classrooms WHERE classroom_id = ?1;";
    let database = db.lock().await;
    let mut stmt = match database.prepare(&query){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    let element = match stmt.query_row([id], |row|{
        Ok(
            quick_match(row.get::<usize, String>(1))
        )
    }){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    match element{
        Some(v1) => {
            return Ok(v1);
        }
        None => return Err(())
    }
}
async fn get_class(id: u8, db: Arc<Mutex<SQLite>>) -> Result<String, ()>{
    let query = "SELECT * FROM Classes WHERE class_id = ?1;";
    let database = db.lock().await;
    let mut stmt = match database.prepare(&query){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    let element = match stmt.query_row([id], |row|{
        Ok(quick_match(row.get::<usize, String>(1)))
    }){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    match element{
        Some(v) => return Ok(v),
        None => return Err(())
    }
}
async fn get_teacher(id: u8, db: Arc<Mutex<SQLite>>) -> Result<String, ()>{
    let query = "SELECT * FROM Teachers WHERE teacher_id = ?1;";
    let database = db.lock().await;
    let mut stmt = match database.prepare(&query){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    let element = match stmt.query_row([id], |row|{
        Ok(
            quick_match(row.get::<usize, String>(1))
        )
    }){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    match element{
        Some(v) => return Ok(v),
        None => return Err(())
    }
}
async fn get_password() -> Option<String>{
    let file = match tokio::fs::read_to_string("data/config.toml").await{
        Ok(v) => v,
        Err(_) => return None
    };
    let structure = match toml::from_str::<Configuration>(&file){
        Ok(v) => v,
        Err(_) => return None
    };
    if structure.password.is_empty(){
        return None
    }
    else{
        return Some(structure.password);
    }
}
