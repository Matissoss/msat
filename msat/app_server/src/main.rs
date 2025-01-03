///================================
///             main.rs
///  This file as well as others
///  were made by MateusDev and 
///  are licensed under MPL 2.0
///  or APACHE 2.0
///================================

// Global Imports
use core::str;
use std::{ 
    io::{
        Read, Write
    }, net::{
        TcpListener, TcpStream
    }, sync::Arc
};
use tokio::sync::{
    Mutex,
    MutexGuard
};
use rusqlite::{Connection as SQLite};
use chrono::{
    self, Datelike, Timelike
};
// Local Imports

use shared_components::{
    cli::{
        self, ERROR, SUCCESS
    }, config as ConfigFile, database as Database, password::get_password, split_string_by, types::*, CLEAR, LOCAL_IP, SQLITE_FLAGS, VERSION,
    vec_to_string
};

// Entry point
#[tokio::main]
async fn main() {
    if let Ok(_) = std::process::Command::new(CLEAR).status(){
        cli::print_dashboard();
    };
    
    let db = match Database::init(*SQLITE_FLAGS).await{
        Ok(v)  => {
            Arc::new(Mutex::new(v))
        }
        Err(_) => {
            println!("Error occured while initializing Database");
            std::process::exit(-1);
        }
    };
    if let Err(error) = db.lock().await.execute_batch("PRAGMA journal_mode = WAL"){
        cli::critical_error("Error executing batch command", error);
    };
    if let Err(error) = db.lock().await.busy_timeout(std::time::Duration::from_secs(4)){
        cli::critical_error("Error setting busy_timeout", error);
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
            if let Some(ip) = v.ip_addr{
                Some(ip)
            }
            else{
                None
            }
        }
        None => {None}
    };
    let listener : TcpListener = match TcpListener::bind
        (format!("{}:8888", ip_address.unwrap_or(*LOCAL_IP)))
        {
            Ok(v) => v,
            Err(e) => 
            {
                if let Some(v) = ip_address {
                    cli::critical_error(&format!("Error connecting to ip_address {}", v), e);
                }
                else{
                    cli::critical_error(
                    "data/config.toml doesn't contain any IP Address, like: `127.0.0.1`;
                    Server automatically used this address with port 8888, but it wasn't able to connect : {}",
                    e);
                }
            }
    };

    println!("Listening on {}:8888", ip_address.unwrap_or(*LOCAL_IP));
    start_listening(listener, db).await;
    println!("Shutdown?");
    std::process::exit(0);
}

async fn start_listening(listener: TcpListener, db: Arc<Mutex<SQLite>>){
    loop{
        for s in listener.incoming(){
            let (mut ip_address, mut port) = (*LOCAL_IP,0);
            if let Ok(stream) = s
            {
                if let Ok(socket_ip) = stream.local_addr()
                {
                    ip_address = socket_ip.ip();
                    port = socket_ip.port();
                };

                let shared_db = Arc::clone(&db);
                tokio::spawn(
                    async move{
                        if let Err(error) = handle_connection(stream, shared_db).await{
                            cli::print_error("Error occured while handling exception", error);
                        }
                        else{
                            cli::print_success(&format!("Successfully handled request from TCP Addr: {}:{}", ip_address, port))
                        };
                    }
                );
            }
            else{
                println!("{} TCPStream is None", ERROR);
            }
        }
    }
}

async fn handle_connection(mut stream: TcpStream, db: Arc<Mutex<SQLite>>) -> Result<(), ConnectionError>{
    let mut data_sent = [0u8; 2048];
    let len = if let Ok(len) = stream.read(&mut data_sent){
        len
    }
    else{
        return Err(ConnectionError::CannotRead);
    };
    let string = {
        if let Ok(str) = String::from_utf8(data_sent[0..len].to_vec()){
            str
        }
        else{
            String::from_utf8_lossy(&data_sent[0..len]).to_string()
        }
    };
    let parsed_req = {
        match parse_request(&string).await{
            Ok(v) => ParsedRequest::from(v),
            Err(e) => return Err(e)
        }
    };
    match parsed_req.request{
        Request::GET|Request::POST =>
        {
            if let None = parsed_req.password{
                return Err(ConnectionError::NoPassword);
            }
            let response = match get_response(parsed_req.clone(), db).await{
                Ok(v) => v,
                Err(e) => RequestError::to_response(e)
            };
            if let Err(_) = stream.write_all(response.as_bytes()){
                return Err(ConnectionError::WritingError);
            }
            else{
                cli::print_success("Handled Request");
            }
        }
        _ => {}
    }
    return Ok(());
}

async fn get_response(parsed_request: ParsedRequest, db: Arc<Mutex<SQLite>>) -> Result<String, RequestError>{
    match parsed_request.request
    {
        Request::GET => {
            match parsed_request.request_number{
                0 => {
                    return Ok("msat/200-OK&get=Server-is-working!".to_string());
                }
                1 => 
                {
                    // GET Lessons for this Day 
                    let teacher = match parsed_request.content[0].parse::<u16>(){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::ParseIntError(parsed_request.content[0].clone()));
                        } 
                    };
                    let date : u8 = chrono::Local::now().weekday() as u8 + 1u8;
                    let database = db.lock().await;
                    let mut prompt = match database.prepare("SELECT * FROM Lessons WHERE teacher_id = ?1 AND week_day = ?2;"){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::DatabaseError);
                        }
                    };
                    // class_id, classroom_id, subject_id, lesson_number
                    let product_iter = match prompt.query_map([teacher, date.into()], |row|{
                        Ok((
                            quick_match(row.get::<usize,u16>(1)), //class_id,
                            quick_match(row.get::<usize,u16>(5)), // classroom_id,
                            quick_match(row.get::<usize,u16>(4)), // subject_id,
                            quick_match(row.get::<usize,u16>(2)) //lesson_hour
                        ))
                    }){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::DatabaseError);
                        }
                    };
                    let mut lesson_vec : Vec<(u16, u16, u16, u16)> = vec![];
                    for result in product_iter
                    {
                        if let Ok(tuple) = result 
                        {
                            if let (Some(class_id), Some(classroom_id), Some(subject_id), Some(lesson_number)) = tuple 
                            {
                                lesson_vec.push((class_id, classroom_id, subject_id, lesson_number));
                            }
                        }
                    }

                    if lesson_vec.len() == 0{
                        return Ok("msat/204-No-Content".to_string());
                    }
                    else{
                        let mut final_response = "msat/200-OK/get=".to_string();
                        for (class_id, classroom_id, subject_id, lesson_num) in lesson_vec{
                            if final_response.ends_with("="){
                                final_response.push_str(&format!("{}+{}+{}+{}", class_id, classroom_id, subject_id, lesson_num));
                            }
                            else{
                                final_response.push_str(&format!("|{}+{}+{}+{}", class_id, classroom_id, subject_id, lesson_num));
                            }
                        }
                        return Ok(final_response);
                    }
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
                    let stmt = { 
                        if let Ok(v) = query.query_row([&current_time_hhmm, &current_time_hhmm],|row| 
                        {
                            Ok((
                                quick_match(row.get::<usize, String>(1)),
                                quick_match(row.get::<usize, String>(2)),
                            ))
                        })
                        {
                            v
                        }
                        else{
                            return Err(RequestError::DatabaseError);
                        }
                    };
                    let (f_end, f_start) = match stmt{
                            (Some(start_time), Some(end_time)) => {
                                (end_time, start_time)
                            }
                            (None, None)|_ => {
                                return Err(RequestError::NoDataFoundError);
                            }
                    };
                    if f_end.is_empty()&f_start.is_empty() == false{
                        return Ok(format!("msat/200-OK&get={}+{}", f_start, f_end));
                    }
                    return Ok("msat/204-No-Content".to_string());
                }
                3 => {
                    // GET next break start_time and end_time | break_num == lesson_num
                    if let Ok(break_num) = str::parse::<u8>(&parsed_request.content[0]){
                        let database = db.lock().await;
                        let query = "SELECT * FROM BreakHours WHERE break_num = ?1";
                        if let Ok(mut stmt) = database.prepare(&query){
                            let iter = stmt.query_row([break_num], |row| {
                                Ok((
                                    row.get::<usize, u16>(1), //start_time
                                    row.get::<usize, u16>(2)  //end_time
                                ))
                            });
                            if let Ok(ok_iter) = iter{
                                if let (Ok(start_time), Ok(end_time)) = ok_iter{
                                    return Ok(format!("msat/200-OK/get={}+{}",start_time,end_time));
                                }
                                else{
                            return Ok("msat/204-No-Content".to_string());
                                }
                            } 
                        }
                        else{
                            return Err(RequestError::DatabaseError);
                        };
                    };
                    return Ok("msat/204-No-Content".to_string());
                }
                4 => {
                    // GET Request for getting if teacher will be on duty on following break:
                    // If false then program returns 200 OK false 
                    // If true then program checks WHERE does the teacher have duty and returns
                    // true
                    let teacher_id = match parsed_request.content[0].parse::<u16>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    let database = db.lock().await;
                    if let Ok((is_selected, break_num)) = get_teacher_duty_bool(teacher_id, Arc::clone(&db)).await{
                        if is_selected{
                            if let Ok(mut stmt) = database.prepare("SELECT duty_place FROM Duties WHERE teacher_id = ?1 
                            AND break_number = ?2 AND week_day = ?3")
                            {
                                if let Ok(Ok(value)) = 
                                stmt.query_row([teacher_id, break_num.into(), chrono::Local::now().weekday() as u16], |row| 
                                {
                                    Ok(row.get::<usize, u16>(0))
                                })
                                {

                                    if let Ok(mut stmt1) = database.prepare("SELECT duty_place_name FROM DutyPlaces WHERE 
                                    dutyplace_id = ?1"){
                                        if let Ok(Ok(duty_place_name)) = stmt1.query_row([value], |row| {Ok(row.get::<usize, String>(0))}){
                                            return Ok(format!("msat/200-OK&get=true+{duty_place_name}"));
                                        }
                                    }
                                }
                            }
                        }
                        else{
                            return Ok("msat/200-OK&get=false".to_string());
                        }
                    }
                    return Err(RequestError::DatabaseError);
                }
                5 => {
                    // GET current classroom && class (as String)
                    let teacher_id = match parsed_request.content[0].parse::<u16>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    let lesson_hour = match get_lesson_hour(Arc::clone(&db)).await{
                        Ok(v) => {
                            if v == 0{
                                match get_break_num(Arc::clone(&db)).await{
                                    Ok(v1) => v1,
                                    _ => {
                                        return Ok("msat/204-No-Content&get=no+num".to_string());
                                    }
                                }
                            }
                            else{
                                v
                            }
                        },
                        Err(_) => return Err(RequestError::DatabaseError)
                    };
                    let query ="SELECT * FROM Lessons 
                        WHERE week_day = ?1 AND lesson_hour = ?2 AND teacher_id = ?3;";
                    let database = db.lock().await;
                    let mut stmt = match database.prepare(&query){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::DatabaseError)
                    };
                    let iter = match stmt.query_map([(chrono::Local::now().weekday() as u8 + 1).into(), lesson_hour.into(), teacher_id], |row| {
                        Ok((
                                quick_match(row.get::<usize, u16>(5)),
                                quick_match(row.get::<usize, u16>(1))
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
                                    return Ok(format!("msat/200-OK&get={}+{}", u_class, u_classroom));
                                }
                                else{
                                    return Err(RequestError::NoDataFoundError);
                                }
                            }
                            Err(_) => return Err(RequestError::DatabaseError)
                        }
                    }
                    return Ok("msat/204-No-Content".to_string())
                }
                6 => {
                    // GET lesson hour 
                    match get_lesson_hour(db).await{
                        Ok(v) => {
                            if v == 0{
                                return Ok("msat/204-No-Content".to_string());
                            }
                            return Ok(format!("msat/200-OK&get={}", v))
                        },
                        Err(_) => return Err(RequestError::DatabaseError)
                    };
                }
                7 => {
                    // GET classroom by id
                    let id = match parsed_request.content[0].parse::<u16>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    match get_classroom(id, db).await{
                        Ok(v) => return Ok(format!("msat/200-OK&get={}", v)),
                        Err(_) => return Err(RequestError::DatabaseError)
                    }
                }
                8 => {
                    // GET class by id 
                    let id = match parsed_request.content[0].parse::<u16>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    match get_class(id, db).await{
                        Ok(v) => return Ok(format!("msat/200-OK&get={}", v)),
                        Err(_) => return Err(RequestError::DatabaseError)
                    }
                }
                9 => {
                    let id = match parsed_request.content[0].parse::<u16>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    match get_teacher(id, db).await{
                        Ok(v) => return Ok(format!("msat/200-OK&get={}", v)),
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
                    return Ok("msat/200-OK&post=Server-is-working!".to_string());
                }
                1 => {
                    // POST Lesson - contains class, classroom, subject, teacher, lesson number
                    let (class_id, classroom_id, subject_id, teacher_id, lesson_number, week_day) :
                        (Option<u16>, Option<u16>, Option<u16>, Option<u16>, Option<u16>, Option<u16>)= 
                    (
                        quick_match(str::parse::<u16>(&parsed_request.content[0])), 
                        quick_match(str::parse::<u16>(&parsed_request.content[1])), 
                        quick_match(str::parse::<u16>(&parsed_request.content[2])), 
                        quick_match(str::parse::<u16>(&parsed_request.content[3])), 
                        quick_match(str::parse::<u16>(&parsed_request.content[4])), 
                        quick_match(str::parse::<u16>(&parsed_request.content[5].trim()))
                    );
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
                                return Ok("msat/201-Created".to_string())
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
                    return Ok("msat/201-Created".to_string());
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
                    return Ok("msat/201-Created".to_string());
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
                    return Ok("msat/201-Created".to_string());
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
                    return Ok("msat/201-Created".to_string());
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
                        Ok(_) => return Ok("msat/201-Created".to_string()),
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
                        Ok(_) => {return Ok("msat/201-Created".to_string())},
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
{
    match statement{
        Ok(v) => return Some(v),
        Err(_) => return None
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
    let (mut request_type,mut content,mut password, mut request_num) : (Request,Vec<String>,Option<String>,u8) = 
    (Request::Other,vec![],None,0);
    let local_input = if input.ends_with("&"){
        input
    }
    else{
        &format!("{}&", input)
    };

    let mut version : u16 = 0;
    let sliced_input = split_string_by(local_input, '&');
    if input[0..4] != *"msat"{
        return Err(ConnectionError::WrongHeader);
    }
    for word in sliced_input{
        if word.starts_with("msat/"){
            if let Ok(v) = split_string_by(&word, '/')[1].trim().parse::<u16>(){
                version = v;
            };
        }
        if word.starts_with("method="){
            let split = split_string_by(&split_string_by(&word, '=')[1], '+');
            match split[0].as_str(){
                "GET" => {
                    request_type = Request::GET
                }
                "POST" => {
                    request_type = Request::POST
                }
                _ => {
                    return Err(ConnectionError::Other)
                }
            }
            if split.len() >= 2{
                if let Ok(val) = str::parse::<u8>(split[1].trim()){
                    request_num = val;
                }
                else{
                    return Err(ConnectionError::RequestParseError);
                }
            }
        }
        if word.starts_with("password="){
            if password.is_some(){
                continue;
            }
            password = {
                let split = split_string_by(&word, '=');
                if split.len() >= 2{
                    Some(split[1].clone())
                }
                else{
                    return Err(ConnectionError::NoPassword)
                }
            };
            if let (Some(p), Some(in_p)) = (get_password().await, &password){
                if &p != in_p || in_p.is_empty(){
                    return Err(ConnectionError::WrongPassword);
                }
            }
        }
        if word.starts_with("args="){
            if word.len() != 5{
                content = split_string_by(&split_string_by(&word, '=')[1], ',');
            }
        }
    }
    if version != VERSION {
        return Err(ConnectionError::WrongVersion);
    }
    if request_num == 0{
        return Err(ConnectionError::RequestParseError);
    }
    Ok((request_type,content,password,request_num))
}
async fn get_lesson_hour(db: Arc<Mutex<SQLite>>) -> Result<u8, ()>{
    let now = chrono::Local::now();
    let database = db.lock().await;
    let (month, day) : (u8, u8) = (match now.month().try_into() {Ok(v) => v,Err(_) => {return Err(())}},
    match now.day().try_into(){Ok(v)=>v,Err(_)=>{return Err(());}});
    let formatted = format_two_digit_time(month, day);
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
                            None => return Ok(0)
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

async fn get_break_num(db: Arc<Mutex<SQLite>>) -> Result<u8, ()>{
    let now = chrono::Local::now();
    let database = db.lock().await;
    let (month, day) : (u8, u8) = (match now.month().try_into() {Ok(v) => v,Err(_) => {return Err(())}},
    match now.day().try_into(){Ok(v)=>v,Err(_)=>{return Err(());}});
    let formatted = format_two_digit_time(month, day);
    let query = "SELECT * FROM BreakHours 
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
                            None => return Ok(0)
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

async fn get_teacher_duty_bool(teacher_id: u16,db: Arc<Mutex<SQLite>>) -> Result<(bool, u8), ()>{
    let database = db.lock().await;
    let lesson_hour = match get_lesson_hour(Arc::clone(&db)).await{
        Ok(v) => v,
        Err(_) => return Err(())
    };
    let mut stmt = match database.prepare("SELECT * FROM Duties 
        WHERE teacher_id = ?1 AND week_day = ?2 AND lesson_hour = ?3;"){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    let item = match stmt.query_row([teacher_id,(chrono::Local::now().weekday() as u8 + 1).into(),lesson_hour.into()], |row|{
    Ok(quick_match(row.get::<usize, u16>(1)))}){
        Ok(v) => v,
        Err(e) => {
            match e{
                rusqlite::Error::QueryReturnedNoRows => {
                    return Ok((false, lesson_hour));
                }
                _ => {
                    return Err(());
                }
            }
        }
    };
    match item{
        Some(v) => return Ok((v == teacher_id, lesson_hour)),
        None => return Err(())
    }
}
async fn get_classroom(id: u16,db: Arc<Mutex<SQLite>>) -> Result<String, ()>{
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
async fn get_class(id: u16, db: Arc<Mutex<SQLite>>) -> Result<String, ()>{
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
async fn get_teacher(id: u16, db: Arc<Mutex<SQLite>>) -> Result<String, ()>{
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

#[cfg(test)]
mod tests{
    use super::*;
    #[tokio::test]
    async fn parse_arg(){
        //              Correct
        let requests1 = "msat/10&method=GET+1&password=test&args=1,3,4,5";
        //              Wrong Version
        let requests2 = "msat/11&method=GET+1&password=test&args=1,3,4,5";
        //              Wrong Request
        let requests3 = "msat/10&method=GE+1&password=test&args=1,3,4,5";
        //              No Request Number
        let requests4 = "msat/10&method=GET+&password=test&args=1,3,4,5";
        //              Wrong Request Number
        let requests5 = "msat/10&method=GET+ab&password=test&args=1,3,4,5";
        //              No password
        let requests6 = "msat/10&method=GET+1&password=&args=1,3,4,5";
        //              Wrong Header
        let requests7 = "mast/10&method=GET+1&password=test&args=1,3,4,5";
        let (_, c, _, _) = parse_request(requests1).await.unwrap();
        assert_eq!(5, str::parse::<u16>(&c[3]).unwrap());
        println!("1. {:?}", parse_request(requests1).await);
        println!("2. {:?}", parse_request(requests2).await);
        println!("3. {:?}", parse_request(requests3).await);
        println!("4. {:?}", parse_request(requests4).await);
        println!("5. {:?}", parse_request(requests5).await);
        println!("6. {:?}", parse_request(requests6).await);
        println!("7. {:?}", parse_request(requests7).await);
    }
}
