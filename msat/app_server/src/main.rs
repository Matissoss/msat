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
    match parsed_request.req_type
    {
        RequestType::GET => {
            match parsed_request.req_numb{
                0 => {
                    return Ok("msat/200-OK&get=Server-is-working".to_string());
                }
                1 => {
                    if let Some(teacher_id_in_str) = args.get("teacher_id"){
                        if let Ok(teacher_id) = teacher_id_in_str.parse::<u16>(){
                            if let Ok(lessons) = backend::get_lessons_by_teacher_id(teacher_id, &*db.lock().await){
                                let mut to_return = "msat/200-OK".to_string();
                                let mut largest = 0u16;
                                for lesson in lessons{
                                    if let Some(lessonh) = lesson.lessonh.lesson_hour{
                                        if largest < lessonh{
                                            largest = lessonh;
                                        }
                                        if let Some(class) = lesson.class{
                                            to_return.push_str(&format!("&class{}={}", lessonh, class.to_single('_')));
                                        }
                                        if let Some(classroom) = lesson.classroom{
                                            to_return.push_str(&format!("&classroom{}={}", lessonh, classroom.to_single('_')));
                                        }
                                        if let Some(subject) = lesson.subject{
                                            to_return.push_str(&format!("&subject{}={}", lessonh, subject.to_single('_')))
                                        }
                                    }
                                }
                                to_return.push_str(&format!("&AMOUNT={}",largest));
                                return Ok(to_return);
                            }
                        }
                    }
                    else{
                        return Ok("msat/204-No-Content&get=None&AMOUNT=0".to_string());
                    }
                }
                2 => {
                    if let Some(teacher_id_in_str) = args.get("teacher_id"){
                        if let Ok(teacher_id) = teacher_id_in_str.parse::<u16>(){
                            let mut to_return = "msat/200-OK".to_string();
                            let mut amount = 0u16;
                            match backend::get_duties_for_teacher(teacher_id, &*db.lock().await){
                                Ok(vector) => {
                                    for duty in vector{
                                        if let Some(lessonh) = duty.break_num.lesson_hour{
                                            if amount < lessonh{
                                                amount = lessonh;
                                            }
                                            if let Some(place) = duty.place{
                                                to_return.push_str(&format!("&place{}={}", lessonh,  place.to_single('_')));
                                            }
                                            if let (Some(hour), Some(minute)) = (duty.break_num.start_hour, duty.break_num.start_minute){
                                                to_return.push_str(&format!("&start{}={}:{}",lessonh,hour,minute));
                                            }
                                            if let (Some(hour), Some(minute)) = (duty.break_num.end_hour, duty.break_num.end_minutes){
                                                to_return.push_str(&format!("&end{}={}:{}", lessonh, hour, minute));
                                            }
                                        }
                                    }
                                    to_return.push_str(&format!("&AMOUNT={}", amount));
                                    return Ok(to_return);
                                }
                                Err(error) => {
                                    if error == rusqlite::Error::QueryReturnedNoRows{
                                        return Ok("msat/200-OK&get=None&AMOUNT=0".to_string());
                                    }
                                }
                            }
                        }
                    }
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
                _ => {
                    return Err(RequestError::UnknownRequestError);
                }
            }
        }
        _ => {}
    }
    Ok("msat/418-I'm-teapot".to_string())
}
