use std::{
    clone, io::{Read, Write}, net::{IpAddr, Ipv4Addr, TcpListener, TcpStream}, str::FromStr, sync::Arc
};

const SUCCESS : &str = "HTTP_SERVER: [     OK     ]";
const ERROR   : &str = "HTTP_SERVER: [    ERROR   ]";

use tokio::{fs, sync::Mutex};
use serde::{Deserialize, Serialize};
use rusqlite::{self, OpenFlags};
use toml;

struct Lesson{
    week_day: u8,
    class_id: u8,
    lesson_hour: u8,
    teacher_id: u8,
    subject_id: u8,
    classroom_id: u8
}
struct Class{
    class_id: u8,
    class_name: String
}
struct LessonHour{
    lesson_num: u8,
    start_time: u16,
    end_time: u16,
}
struct Teacher{
    teacher_id: u8,
    first_name: String,
    last_name: String
}
struct Classroom{
    classroom_id: u8,
    classroom_name: String
}
struct Subject{
    subject_id: u8,
    subject_name: String
}
struct Duty{
    lesson_hour: u8,
    teacher_id: u8,
    classroom_id: u8,
    week_day: u8
}

#[tokio::main]
async fn main(){
    init(IpAddr::from_str("127.0.0.1").unwrap()).await;
}

pub async fn init(ip_addr: IpAddr) {
    if let Err(_) = init_database().await{
        std::process::exit(-1);
    };
    let database = Arc::new(Mutex::new(
            match rusqlite::Connection::open_with_flags("data/database.db",
                OpenFlags::SQLITE_OPEN_CREATE|OpenFlags::SQLITE_OPEN_FULL_MUTEX|OpenFlags::SQLITE_OPEN_READ_WRITE){
                Ok(v) => v,
                Err(_) => std::process::exit(-1)
            }
    ));
    let final_address = format!("{}:8000", ip_addr.to_string());
    let shared_ipaddr = Arc::new(ip_addr);
    let listener: TcpListener = match TcpListener::bind(final_address) {
        Ok(v) => v,
        Err(_) => std::process::exit(-1),
    };
    println!("initialized");
    loop {
        for s in listener.incoming() {
            println!("request");
            if let Ok(stream) = s{
                let cloned_dbptr = Arc::clone(&database);
                tokio::spawn(async {
                    handle_connection(stream, cloned_dbptr).await;
                });
            }
            else if let Err(error) = s{
                println!("eror?: {}", error);
            }
        }
    }
}
pub async fn handle_connection(mut stream: TcpStream, db_ptr: Arc<Mutex<rusqlite::Connection>>) {
    let mut buffer = [0u8; 2048];
    if let Ok(len) = stream.read(&mut buffer) {
        if len == 0 {
        } else {
            let request = String::from_utf8_lossy(&buffer).to_string();
            let lines = request
                .lines()
                .filter(|s| s.is_empty() == false)
                .collect::<Vec<&str>>();
            let mut types: Vec<String> = vec![];
            let mut file_path: String = String::new();
            for line in lines {
                let request = line
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();
                if request.contains(&"GET".to_string()) {
                    let split_line: Vec<String> = request
                        .clone()
                        .into_iter()
                        .filter(|s| !s.starts_with("GET") && s.starts_with('/'))
                        .collect();
                    for w in split_line {
                        if w == "/" || w.starts_with("/?lang"){
                            file_path = "./web/index.html".to_string();
                        } else {
                            if !w.starts_with("/?"){
                                file_path = format!("./web{}", w)
                            }
                            else if w.starts_with("/?") && !w.starts_with("/?lang="){
                                let cloned_dbptr = Arc::clone(&db_ptr);
                                let response = handle_custom_request(&w, cloned_dbptr).await;
                                match stream.write_all(
                                    format!("HTTP/1.1 200 OK\r\nContent-Length:{}\r\nContent-Type:application/xml\r\n\r\n{}",
                                        response.len(), response).as_bytes())
                                {
                                    Ok(_) => println!("Success"),
                                    Err(_) => println!("Error")
                                };
                            }
                        }
                    }
                }
                if request.contains(&"Accept:".to_string()) {
                    let split_line: Vec<String> = request
                        .into_iter()
                        .filter(|s| !s.starts_with("Accept:"))
                        .collect();
                    for w in split_line {
                        types = split_string_by(&w, ',');
                    }
                }
            }
            if types.len() == 0 {
                types = vec!["*/*".to_string()];
            }
            // End of checks
            let binary: bool = types[0].starts_with("image");
            let f_type = &types[0];
            println!("FILE_PATH = {}\n===", file_path);

            if binary == false {
                if let Ok(buf) = tokio::fs::read(file_path).await {
                    if let Ok(string) = String::from_utf8(buf.clone()) {
                        stream.write_all(
                                        format!("HTTP/1.1 200 OK\r\nContent-Length:{}\r\nContent-Type:{}\r\n\r\n{}",
                                            string.len(), f_type, string)
                                        .as_bytes())
                                    .unwrap();
                    } else {
                        let string = String::from_utf8_lossy(&buf).to_string();
                        stream.write_all(
                                        format!("HTTP/1.1 200 OK\r\nContent-Length:{}\r\nContent-Type:{}\r\n\r\n{}",
                                            string.len(), f_type, string).as_bytes())
                                    .unwrap()
                    };
                } else {
                    not_found(&mut stream);
                }
            } else {
                if let Ok(buf) = tokio::fs::read(file_path).await {
                    let http_header = 
                                    format!("HTTP/1.1 200 OK\r\nContent-Length:{}\r\nContent-Type:{}\r\nConnection: keep-alive\r\n\r\n",
                                    buf.len(), f_type);
                    stream.write_all(http_header.as_bytes()).unwrap();
                    let mut vector = Vec::with_capacity(buf.len() + http_header.len());
                    vector.extend_from_slice(buf.as_slice());
                    vector.extend_from_slice(http_header.as_bytes());
                    stream.write_all(&buf).unwrap();
                } else {
                    not_found(&mut stream);
                }
            }
        }
    }
}
async fn handle_custom_request(request: &str, db: Arc<Mutex<rusqlite::Connection>>) -> String{
    // request example: /?method=POST&version=10&args=20
    let request = String::from_iter(request.chars().collect::<Vec<char>>()[2..].iter());
    println!("REQUEST {}", request);
    let mut request_type = "".to_string();
    let mut args : Vec<String> = vec![];
    let mut request_number = 0;
    let req_split = split_string_by(&request, '&');

    for s in req_split{
        if s.starts_with("args="){
            println!("{}", s);
            args = 
                split_string_by(&strvec_to_str(&split_string_by(&s, '=')[1..].to_vec()),'+');
        }
        else if s.starts_with("method="){
            println!("{}", s);
            let request_arguments = split_string_by(&strvec_to_str(&split_string_by(&s, '=')[1..].to_vec()),'+');
            request_type = request_arguments[0].to_string();
            if let Ok(v) = str::parse::<u8>(&request_arguments[1]){
                request_number = v;
            }
        }
    }
    
    println!("REQUEST: \"{}\", {}, \"{}\"", request_type, request_number, strvec_to_str(&args));
    match request_type.as_str(){
        "GET" => {
            match request_number{
                1 => {
                    let database = db.lock().await;
                    let query = "SELECT * FROM Lessons";
                    if let Ok(mut stmt) = database.prepare(&query){
                        if let Ok(iter) = stmt.query_map([], |row| {
                            Ok(Lesson{
                                week_day: row.get(0).unwrap_or(0),
                                class_id: row.get(1).unwrap_or(0),
                                lesson_hour: row.get(2).unwrap_or(0),
                                teacher_id: row.get(3).unwrap_or(0),
                                subject_id: row.get(4).unwrap_or(0),
                                classroom_id: row.get(5).unwrap_or(0)
                            })
                        })
                        {
                            let filtered_iter : Vec<Lesson> 
                                = iter.filter(|s| s.is_ok())
                                .map(|s| s.unwrap())
                                .filter(|s| s.class_id!=0&&s.lesson_hour!=0&&s.teacher_id!=0&&s.subject_id!=0&&s.classroom_id!=0&&s.week_day!=0)
                                .collect();
                            let mut to_return : String = String::from("<db_col>
                                <db_row>
                                <p>Week Day</p>
                                <p>Teacher ID</p>
                                <p>Class ID</p>
                                <p>Classroom ID</p>
                                <p>Subject ID</p>
                                <p>Lesson Hour</p>
                                </db_row>");
                            for e in filtered_iter{
                                to_return.push_str(
                                    format!("<db_row>
                                    <p>{}</p>
                                    <p>{}</p>
                                    <p>{}</p>
                                    <p>{}</p>
                                    <p>{}</p>
                                    <p>{}</p></db_row>", e.week_day,e.teacher_id,e.class_id,e.classroom_id,e.subject_id,e.lesson_hour).as_str()
                                );
                            }
                            to_return.push_str("</db_col>");
                            return to_return;
                        };
                    };
                }
                2 => {
                    let database = db.lock().await;
                    let query = "SELECT * FROM Teachers";
                    if let Ok(mut stmt) = database.prepare(&query){
                        if let Ok(iter) = stmt.query_map([], |row| {
                            Ok(Teacher{
                                teacher_id: row.get(0).unwrap_or(0),
                                first_name: row.get(1).unwrap_or("".to_string()),
                                last_name: row.get(2).unwrap_or("".to_string())
                            })
                        })
                        {
                            let filtered_iter : Vec<Teacher> 
                                = iter.filter(|s| s.is_ok())
                                .map(|s| s.unwrap())
                                .filter(|s| s.first_name.is_empty()==false&&s.last_name.is_empty()==false&&s.teacher_id!=0)
                                .collect();
                            let mut to_return : String = String::from("<db_col>
                                <db_row>
                                <p>Teacher ID</p>
                                <p>First Name</p>
                                <p>Last Name</p>
                                </db_row>");
                            for e in filtered_iter{
                                to_return.push_str(
                                    format!("<db_row>
                                    <p>{}</p>
                                    <p>{}</p>
                                    <p>{}</p></db_row>", e.teacher_id,e.first_name,e.last_name).as_str()
                                );
                            }
                            to_return.push_str("</db_col>");
                            return to_return;
                        };
                    };
                }
                3 => {
                    let database = db.lock().await;
                    let query = "SELECT * FROM Duties";
                    if let Ok(mut stmt) = database.prepare(&query){
                        if let Ok(iter) = stmt.query_map([], |row| {
                            Ok(Duty{
                                lesson_hour: row.get(0).unwrap_or(0),
                                teacher_id: row.get(1).unwrap_or(0),
                                classroom_id: row.get(2).unwrap_or(0),
                                week_day: row.get(3).unwrap_or(0),
                            })
                        })
                        {
                            let filtered_iter : Vec<Duty> 
                                = iter.filter(|s| s.is_ok())
                                .map(|s| s.unwrap())
                                .filter(|s| s.lesson_hour!=0&&s.teacher_id!=0&&s.classroom_id!=0&&s.week_day!=0)
                                .collect();
                            let mut to_return : String = String::from("<db_col>
                                <db_row>
                                <p>Lesson Hour</p>
                                <p>Teacher ID</p>
                                <p>Classroom ID</p>
                                <p>Week Day</p>
                                </db_row>");
                            for e in filtered_iter{
                                to_return.push_str(
                                    format!("<db_row>
                                    <p>{}</p>
                                    <p>{}</p>
                                    <p>{}</p>
                                    <p>{}</p></db_row>", e.lesson_hour,e.teacher_id,e.classroom_id,e.week_day).as_str()
                                );
                            }
                            to_return.push_str("</db_col>");
                            return to_return;
                        };
                    };
                }
                4 => {
                    let database = db.lock().await;
                    let query = "SELECT * FROM Subjects";
                    if let Ok(mut stmt) = database.prepare(&query){
                        if let Ok(iter) = stmt.query_map([], |row| {
                            Ok(Subject{
                                subject_id: row.get(0).unwrap_or(0),
                                subject_name: row.get(1).unwrap_or("".to_string()),
                            })
                        })
                        {
                            let filtered_iter : Vec<Subject> 
                                = iter.filter(|s| s.is_ok())
                                .map(|s| s.unwrap())
                                .filter(|s| s.subject_name.is_empty()==false&&s.subject_id!=0)
                                .collect();
                            let mut to_return : String = String::from("<db_col>
                                <db_row>
                                <p>Subject ID</p>
                                <p>Subject Name</p>
                                </db_row>");
                            for e in filtered_iter{
                                to_return.push_str(
                                    format!("<db_row>
                                    <p>{}</p>
                                    <p>{}</p></db_row>", e.subject_id,e.subject_name).as_str()
                                );
                            }
                            to_return.push_str("</db_col>");
                            return to_return;
                        };
                    };
                }
                5 => {
                    let database = db.lock().await;
                    let query = "SELECT * FROM Classes";
                    if let Ok(mut stmt) = database.prepare(&query){
                        if let Ok(iter) = stmt.query_map([], |row| {
                            Ok(Class{
                                class_id: row.get(0).unwrap_or(0),
                                class_name: row.get(1).unwrap_or("".to_string()),
                            })
                        })
                        {
                            let filtered_iter : Vec<Class> 
                                = iter.filter(|s| s.is_ok())
                                .map(|s| s.unwrap())
                                .filter(|s| s.class_name.is_empty()==false&&s.class_id!=0)
                                .collect();
                            let mut to_return : String = String::from("<db_col>
                                <db_row>
                                <p>Class ID</p>
                                <p>Class Name</p>
                                </db_row>");
                            for e in filtered_iter{
                                to_return.push_str(
                                    format!("<db_row>
                                    <p>{}</p>
                                    <p>{}</p></db_row>", e.class_id,e.class_name).as_str()
                                );
                            }
                            to_return.push_str("</db_col>");
                            return to_return;
                        };
                    };
                }
                6 => {
                    let database = db.lock().await;
                    let query = "SELECT * FROM Classrooms";
                    if let Ok(mut stmt) = database.prepare(&query){
                        if let Ok(iter) = stmt.query_map([], |row| {
                            Ok(Classroom{
                                classroom_id: row.get(0).unwrap_or(0),
                                classroom_name: row.get(1).unwrap_or("".to_string()),
                            })
                        })
                        {
                            let filtered_iter : Vec<Classroom> 
                                = iter.filter(|s| s.is_ok())
                                .map(|s| s.unwrap())
                                .filter(|s| s.classroom_id!=0&&s.classroom_name.is_empty()==false)
                                .collect();
                            let mut to_return : String = String::from("<db_col>
                                <db_row>
                                <p>Classroom ID</p>
                                <p>Classroom Name</p>
                                </db_row>");
                            for e in filtered_iter{
                                to_return.push_str(
                                    format!("<db_row>
                                    <p>{}</p>
                                    <p>{}</p></db_row>", e.classroom_id,e.classroom_name).as_str()
                                );
                            }
                            to_return.push_str("</db_col>");
                            return to_return;
                        };
                    };
                }
                7 => {    
                    let database = db.lock().await;
                    let query = "SELECT * FROM LessonHours";
                    if let Ok(mut stmt) = database.prepare(&query){
                        if let Ok(iter) = stmt.query_map([], |row| {
                            Ok(LessonHour{
                                lesson_num: row.get(0).unwrap_or(0),
                                start_time: row.get(1).unwrap_or(0),
                                end_time: row.get(2).unwrap_or(0)
                            })
                        })
                        {
                            let filtered_iter : Vec<LessonHour> 
                                = iter.filter(|s| s.is_ok())
                                .map(|s| s.unwrap())
                                .filter(|s| s.lesson_num!=0&&s.start_time!=0&&s.end_time!=0)
                                .collect();
                            let mut to_return : String = String::from("<db_col>
                                <db_row>
                                <p>Lesson Number</p>
                                <p>Start Time</p>
                                <p>End Time</p>
                                </db_row>");
                            for e in filtered_iter{
                                to_return.push_str(
                                    format!("<db_row>
                                    <p>{}</p>
                                    <p>{}</p>
                                    <p>{}</p></db_row>", e.lesson_num,e.start_time,e.end_time).as_str()
                                );
                            }
                            to_return.push_str("</db_col>");
                            return to_return;
                        };
                    };
                }
                _ => {
                }
            }
        }
        "POST" => {
            match request_number{
                1 => {
                    let query = "INSERT INTO Lessons 
                            (week_day, class_id, classroom_id, subject_id, teacher_id, lesson_hour) 
                            VALUES (?1,?2,?3,?4,?5,?6)
                            ON CONFLICT (class_id, lesson_hour, week_day) 
                            DO UPDATE SET classroom_id = excluded.classroom_id, subject_id = excluded.subject_id,
                            teacher_id = excluded.teacher_id;";
                    let (class_id, classroom_id, subject_id, teacher_id, lesson_num, week_day) = 
                    (
                        str::parse::<u8>(args[2].trim()), str::parse::<u8>(args[3].trim()),
                        str::parse::<u8>(args[4].trim()), str::parse::<u8>(args[1].trim()),
                        str::parse::<u8>(args[5].trim()), str::parse::<u8>(args[0].trim())
                    );
                    if class_id.is_ok()&classroom_id.is_ok()&subject_id.is_ok()
                        &teacher_id.is_ok()&lesson_num.is_ok()&week_day.is_ok()== true
                    {
                        let (u_class, u_classroom, u_subject, u_teacher, u_lesson, u_weekday) = 
                        (class_id.unwrap(), classroom_id.unwrap(), subject_id.unwrap(), teacher_id.unwrap(), 
                         lesson_num.unwrap(),week_day.unwrap());
                        let database = db.lock().await;
                        if let Ok(_) = database.execute(&query, 
                            [u_weekday, u_class, u_classroom, u_subject, u_teacher, u_lesson])
                        {
                            return 
                                "<db_col><db_row><p>Successfully added data</p></db_row>
                                <db_row><p>Pomyślnie dodano dane do bazy danych</p></db_row></db_col>".to_string()
                        } else{
                            return "<db_col>
                            <db_row><p>Error</p></db_row>
                            <db_row><p>Błąd</p></db_row>
                                </db_col>".to_string()
                        };
                    }
                    else{
                        return "<db_col><db_row><p>Error 1</p></db_row><db_row><p>Błąd 1</p></db_row></db_col>".to_string()
                    }
                }
                _ => {
                    return "<h1>Unknown request/Nieznane zapytanie</h1>".to_string()
                }
            }
        }
        _ => {
            return "<h1>Error - wrong request</h1>".to_string();
        }
    }
    return "<h1>Couldn't get any data</h1>".to_string();
}

fn not_found(tcp: &mut TcpStream) {
    match tcp.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n<h1>404 - Not Found</h1>") {
        Ok(_) => println!("Success"),
        Err(e) => eprintln!("Error: {}", e),
    };
}

fn strvec_to_str(vec: &Vec<String>) -> String{
    let mut finstr = "".to_string();
    for e in vec {
        if finstr.as_str() != ""{
            finstr = format!("{} {}", finstr, e);
        }
        else{
            finstr = e.to_string();
        }
    }
    return finstr
}

fn get_types(line: String) -> Vec<String> {
    let split_line = line.split_whitespace().collect::<Vec<&str>>();
    let mut types: Vec<String> = vec![];
    for s in split_line {
        if !s.starts_with("Accept:") {
            types = split_string_by(s, ',');
        }
    }
    types
}
fn split_string_by(string: &str, chr: char) -> Vec<String> {
    let mut temp_buf = vec![];
    let mut finvec = vec![];
    for c in string.chars().collect::<Vec<char>>() {
        if c != chr {
            temp_buf.push(c);
        } else {
            finvec.push(String::from_iter(temp_buf.iter()));
            temp_buf = vec![];
        }
    }
    if temp_buf.is_empty() == false{
        finvec.push(String::from_iter(temp_buf.iter()));
    }
    finvec
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test(){
        let v = split_string_by("dsdasd=dsda", '=');
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], "dsdasd");
        assert_eq!(v[1], "dsda");
    }
}
use rusqlite::Connection;

pub async fn init_database() -> Result<(), ()>{
    match std::fs::read_dir("data"){
        Ok(_) => {},
        Err(_) => {
            match std::fs::create_dir("data"){
                Ok(_) => {}
                Err(e)=>{
                    eprintln!("{} Error creating directory \"data\": {}", ERROR, e);
                    return Err(());
                }
            }
        }
    }
    let database: Connection = match Connection::open("data/database.db"){
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} Error connecting to database: {}", ERROR, e);
            return Err(());
        }
    };
    let query = ["CREATE TABLE IF NOT EXISTS LessonHours(
	lesson_num INTEGER PRIMARY KEY,
	start_time INTEGER NOT NULL,
	end_time INTEGER NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS Classes(
        class_id INTEGER PRIMARY KEY,
        class_name TEXT NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS Lessons(
        week_day INTEGER NOT NULL,
	class_id INTEGER NOT NULL,
	lesson_hour INTEGER NOT NULL,
	teacher_id INTEGER NOT NULL,
	subject_id INTEGER NOT NULL,
	classroom_id INTEGER NOT NULL,
	PRIMARY KEY (class_id, lesson_hour, week_day)
    );",
    "CREATE TABLE IF NOT EXISTS Teachers(
	teacher_id INTEGER PRIMARY KEY,
	first_name TEXT NOT NULL,
        last_name TEXT NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS Classrooms(
	classroom_id INTEGER PRIMARY KEY,
	classroom_name TEXT NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS Subjects(
	subject_id INTEGER PRIMARY KEY,
	subject_name TEXT NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS Duties(
	lesson_hour INTEGER NOT NULL,
	teacher_id INTEGER NOT NULL,
	classroom_id INTEGER NOT NULL,
        week_day INTEGER NOT NULL,
	PRIMARY KEY (lesson_hour, teacher_id, week_day),
	FOREIGN KEY (teacher_id) REFERENCES Teachers(teacher_id),
	FOREIGN KEY (classroom_id) REFERENCES Classrooms(classroom_id),
	FOREIGN KEY (lesson_hour) REFERENCES LessonHours(lesson_num)
    );"];
    for r in query{
        match database.execute(&r, []){
            Ok(_) => println!("{} Created Table", SUCCESS),
            Err(_) => println!("{} Couldn't create Table", ERROR)
        }
    }
    println!("{} Opened database", SUCCESS);
    return Ok(());
}

