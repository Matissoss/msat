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
    backend::{self, manipulate_database, MainpulationType, ParsedRequest, Request, RequestType}, consts::*, types::*, utils, visual
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
    match parsed_request.req_type{
        RequestType::GET => {
            match parsed_request.req_numb{
                1 => {
                    if let (Some(class_id), Some(weekday), Some(lesson_hour), Some(semester), Some(academic_year)) = 
                    (args.get("class_id"), args.get("weekday"), args.get("lesson_hour"), args.get("semester"), 
                     args.get("academic_year")) 
                    {
                        if let (Ok(class), Ok(weekd), Ok(lesson_hour), Ok(semester), Ok(academic_year)) =
                        (class_id.parse(), weekday.parse(), lesson_hour.parse(), semester.parse(), academic_year.parse())
                        {
                            match manipulate_database(
                                MainpulationType::Get(backend::GET::Lesson 
                                    { 
                                        class, 
                                        lesson_hour, 
                                        weekd, 
                                        semester, 
                                        academic_year
                                    }
                                ), &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    eprintln!("{error}");
                                    if error == rusqlite::Error::QueryReturnedNoRows{
                                        return Ok("msat/204-No-Content".to_string());
                                    }
                                    return Err(RequestError::DatabaseError);
                                }
                            }
                        }
                    }
                }
                2 => {
                    if let (Some(weekd_str), Some(break_num_str), Some(teacher_str), Some(semester_str), Some(year_str)) 
                    = (args.get("weekday"), args.get("break_num"), args.get("teacher_id"), args.get("semester"), args.get("academic_year")) 
                    {
                        if let (Ok(weekd), Ok(break_num), Ok(teacher_id), Ok(semester), Ok(academic_year))
                        = (weekd_str.parse::<u8>(), break_num_str.parse::<u8>(), teacher_str.parse::<u16>(), semester_str.parse::<u8>(), year_str.parse::<u8>())
                        {
                            match manipulate_database(MainpulationType::Get(backend::GET::Duty 
                                    { 
                                        weekd, 
                                        break_num, 
                                        teacher_id, 
                                        semester, 
                                        academic_year
                                    }), &*db.lock().await)
                            {
                                Ok(v) => return Ok(format!("{}&has_break=true", v)),
                                Err(error) => {
                                    if error == rusqlite::Error::QueryReturnedNoRows{
                                        return Ok("msat/200-OK&has_break=false".to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        RequestType::POST => {
            match parsed_request.req_numb{
                // Lesson
                1 => {
                    if let (Some(weekday_str), Some(classid_str), Some(classroomid_str), Some(teacherid_str), Some(subjectid_str), Some(semester_str),
                        Some(academicyear_str), Some(lessonhour_str)) = (args.get("weekday"), args.get("class_id"), args.get("classroom_id"), args.get("teacher_id"), 
                            args.get("subject_id"), args.get("semester"), args.get("academic_year"), args.get("lesson_hour"))
                    {
                        if let (Ok(weekday), Ok(class_id), Ok(classroom_id), Ok(teacher_id), Ok(subject_id), Ok(semester), Ok(academic_year),
                            Ok(lesson_hour)) = 
                        (weekday_str.parse::<u8>(),classid_str.parse::<u16>(),classroomid_str.parse::<u16>(),teacherid_str.parse::<u16>(),
                        subjectid_str.parse::<u16>(), semester_str.parse::<u8>(), academicyear_str.parse::<u8>(), lessonhour_str.parse::<u16>())
                        {
                            match manipulate_database(
                            MainpulationType::Insert
                            (backend::POST::Lesson(Some((weekday, class_id, classroom_id, teacher_id, subject_id, lesson_hour, semester, academic_year)))), &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
                                    return Err(RequestError::DatabaseError);
                                }
                            }
                        }
                    }
                }
                // Year
                2 => {
                    if let (Some(academicyear_str), Some(yearname_str), Some(startdate_str), Some(enddate_str)) = 
                    (args.get("academic_year"), args.get("year_name"), args.get("start_date"), args.get("end_date")){
                        if let Ok(academic_year) = academicyear_str.parse::<u8>(){
                            match manipulate_database(MainpulationType::Insert(backend::POST::Year(Some(
                            (academic_year, yearname_str.to_string(), startdate_str.to_string(), enddate_str.to_string()))))
                                ,&*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
                                    return Err(RequestError::DatabaseError)
                                }
                            };
                        }
                    }
                }
                // Duty
                3 => {
                    if let (Some(weekday_str), Some(breaknum_str), Some(teacherid_str), Some(semester_str), Some(academicyear_str), Some(placeid_str)) = 
                    (args.get("weekday"), args.get("break_num"), args.get("teacher_id"), args.get("semester"), args.get("academic_year"), args.get("place_id"))
                    {
                        if let (Ok(weekday), Ok(break_num), Ok(teacher_id), Ok(semester), Ok(academic_year), Ok(place_id)) = 
                        (weekday_str.parse::<u8>(),breaknum_str.parse::<u8>(),teacherid_str.parse::<u16>(),semester_str.parse::<u8>(), academicyear_str.parse::<u8>(),
                         placeid_str.parse::<u16>())
                        {
                            match manipulate_database(MainpulationType::Insert
                                (backend::POST::Duty(Some((weekday, break_num, teacher_id, place_id, semester, academic_year)))), 
                                &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
                                    return Err(RequestError::DatabaseError);
                                }
                            }
                        }
                    }
                }
                // Break
                4 => {
                    if let (Some(breaknum_str), Some(starthour_str), Some(startminute_str), Some(endhour_str), Some(endminute_str)) = 
                    (args.get("break_num"), args.get("start_hour"), args.get("start_minute"), args.get("end_hour"), args.get("end_minute"))
                    {
                        if let (Ok(break_num), Ok(start_hour), Ok(start_minute), Ok(end_hour), Ok(end_minute)) = 
                        (breaknum_str.parse::<u8>(),starthour_str.parse::<u8>(),startminute_str.parse::<u8>(),
                         endhour_str.parse::<u8>(),endminute_str.parse::<u8>())
                        {
                            match manipulate_database(MainpulationType::Insert(
                            backend::POST::Break(Some((break_num, start_hour, start_minute, end_hour, end_minute))))
                                , &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
                                    return Err(RequestError::DatabaseError);
                                }
                            }
                        }
                    }
                }
                // Semester
                5 => {
                    if let (Some(semester_str), Some(semester_name), Some(start_date), Some(end_date)) = 
                    (args.get("semester"),args.get("semester_name"),args.get("start_date"),args.get("end_date"))
                    {
                        if let Ok(semester) = semester_str.parse::<u8>(){
                            match manipulate_database(MainpulationType::Insert(
                                    backend::POST::Semester(
                                        Some((semester, semester_name.to_string(), start_date.to_string(), end_date.to_string())))), &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(err) => {
                                    visual::error(Some(err), "Database Error");
                                    return Err(RequestError::DatabaseError);
                                }
                            }
                        }
                    }
                }
                // LessonHour 
                6 => {
                    if let (Some(lessonnum_str), Some(starthour_str), Some(startminute_str), Some(endhour_str), Some(endminute_str)) = 
                    (args.get("lesson_num"), args.get("start_hour"), args.get("start_minute"), args.get("end_hour"), args.get("end_minute"))
                    {
                        if let (Ok(lesson_num), Ok(start_hour), Ok(start_minute), Ok(end_hour), Ok(end_minute)) = 
                        (lessonnum_str.parse::<u16>(),starthour_str.parse::<u8>(),startminute_str.parse::<u8>(),
                         endhour_str.parse::<u8>(),endminute_str.parse::<u8>())
                        {
                            match manipulate_database(MainpulationType::Insert(
                            backend::POST::LessonHours(Some((lesson_num, start_hour, start_minute, end_hour, end_minute))))
                                , &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
                                    return Err(RequestError::DatabaseError);
                                }
                            }
                        }
                    }
                }
                // Teacher
                7 => {
                    if let (Some(teacherid_str), Some(teacher_name)) = (args.get("teacher_id"), args.get("teacher_name")){
                        if let Ok(teacher_id) = teacherid_str.parse::<u16>(){
                            match manipulate_database(
                                MainpulationType::Insert(backend::POST::Teacher(Some ((teacher_id, teacher_name.to_string())) )), &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
                                }
                            }
                        }
                    }
                }
                // Class
                8 => {
                    if let (Some(teacherid_str), Some(teacher_name)) = (args.get("class_id"), args.get("class_name")){
                        if let Ok(teacher_id) = teacherid_str.parse::<u16>(){
                            match manipulate_database(
                                MainpulationType::Insert(backend::POST::Class(Some ((teacher_id, teacher_name.to_string())) )), &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
                                }
                            }
                        }
                    }
                }
                // Classroom
                9 => {
                    if let (Some(teacherid_str), Some(teacher_name)) = (args.get("classroom_id"), args.get("classroom_name")){
                        if let Ok(teacher_id) = teacherid_str.parse::<u16>(){
                            match manipulate_database(
                                MainpulationType::Insert(backend::POST::Classroom(Some ((teacher_id, teacher_name.to_string())) )), &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
                                }
                            }
                        }
                    }
                }
                // Subject
                10 => {
                    if let (Some(teacherid_str), Some(teacher_name)) = (args.get("subject_id"), args.get("subject_name")){
                        if let Ok(teacher_id) = teacherid_str.parse::<u16>(){
                            match manipulate_database(
                                MainpulationType::Insert(backend::POST::Subject(Some ((teacher_id, teacher_name.to_string())) )), &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
                                }
                            }
                        }
                    }
                }
                11 => {
                    if let (Some(teacherid_str), Some(teacher_name)) = (args.get("place_id"), args.get("place_name")){
                        if let Ok(teacher_id) = teacherid_str.parse::<u16>(){
                            match manipulate_database(
                                MainpulationType::Insert(backend::POST::Corridors(Some ((teacher_id, teacher_name.to_string())) )), &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    return Err(RequestError::UnknownRequestError);
}
