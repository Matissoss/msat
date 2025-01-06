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
    }, net::{
        TcpListener, TcpStream
    }, sync::Arc,
};
use tokio::sync::{Mutex, Semaphore};
use rusqlite::Connection as SQLite;
use chrono::{
    self, Datelike, Timelike
};
use colored::Colorize;
// Local Imports

use shared_components::{
    cli::{
        self, ERROR, ARGS
    }, config as ConfigFile, database as Database, password::get_password, split_string_by, types::*,
    CLEAR, LOCAL_IP, SQLITE_FLAGS, VERSION,
    utils
};
use shared_components::utils::*;

// Entry point
#[tokio::main]
async fn main() {
    cli::main();
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
            cli::print_errwithout("Error getting configuration"); 
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
            cli::print_info(&format!("This is your public ip: {}", public_ip.to_string().on_black().white().bold()));
            cli::print_info(&format!("This Code should be entered by clients: {}", invite_code.yellow().on_black().bold()));
        }
        else{
            cli::print_info(&format!("This is your public ip: {}", public_ip.to_string()));
            cli::print_info(&format!("This Code should be entered by clients: {}", invite_code));
        }
        if let Err(error) = tokio::fs::write("data/invite.code", invite_code).await{
            cli::print_error("Error occured while saving to file 'data/invite.code'", error);
        }
        else{
            cli::print_success("Successfully saved to data/invite.code");
        }
    }

    let listener : TcpListener = match TcpListener::bind
        (format!("{}:{}", ip_address.unwrap_or(*LOCAL_IP), port))
        {
            Ok(v) => v,
            Err(e) => 
            {
                if let Some(v) = ip_address {
                    cli::critical_error(&format!("Error connecting to ip_address {}", v), e);
                }
                else{
                    cli::critical_error("data/config.toml doesn't contain any IP Address, like: `127.0.0.1`;",e);
                }
            }
    };
    

    cli::debug_log(&format!("Listening on {}:8888", ip_address.unwrap_or(*LOCAL_IP)));
    start_listening(listener, db, limit, max_timeout).await;
    cli::debug_log("Shutdown?");
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
                                cli::print_error("Error occured while handling exception", error);
                            }
                            else{
                                cli::print_success(&format!("Successfully handled request from TCP Addr: {}:{}", ip_address, port))
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
    let args = &parsed_request.content;
    match parsed_request.request
    {
        Request::GET => {
            match parsed_request.request_number{
                0 => {
                    return Ok("msat/200-OK&get=Server-is-working!".to_string());
                }
                1 => 
                {
                    // GET Lessons for this day 
                    if args.len() < 1{
                        return Ok(not_enough_arguments(args.len(), 1));
                    }
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
                        let (class_hashmap, classroom_hashmap, subject_hashmap) = 
                            (
                                Database::get_classes   (&database),
                                Database::get_classrooms(&database),
                                Database::get_subjects  (&database)
                            );
                        let mut final_response = "msat/200-OK/get=".to_string();
                        for (class_id, classroom_id, subject_id, lesson_num) in lesson_vec{
                            if !final_response.ends_with("="){
                                final_response.push('|');
                            }

                            final_response.push_str(&format!("{}+{}+{}+{}", 
                                    class_hashmap    .get(&class_id)    .unwrap_or(&class_id    .to_string()), 
                                    classroom_hashmap.get(&classroom_id).unwrap_or(&classroom_id.to_string()), 
                                    subject_hashmap  .get(&subject_id)  .unwrap_or(&subject_id  .to_string()), 
                                    lesson_num
                            ));
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
                    if args.len() < 1{
                        return Ok(not_enough_arguments(args.len(), 1));
                    }
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
                    if args.len() < 1{
                        return Ok(not_enough_arguments(args.len(), 1));
                    }
                    let teacher_id = match parsed_request.content[0].parse::<u16>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
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
                }
                5 => {
                    // GET current classroom && class (as String)
                    if args.len() < 1{
                        return Ok(not_enough_arguments(args.len(), 1));
                    }
                    let teacher_id = match parsed_request.content[0].parse::<u16>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
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
                }
                6 => {
                    // GET lesson hour 
                    match Database::get_lesson_hour(&db.lock().await){
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
                    if args.len() < 1{
                        return Ok(not_enough_arguments(args.len(), 1));
                    }
                    let id = match parsed_request.content[0].parse::<u16>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    match Database::get_classroom(id, &db.lock().await){
                        Ok(v) => return Ok(format!("msat/200-OK&get={}", v)),
                        Err(_) => return Err(RequestError::DatabaseError)
                    }
                }
                8 => {
                    // GET class by id 
                    if args.len() < 1{
                        return Ok(not_enough_arguments(args.len(), 1));
                    }
                    let id = match parsed_request.content[0].parse::<u16>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    match Database::get_class(id, &db.lock().await){
                        Ok(v) => return Ok(format!("msat/200-OK&get={}", v)),
                        Err(_) => return Err(RequestError::DatabaseError)
                    }
                }
                9 => {
                    if args.len() < 1{
                        return Ok(not_enough_arguments(args.len(), 1));
                    }
                    let id = match parsed_request.content[0].parse::<u16>(){
                        Ok(v) => v,
                        Err(_) => return Err(RequestError::ParseIntError(parsed_request.content[0].clone()))
                    };
                    match Database::get_teacher(id, &db.lock().await){
                        Ok(v) => return Ok(format!("msat/200-OK&get={}", v)),
                        Err(_) => return Err(RequestError::DatabaseError)
                    }
                }
                10 => {
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
                }

                // ADMIN Commands
                20 => {

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
                    if args.len() < 6{
                        return Ok(not_enough_arguments(args.len(), 6));
                    }
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
                    if args.len() < 3{
                        return Ok(not_enough_arguments(args.len(), 3));
                    }
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
                    if args.len() < 3{
                        return Ok(not_enough_arguments(args.len(), 3));
                    }
                    let content = &parsed_request.content;
                    let lesson_num : u8 = match content[0].parse::<u8>(){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::ParseIntError(content[0].clone()));
                        }
                    };
                    let (s_hour, s_minute) = match format_mmdd(&content[1]){
                        Ok(v) => v,
                        Err(_) => {
                            return Err(RequestError::ParseIntError(content[1].clone()));
                        }
                    };
                    let (e_hour, e_minute) = match format_mmdd(&content[2]){
                        Ok(v) => v,
                        Err(_) => {
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
                            Err(_) => {
                                return Err(RequestError::DatabaseError);
                            }
                        };
                    return Ok("msat/201-Created".to_string());
                }
                4 => {
                    // POST Subjects - contains id and name
                    if args.len() < 2{
                        return Ok(not_enough_arguments(args.len(), 2));
                    }
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
                    if args.len() < 2{
                        return Ok(not_enough_arguments(args.len(), 2));
                    }
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
                    if args.len() < 4{
                        return Ok(not_enough_arguments(args.len(), 4));
                    }
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
                    let break_place = match parsed_request.content[3].as_str(){
                        ""  => {
                            return Err(RequestError::UnknownRequestError);
                        },
                        _ => {
                            &parsed_request.content[3]
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
                    if args.len() < 2{
                        return Ok(not_enough_arguments(args.len(), 2));
                    }
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
                8 => {
                    // POST BreakHours - break_num, start_time, end_time
                    if args.len() < 3{
                        return Ok(not_enough_arguments(args.len(), 3));
                    }
                    if let (Ok(break_num), Ok(start_time), Ok(end_time)) = 
                    (str::parse::<u8>(&args[0]), str::parse::<u16>(&args[1]), str::parse::<u16>(&args[2])){
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
        Request::Other => {}
    }
    Ok("msat/418-I'm-teapot".to_string())
}

async fn parse_request(input: &str) -> Result<(Request,Vec<String>, Option<String>,u8), ConnectionError> {
    if input[0..4] != *"msat"{
        return Err(ConnectionError::WrongHeader);
    }
    let (mut request_type,mut content,mut password, mut request_num) : (Request,Vec<String>,Option<String>,u8) = 
    (Request::Other,vec![],None,0);
    let mut version : u16 = 0;
    let sliced_input = split_string_by(input, '&');
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

fn not_enough_arguments(found: usize, expected: u8) -> String{
    return format!("msat/400-Bad-Request&get=Expected+{expected}+args+found+{found}");
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
