///============================
/// Contains core of msat's
/// backend logic like parsing
/// requests and handling them
///============================

// Global Imports
use rusqlite::{
    Connection as Database,
    OpenFlags  as Flags,
    Error      as SQLiteError
};
use tokio::fs;
use toml::from_str;
use std::{
    net::TcpStream,
    io::Read,
    collections::HashMap
};
// Local Imports 
use crate::consts::VERSION;
use crate::types::Configuration;
// Struct Initialization

#[allow(unused)]
#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestType{
    GET,
    POST,
    Other(String),
    #[default]
    Unknown
}
#[allow(unused)]
#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Request{
    pub request: String,
}
#[allow(unused)]
#[derive(Default, Clone, PartialEq, Eq)]
pub struct ParsedRequest{
    pub req_type: RequestType,
    pub req_numb: u8,
    pub args: HashMap<String, String>
}

impl Request{
    fn from_tcp(request: &mut TcpStream) -> Result<Request, ()>{
        let mut buf : Vec<u8> = Vec::new();
        if let Ok(len) = request.read(&mut buf){
            if len <= 1{
                return Err(());
            }
            else{
                return Ok(Request{request: String::from_utf8_lossy(&buf).to_string()});
            }
        }
        else{
            return Err(());
        }
    }
    fn from_str(request: &str) -> Request{
        return Request{request: request.to_string()}
    }
    fn parse(&self) -> Result<ParsedRequest, ()>{
        if self.request.starts_with(&format!("/msat/{}", VERSION)) == false || 
           self.request.starts_with(&format!("msat/{}" , VERSION)) == false
        {
            return Err(());
        }
        let vector = self.request.split('&').collect::<Vec<&str>>();
        if vector.len() == 1{
            return Err(());
        }
        let mut to_return = ParsedRequest::default();
        let mut finhashmap = HashMap::new();
        for word in vector{
            if let Some((key, value)) = word.split_once('='){
                finhashmap.insert(key, value);
            }
        }
        if let Some(value) = finhashmap.get("method"){
            if let Some((method, numb)) = value.split_once('+'){
                match method.to_uppercase().as_str(){
                    "GET"  => to_return.req_type = RequestType::GET,
                    "POST" => to_return.req_type = RequestType::POST,
                    _      => to_return.req_type = RequestType::Other(method.to_string())
                }
                to_return.req_numb = numb.parse().unwrap_or(0);
            }
        }

        return Err(());
    }
}

// Functions

pub async fn get_config() -> Option<Configuration>{
    return match fs::read_to_string("data/config.toml").await{
        Ok(v) => {
            match toml::from_str::<Configuration>(&v){
                Ok(conf) => Some(conf),
                Err(_)   => None
            }
        }
        Err(_) => {
            if let Ok(b) = fs::try_exists("data/config.toml").await{
                if b == false{
                    let _ = fs::write("data/config.toml", "").await;
                }
            }
            None
        }
    }
}

pub async fn get_password() -> Option<String>{
    return match fs::read_to_string("data/config.toml").await{
        Ok(v) => {
            match from_str::<Configuration>(&v){
                Ok(conf) => {
                    if conf.password == ""{
                        None
                    }
                    else{
                        Some(conf.password)
                    }
                }
                Err(_) => None
            }
        }
        Err(_) => None
    };
}

pub fn init_db() -> Result<Database, SQLiteError>{
    let db = Database::open_with_flags(
        "data/database.db",
        // Enter flags
        Flags::SQLITE_OPEN_FULL_MUTEX|
        Flags::SQLITE_OPEN_READ_WRITE|
        Flags::SQLITE_OPEN_CREATE
    )?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Classes(
            class_id   INTEGER PRIMARY KEY,
            class_name TEXT NOT NULL UNIQUE
        );
        "
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Classrooms(
            classroom_id INTEGER PRIMARY KEY,
            class_name   TEXT NOT NULL UNIQUE
        );"
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Teachers(
            teacher_id   INTEGER PRIMARY KEY,
            teacher_name TEXT NOT NULL UNIQUE
        );"
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Subjects(
            subject_id   INTEGER PRIMARY KEY,
            subject_name TEXT NOT NULL UNIQUE
        );"
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS LessonHours(
            lesson_hour   INTEGER PRIMARY KEY,
            start_hour    INTEGER NOT NULL CHECK(start_hour >= 0 AND start_hour < 24),
            start_minutes INTEGER NOT NULL CHECK(start_minutes >= 0 AND start_minutes < 60),
            end_hour      INTEGER NOT NULL CHECK(end_hour >= 0 AND end_hour < 24),
            end_minutes   INTEGER NOT NULL CHECK(end_minutes >= 0 AND end_minutes < 60)
        );"
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Semesters(
            semester_num  INTEGER PRIMARY KEY,
            semester_name TEXT NOT NULL UNIQUE
        );"
        ,[])?;
    // start_date should be formatted as ISO8601 compatible date
    db.execute(
        "CREATE TABLE IF NOT EXISTS Years(
            academic_year  INTEGER PRIMARY KEY,
            year_name      TEXT NOT NULL UNIQUE,
            start_date     TEXT NOT NULL,
            end_date       TEXT NOT NULL
        );"
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Lessons (
            weekday       INTEGER NOT NULL,
            class_id      INTEGER NOT NULL,
            classroom_id  INTEGER NOT NULL,
            teacher_id    INTEGER NOT NULL,
            subject_id    INTEGER NOT NULL,
            lesson_hour   INTEGER NOT NULL,
            semester      INTEGER NOT NULL,
            academic_year TEXT    NOT NULL,
            PRIMARY KEY (class_id, weekday, lesson_hour, semester, academic_year),
            FOREIGN KEY (class_id)      REFERENCES Classes    (class_id),
            FOREIGN KEY (classroom_id)  REFERENCES Classrooms (classroom_id),
            FOREIGN KEY (teacher_id)    REFERENCES Teachers   (teacher_id),
            FOREIGN KEY (subject_id)    REFERENCES Subjects   (subject_id),
            FOREIGN KEY (lesson_hour)   REFERENCES LessonHours(lesson_hour),
            FOREIGN KEY (semester)      REFERENCES Semesters  (semester_num),
            FOREIGN KEY (academic_year) REFERENCES Years      (academic_year)
        );
        "
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Corridors(
            corridor      INTEGER PRIMARY KEY,
            corridor_name TEXT NOT NULL UNIQUE
        );
        "
        ,[])?;
    db.execute(
        "
        CREATE TABLE IF NOT EXISTS Breaks(
            break_num     INTEGER PRIMARY KEY,
            start_hour    INTEGER NOT NULL CHECK(start_hour >= 0 AND start_hour < 24),
            start_minutes INTEGER NOT NULL CHECK(start_minutes >= 0 AND start_minutes < 60),
            end_hour      INTEGER NOT NULL CHECK(end_hour >= 0 AND end_hour < 24),
            end_minutes   INTEGER NOT NULL CHECK(end_minutes >= 0 AND end_minutes < 60)
        );
        "
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Duties(
            weekday       INTEGER NOT NULL,
            break_num     INTEGER NOT NULL,
            teacher_id    INTEGER NOT NULL,
            place_id      INTEGER NOT NULL,
            semester      INTEGER NOT NULL,
            academic_year INTEGER NOT NULL,
            PRIMARY KEY (weekday, break_num, teacher_id, semester, academic_year),
            FOREIGN KEY (break_num) REFERENCES Breaks (break_num),
            FOREIGN KEY (teacher_id) REFERENCES Teachers (teacher_id),
            FOREIGN KEY (place_id) REFERENCES 
        );
        "
        ,[])?;
    db.execute_batch("PRAGMA journal_mode = WAL")?;
    db.busy_timeout(std::time::Duration::from_secs(4))?;

    return Ok(db);
}
