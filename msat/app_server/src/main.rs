///================================
///             main.rs
///  This file as well as others
///  were made by MateusDev and 
///  licensed under X11 (MIT) LICENSE
///================================

// Global Imports
use core::str;
use std::{ 
    io::{
        Read, Write
    }, 
    net::{
        TcpListener, TcpStream
    }, 
    sync::Arc
};
use tokio::sync::{
    Mutex, 
    Semaphore
};
use rusqlite::Connection as SQLite;
use chrono::{
    self, Datelike, Timelike
};
use colored::Colorize;
// Local Imports

use shared_components::{
    backend::{self, Request, RequestType, ParsedRequest}, consts::*, types::*, utils, visual
};
use shared_components::utils::*;

// Entry point
#[tokio::main]
async fn main() {
    visual::main();
    if let Ok(_) = std::process::Command::new(CLEAR).status(){
    };
    
    let db = match backend::init_db(){
        Ok(v)  => {
            Arc::new(Mutex::new(v))
        }
        Err(_) => {
            println!("Error occured while initializing Database");
            std::process::exit(-1);
        }
    };
    let shared_config = Arc::new(match backend::get_config().await{
        Some(v) => Some(v),
        None => {
            visual::info("Config was not found! Create configuration file!");
            None
        }
    });
    let (ip_address, port, max_connections, max_timeout) = match &*shared_config{
        Some(v) => {
            if let (Some(ip), port, max, maxt) 
                = (v.application_server.tcp_ip, v.application_server.port,v.application_server.max_connections,
                    v.application_server.max_timeout_seconds){
                (Some(ip), port, max, Arc::new(maxt.into()))
            }
            else{
                (None, 8888, 100, Arc::new(6))
    }
        }
        None => {(None, 8888, 100, Arc::new(6))}
    };
    let limit = Arc::new(Semaphore::new(max_connections.into()));
    let public_ip = match utils::get_public_ip(){
        Ok(ip) => ip,
        Err(_) => *LOCAL_IP
    };
    if let Ok(invite_code) = utils::encode_ip(public_ip, port){
        if ARGS.contains(&"--color".to_string()){
            visual::info(&format!("This is your public ip: {}", public_ip.to_string().on_black().white().bold()));
            visual::info(&format!("This Code should be entered by clients: {}", invite_code.yellow().on_black().bold()));
        }
        else{
            visual::info(&format!("This is your public ip: {}", public_ip.to_string()));
            visual::info(&format!("This Code should be entered by clients: {}", invite_code));
        }
        if let Err(error) = tokio::fs::write("data/invite.code", invite_code).await{
            visual::error(Some(error), "Error occured while saving to file 'data/invite.code'");
        }
        else{
            visual::success("Successfully saved to data/invite.code");
        }
    }

    let listener : TcpListener = match TcpListener::bind
        (format!("{}:{}", ip_address.unwrap_or(*LOCAL_IP), port))
        {
            Ok(v) => v,
            Err(e) => 
            {
                if let Some(v) = ip_address {
                    visual::critical_error(Some(e), &format!("Error connecting to ip_address {}", v));
                }
                else{
                    visual::critical_error(Some(e), "data/config.toml doesn't contain any IP Address, like: `127.0.0.1`;");
                }
            }
    };
    

    visual::debug(&format!("Listening on {}:8888", ip_address.unwrap_or(*LOCAL_IP)));
    
    // Start of actual program
    start_listening(listener, db, limit, max_timeout).await;
    

    visual::debug("Shutdown?");
    std::process::exit(0);
}

async fn start_listening(listener: TcpListener, db: Arc<Mutex<SQLite>>, limit: Arc<Semaphore>, timeout: Arc<u64>){
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
                let cloned_timeout = Arc::clone(&timeout);
                let cloned_limit = Arc::clone(&limit);
                if let Ok(_) = tokio::time::timeout(std::time::Duration::from_secs(*cloned_timeout), cloned_limit.acquire_owned()).await{
                let shared_db = Arc::clone(&db);
                    tokio::spawn(
                        async move{
                            if let Err(error) = handle_connection(stream, shared_db).await{
                                visual::error(Some(error), "Error occured while handling exception");
                            }
                            else{
                                visual::success(&format!("Successfully handled request from TCP Addr: {}:{}", ip_address, port))
                            };
                        }
                    );
                }
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

    if let Ok(request) = Request::from_str(&String::from_utf8_lossy(&data_sent[..len]).to_string()).parse(){
        let response = match get_response(request, db).await{
            Ok(v) => v,
            Err(e) => RequestError::to_response(e)
        };
        if let Err(_) = stream.write_all(response.as_bytes()){
            return Err(ConnectionError::WritingError);
        }
        else{
            visual::success("Handled Request");
        }
    }
    return Ok(());
}

async fn get_response(parsed_request: ParsedRequest, db: Arc<Mutex<SQLite>>) -> Result<String, RequestError>{
    let args = &parsed_request.args;
    let empty = &"".to_string();
    match parsed_request.req_type
    {
        RequestType::GET => {
            match parsed_request.req_numb{
                0 => {
                    return Ok("msat/200-OK&get=Server-is-working!".to_string());
                }
                1 => 
                {
                    // GET Lessons for this day 
                    let teacher_id = if let Some(v) = args.get("arg1"){
                        v
                    }
                    else{
                        return Err(RequestError::LengthError);
                    };

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
                    if let Some(break_num) = args.get("arg1"){
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
                    /*
                    let teacher_id = match args.get("arg1"){
                        Some(v) => {
                            if let Ok(v1) = v.parse(){
                                v1
                            }
                            else{
                                return Err(RequestError::ParseIntError(args.get("arg1").unwrap().to_string()))
                            }
                        },
                        None => return Err(RequestError::LengthError)
                    };
                    let database = db.lock().await;
                    if let Ok(is_selected) = Database::get_teacher_duty_bool(chrono::Local::now().weekday() as u8, teacher_id, &database){
                        if is_selected{
                            if let Ok(mut stmt) = database.prepare("SELECT duty_place FROM Duties WHERE teacher_id = ?1 
                            AND break_number = ?2 AND week_day = ?3")
                            {
                                let break_num = Database::get_break_num(&database);
                                if let Ok(Ok(value)) = 
                                stmt.query_row([teacher_id, break_num.unwrap_or(0).into(), chrono::Local::now().weekday() as u16], |row| 
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
                    */
                }
                5 => {
                    // GET current classroom && class (as String)
                    /*
                    let teacher_id = if let Some(v) = args.get("arg1"){
                        if let Ok(v1) = v.parse::<u16>(){
                            v1
                        }
                        else{
                            return Err(RequestError::ParseIntError(args.get("arg1").unwrap().to_string()));
                        }
                    }else{
                        return Err(RequestError::LengthError);
                    };
                    let database = db.lock().await;
                    let lesson_hour = match Database::get_lesson_hour(&database){
                        Ok(v) => {
                            if v == 0{
                                match Database::get_break_num(&database){
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
                                    let (classroom, class) = (
                                        Database::get_classroom(u_classroom, &database),
                                        Database::get_class(u_class, &database)
                                    );
                                    return Ok(format!("msat/200-OK&get={}+{}", 
                                            class    .unwrap_or(u_class    .to_string()), 
                                            classroom.unwrap_or(u_classroom.to_string())
                                    ));
                                }
                                else{
                                    return Err(RequestError::NoDataFoundError);
                                }
                            }
                            Err(_) => return Err(RequestError::DatabaseError)
                        }
                    }
                    return Ok("msat/204-No-Content".to_string())
                    */
                }
                6 => {
                    // GET lesson hour 
                    /*
                    match Database::get_lesson_hour(&db.lock().await){
                        Ok(v) => {
                            if v == 0{
                                return Ok("msat/204-No-Content".to_string());
                            }
                            return Ok(format!("msat/200-OK&get={}", v))
                        },
                        Err(_) => return Err(RequestError::DatabaseError)
                    };
                    */
                }
                7 => {
                    // GET classroom by id
                    /*
                    if let Some(v) = args.get("arg1"){
                        if let Ok(id) = v.parse(){
                            match Database::get_classroom(id, &db.lock().await){
                                Ok(v) => return Ok(format!("msat/200-OK&get={}", v)),
                                Err(_) => return Err(RequestError::DatabaseError)
                            }
                        }
                    }
                    */
                }
                8 => {
                    // GET class by id 
                    /*
                    if let Some(v) = args.get("arg1"){
                        if let Ok(id) = v.parse(){
                            match Database::get_class(id, &db.lock().await){
                                Ok(v) => return Ok(format!("msat/200-OK&get={}", v)),
                                Err(_) => return Err(RequestError::DatabaseError)
                            }
                        }
                    }
                    */
                }
                9 => {
                    /*
                    if let Some(v) = args.get("arg1"){
                        if let Ok(id) = v.parse(){
                            match Database::get_teacher(id, &db.lock().await){
                                Ok(v) => return Ok(format!("msat/200-OK&get={}", v)),
                                Err(_) => return Err(RequestError::DatabaseError)
                            }
                        }
                    }
                    */
                }
                10 => {
                    /*
                    match Database::get_break_num(&db.lock().await){
                        Ok(v) => {
                            if v == 0 {
                                return Ok("msat/204-No-Content&get=Not-a-Break".to_string());
                            }
                            else{
                                return Ok(format!("msat/200-OK&get={v}"));
                            }
                        }
                        Err(_) => {
                            return Err(RequestError::DatabaseError);
                        }
                    }
                    */
                }
                _ => {
                    return Err(RequestError::UnknownRequestError);
                }
            };
        }
        RequestType::POST => {
            match parsed_request.req_numb {
                0 => {
                    return Ok("msat/200-OK&post=Server-is-working!".to_string());
                }
                1 => {
                    // POST Lesson - contains class, classroom, subject, teacher, lesson number
                    let (class_id, classroom_id, subject_id, teacher_id, lesson_number, week_day) :
                        (Option<u16>, Option<u16>, Option<u16>, Option<u16>, Option<u16>, Option<u16>)= 
                    (
                        quick_match(args.get("arg1").unwrap_or(&"".to_string()).parse::<u16>()), 
                        quick_match(args.get("arg2").unwrap_or(&"".to_string()).parse::<u16>()), 
                        quick_match(args.get("arg3").unwrap_or(&"".to_string()).parse::<u16>()), 
                        quick_match(args.get("arg4").unwrap_or(&"".to_string()).parse::<u16>()), 
                        quick_match(args.get("arg5").unwrap_or(&"".to_string()).parse::<u16>()), 
                        quick_match(args.get("arg6").unwrap_or(&"".to_string()).parse::<u16>()), 
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
                        return Err(RequestError::LengthError)
                    }
                }
                2 => {
                    // POST Teacher - contains ID and full name
                    if let (Some(id), Some(name), Some(last_name)) = (args.get("arg1"), args.get("arg2"), args.get("arg3")){
                    let database = db.lock().await;
                        match database.execute("INSERT INTO Teachers (teacher_id, first_name, last_name) VALUES (?1, ?2, ?3)
                            ON CONFLICT (teacher_id) DO UPDATE SET first_name = excluded.first_name, last_name = excluded.last_name;", 
                            [id.to_string().as_str(), name, last_name]){
                            Ok(_) => {},
                            Err(_) => return Err(RequestError::DatabaseError)
                        };
                        return Ok("msat/201-Created".to_string());
                    }
                }
                3 => {
                    // POST Hours - contains start hour, lesson number and end number
                    let lesson_num : u8 = match args.get("arg1").unwrap_or(&"".to_string()).parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::LengthError);
                        }
                    };
                    let (s_hour, s_minute) = match format_mmdd(args.get("arg2").unwrap_or(&"".to_string())){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::LengthError);
                        }
                    };
                    let (e_hour, e_minute) = match format_mmdd(args.get("arg3").unwrap_or(&"".to_string())){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::LengthError);
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
                            Err(_) => {
                                return Err(RequestError::DatabaseError);
                            }
                        };
                    return Ok("msat/201-Created".to_string());
                }
                4 => {
                    // POST Subjects - contains id and name
                    let id = match args.get("arg1").unwrap_or(&"".to_string()).parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::LengthError)
                    };
                    let name;
                    match args.get("arg2").unwrap_or(&"".to_string()).as_str(){
                        "" => return Err(RequestError::UnknownRequestError),
                        _ => {name = args.get("arg2").unwrap()}
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
                    let id = match args.get("arg1").unwrap_or(&"".to_string()).parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::LengthError)
                    };
                    let name;
                    match args.get("arg2").unwrap_or(&"".to_string()).as_str(){
                        "" => return Err(RequestError::LengthError),
                        _ => {name = args.get("arg2").unwrap_or(empty).as_str()}
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
                    let teacher_id = match args.get("arg1").unwrap_or(&"".to_string()).parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::LengthError)
                    };
                    let weekday = match args.get("arg2").unwrap_or(&"".to_string()).parse::<u8>(){
                        Ok(v) => {
                            if v <= 7 && v > 0{
                                v
                            }
                            else{
                                return Err(RequestError::LengthError);
                            }
                        }
                        Err(_) => return Err(RequestError::LengthError)
                    };
                    let lesson_number = match args.get("arg3").unwrap_or(&"".to_string()).parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::LengthError)
                    };
                    let break_place = match args.get("arg4").unwrap_or(&"".to_string()).as_str(){
                        ""  => {
                            return Err(RequestError::LengthError);
                        },
                        _ => {
                            args.get("arg4").unwrap_or(empty)
                        }
                    };
                    let database = db.lock().await;
                    match database.execute("INSERT INTO Duties (break_num, teacher_id, duty_place, week_day,) 
                    VALUES (?1, ?2, ?3, ?4)
                        ON CONFLICT (lesson_hour, teacher_id, week_day) DO UPDATE SET classroom_id = excluded.classroom_id", 
                        &[lesson_number.to_string().as_str(), teacher_id.to_string().as_str(), 
                        break_place.to_string().as_str(), weekday.to_string().as_str()]){
                        Ok(_) => return Ok("msat/201-Created".to_string()),
                        Err(_) => {
                            return Err(RequestError::DatabaseError)
                        }
                    }
                }
                7 => {
                    // POST Classes - contains class number (UNIQUE!) and name
                    let id = match args.get("arg1").unwrap_or(&"".to_string()).parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::LengthError)
                    };
                    let name;
                    match args.get("arg2").unwrap_or(&"".to_string()).as_str(){
                        "" => return Err(RequestError::UnknownRequestError),
                        _ => {name = args.get("arg2").unwrap_or(empty).as_str()}
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
                8 => {
                    // POST BreakHours - break_num, start_time, end_time
                    if let (Ok(break_num), Ok(start_time), Ok(end_time)) = 
                    (str::parse::<u8>(&args.get("arg1").unwrap_or(&"".to_string())), 
                     str::parse::<u16>(&args.get("arg2").unwrap_or(&"".to_string())), 
                     str::parse::<u16>(&args.get("arg3").unwrap_or(&"".to_string()))){
                        let query = "INSERT INTO BreakHours (break_num, start_time, end_time) 
                        VALUES (?1, ?2, ?3) ON CONFLICT (break_num) DO UPDATE SET 
                        start_time = excluded.start_time, end_time = excluded.end_time";
                        if let Err(_) = db.lock().await.execute(&query, [break_num.into(), start_time, end_time]){
                            return Err(RequestError::DatabaseError);
                        }
                        else{
                            return Ok("msat/201-Created".to_string());
                        }
                    }
                }
                _ => {
                    return Err(RequestError::UnknownRequestError);
                }
            }
        }
        _ => {}
    }
    Ok("msat/418-I'm-teapot".to_string())
}
