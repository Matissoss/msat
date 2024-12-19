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
use chrono::{self, Datelike, Timelike};

#[derive(Clone,Debug,Default, PartialEq, Eq, PartialOrd, Ord)]
enum Request{
    GET,
    POST,
    #[default]
    Other
}

const VERSION : u16  = 10;
const SUCCESS : &str = "[     OK     ]";
const ERROR   : &str = "[     ERR     ]";

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
    WrongPassword,
    NoPassword,
    WritingError,
    ResponseError,
    Other
}

impl std::fmt::Display for ConnectionError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self{
            Self::Other => {
                writeln!(f, "Other")
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

#[tokio::main]
async fn main() {
    match Database::init().await{
        Ok(_) => {}
        Err(_) => {
            println!("Error initializing database");
            std::process::exit(-1);
        }
    }
    let database : Arc<Mutex<SQLite>> = Arc::new(Mutex::new(match SQLite::open("data/database.db"){
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error opening database: {}", e);
            std::process::exit(-1);
        }
    }));
    match database.lock().await.execute_batch("PRAGMA journal_mode = WAL;"){
        Ok(_) => {}
        Err(e) =>{
            eprintln!("Error executing batch: {}", e);
            std::process::exit(-1);
        }
    }
    match database.lock().await.busy_timeout(std::time::Duration::from_secs(4)){
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error setting busy_timeout: {}", e);
            std::process::exit(-1);
        }
    }

    let shared_config = Arc::new(match ConfigFile::get().await{
        Ok(v) => {v},
        Err(_) => {
            println!("Error getting configuration"); 
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
            println!("-----\nConnection with: {}, Port:{}", 
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
            match get_response(parsed_req, db).await{
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

async fn get_response(parsed_request: ParsedRequest, db: Arc<Mutex<SQLite>>) -> Result<String, ()>{
    println!("{}", parsed_request.request_number);
    match parsed_request.request{
        Request::GET => {
            match parsed_request.request_number{
                0 => {
                    return Err(());
                }
                1 => {
                    // GET Lessons for this Day 
                    let teacher = match parsed_request.content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(());
                        } 
                    };
                    let date : u8 = chrono::Local::now().weekday() as u8;
                    let database = db.lock().await;
                    let mut prompt = match database.prepare(
                    &format!("SELECT * FROM lessons WHERE teacher_id = {} AND week_day = {}", teacher,date)){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(());
                        }
                    };
                    // class_id, classroom_id, subject_id, lesson_number
                    let product_iter = match prompt.query_map([], |row|{
                        Ok((
                            quick_match(row.get::<usize,u8>(1)), //class_id,
                            quick_match(row.get::<usize,u8>(2)), // classroom_id,
                            quick_match(row.get::<usize,u8>(3)), // subject_id,
                            quick_match(row.get::<usize,u8>(5)) //lesson_number
                        ))
                    }){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(());
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
                                    return Ok(format!("200 {};{};{};{};", u_class,u_classroom,u_subject,u_lesson));
                                }
                                else{
                                    return Err(());
                                }
                            }
                            Err(_) => {
                                return Err(());
                            }
                        }
                    }
                    return Ok("200".to_string());
                }
                2 => {
                    // GET Hours for this lesson (start time and end time)
                    let current_time_hhmm = format!("{}{}",
                        format_time(chrono::Local::now().hour()), format_time(chrono::Local::now().minute()));
                    let current_time_mmdd = format!("{}{}",
                        format_time(chrono::Local::now().month()), format_time(chrono::Local::now().day()));
                    let database1 = db.lock().await;
                    let mut query = match database1.prepare(&format!("SELECT * FROM hours 
                            WHERE CAST(start_time AS INTEGER) < CAST({} AS INTEGER) AND 
                            CAST(end_time AS INTEGER) > CAST({} AS INTEGER) 
                            AND CAST({} AS INTEGER) = CAST(date AS INTEGER);",
                            current_time_hhmm, current_time_hhmm, current_time_mmdd))
                    {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("Error with db: {}", e);
                            return Err(());
                        } 
                    };
                    let stmt = match query.query_map([],|row| {
                        Ok((
                                quick_match(row.get::<usize, String>(1)),
                                quick_match(row.get::<usize, String>(2)),
                                quick_match(row.get::<usize, String>(3))
                        ))
                    }){
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("Error with database?: {}", e);
                            return Err(());
                        }
                    };
                    let (mut f_end, mut f_start, mut f_date) = ("".to_string(), "".to_string(), "".to_string());
                    for result in stmt{
                        match result{
                            Ok((date, start_time, end_time)) => {
                                if end_time.is_some()&start_time.is_some()&date.is_some() == true{
                                    let (u_end, u_start, u_date) = (end_time.unwrap(), start_time.unwrap(), date.unwrap());
                                    (f_end, f_start, f_date) = (u_end, u_start, u_date);
                                }
                                else{
                                    return Err(());
                                }
                            }
                            Err(e) => {
                                eprintln!("Error getting values: {}", e);
                                return Err(());
                            }
                        }
                    }
                    if f_end.is_empty()&f_start.is_empty()&f_date.is_empty() == false{
                        return Ok(format!("200 {};{};{}", f_date, f_start, f_end));
                    }
                    return Ok("100".to_string());
                }
                3 => {
                    // GET teacher for next duty (name)
                }
                4 => {
                    // GET teacher for next duty (bool)
                }
                5 => {
                    // GET current classroom && class
                }
                _ => {
                    return Err(());
                }
            };
        }
        Request::POST => {
            match parsed_request.request_number {
                0 => {
                    return Err(());
                }
                1 => {
                    // POST Lesson - contains class, classroom, subject, teacher, lesson number
                    let content = parsed_request.content;
                    let (class_id, classroom_id, subject_id, teacher_id, lesson_number) :
                        (Option<u8>, Option<u8>, Option<u8>, Option<u8>, Option<u8>)= 
                        (quick_match(content[0].parse()), quick_match(content[1].parse()), quick_match(content[2].parse()),
                         quick_match(content[3].parse()), quick_match(content[4].parse()));
                    if class_id.is_some()&classroom_id.is_some()&subject_id.is_some()&teacher_id.is_some()
                        &lesson_number.is_some() == true
                    {
                        let (u_class, u_classroom, u_subject, u_teacher, u_lesson) = 
                            (class_id.unwrap(), classroom_id.unwrap(), subject_id.unwrap(), 
                             teacher_id.unwrap(), lesson_number.unwrap());
                        let database = db.lock().await;
                        match database.execute(&format!("INSERT INTO lessons 
                            (week_day, class_id, classroom_id, subject_id, teacher_id, lesson_number) 
                            VALUES ({},{},{},{},{},{});", 
                            chrono::Local::now().weekday() as u8, u_class, u_classroom, u_subject, u_teacher, u_lesson), [])
                        {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("Error with database?: {}", e);
                                return Err(());
                            }
                        };
                    }
                    else{println!("All of values are none");return Err(())}
                    return Ok("200".to_string());
                }
                2 => {
                    // POST Teacher - contains ID and full name
                }
                3 => {
                    // POST Hours - contains start hour, number and end number
                    let content = parsed_request.content;
                    let mut index = 0;
                    for s in &content{
                        println!("{} : {}", index, s);
                        index += 1;
                    }
                    let date : u16 = match content[0].parse::<u16>(){
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("Error parsing: {}", e);
                            return Err(());
                        }
                    };
                    let (s_hour, s_minute) = match format_mmdd(&content[1]){
                        Ok(v) => v,
                        Err(_) => {
                            eprintln!("Error formatting to MMDD");
                            return Err(());
                        }
                    };
                    let (e_hour, e_minute) = match format_mmdd(&content[2]){
                        Ok(v) => v,
                        Err(_) => {
                            eprintln!("Error formatting to MMDD 2");
                            return Err(());
                        }
                    };
                        let database = db.lock().await;
                        match database.execute(&format!("INSERT INTO hours 
                            (date, start_time, end_time) 
                            VALUES ({},{},{});", 
                            date, format_two_digit_time(s_hour, s_minute), format_two_digit_time(e_hour, e_minute)), [])
                        {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("Error with database?: {}", e);
                                return Err(());
                            }
                        };
                    return Ok("200".to_string());
                }
                4 => {
                    // POST Subjects - contains id and name
                }
                5 => {
                    // POST Classrooms - contains id and name
                }
                6 => {
                    // POST Duties - contains teacher id, day (1, 7), and break number
                }
                7 => {
                    // POST Classes - contains class number (UNIQUE!) and name
                }
                _ => {
                    return Err(());
                }
            }
        }
        Request::Other => {}
    }
    Ok("Server didn't recognize request.".to_string())
}

fn quick_match<T, E>(statement: Result<T, E>) -> Option<T>{
    match statement{
        Ok(v) => {return Some(v);},
        Err(_) => {return None;}
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
    let month = match input[0..2].parse::<u8>(){
        Ok(v) => v,
        Err(_) => {
            return Err(());
        }
    };
    let day = match input[3..4].parse::<u8>(){
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
                password = Some(word[8..].to_string());
            } 
        }
        else if word.contains("password=") && password.is_some(){
            println!("Password was provided more than once!");
        }
        else if word.is_empty() == false{
            content.push(word.to_string());
        }
    }
    Ok((request_type,content,password,request_num))
}
