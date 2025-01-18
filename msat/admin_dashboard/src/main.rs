///====================================
///         admin_dashboard
/// This file is responsible for http server
/// made from scratch with TCP protocol
///=========================================

// Global imports
use std::{
    io::{
        Read, 
        Write
    }, 
    net::{
        IpAddr, 
        TcpListener, 
        TcpStream
    }, 
    sync::Arc
};
use tokio::sync::{Mutex, Semaphore};
use rusqlite;

// Local Imports 
use shared_components::{
    backend::{self, Request}, consts::*, types::*, visual
};
mod web;

#[tokio::main]
#[allow(warnings)]
async fn main(){
    visual::main();
    init_httpserver().await;
}

pub async fn init_httpserver() {
    let database = Arc::new(Mutex::new(
            match backend::init_db(){
                Ok(v) => v,
                Err(_) => visual::critical_error::<u8>(None, "Error occured while initializing database")
            }
    ));

    let (ip, port, max_limit, max_timeout, lang) : (IpAddr, u16, u16, Arc<u64>, Arc<Language>) = match backend::get_config().await{
        Some(c) => {
                (c.http_server.tcp_ip.unwrap_or(*LOCAL_IP), 
                 c.http_server.http_port, c.http_server.max_connections,
                 Arc::new(c.http_server.max_timeout_seconds.into()),
                 Arc::new(c.language))
        }
        None => {
            (*LOCAL_IP, 8000, 100, Arc::new(10), Arc::new(Language::default()))
        }
    };
    let limit = Arc::new(Semaphore::new(max_limit.into()));
    let final_address = format!("{}:{}", ip.to_string(), port);
    let listener: TcpListener = match TcpListener::bind(final_address) {
        Ok(v) => v,
        Err(_) => std::process::exit(-1),
    };
    visual::success("Initialized HTTP Server");
    loop {
        for s in listener.incoming() {
            visual::debug("Request Incoming");
            if let Ok(stream) = s{
                let cloned_dbptr = Arc::clone(&database);
                let cloned_permit = Arc::clone(&limit);
                let cloned_timeout = Arc::clone(&max_timeout);
                if let Ok(_) = tokio::time::timeout(std::time::Duration::from_secs(*cloned_timeout), 
                    cloned_permit.acquire_owned()).await{
                    let lang = Arc::clone(&lang);
                    tokio::spawn(async move {
                        handle_connection(stream, cloned_dbptr, Arc::clone(&lang)).await;
                    });
                }
            }
            else if let Err(error) = s{
                visual::error(Some(error), "TCPStream is Err");
            }
        }
    }
}
pub async fn handle_connection(mut stream: TcpStream, db_ptr: Arc<Mutex<rusqlite::Connection>>, lang: Arc<Language>) {
    let mut buffer = [0u8; 2048];
    if let Ok(len) = stream.read(&mut buffer) {
        if len == 0 {
        } else {
            let request = String::from_utf8_lossy(&buffer[0..len]).to_string();
            for l in request.lines(){
                if !l.is_empty()
                {
                    visual::debug(l);
                }
            }
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
                            else if w.starts_with("/?msat") && !w.starts_with("/?lang="){
                                let cloned_dbptr = Arc::clone(&db_ptr);
                                let cloned_lang = Arc::clone(&lang);
                                let response = handle_custom_request(&w, cloned_dbptr, cloned_lang).await;
                                match stream.write_all(
                                    format!("HTTP/1.1 200 OK\r\nContent-Length:{}\r\nContent-Type: application/xml\r\n\r\n{}",
                                        response.len(), response).as_bytes())
                                {
                                    Ok(_) =>  visual::info("Handled Request"),
                                    Err(_) => visual::info("Couldn't Handle Request")
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
                        types = get_types(w);
                    }
                }
            }
            if types.len() == 0 {
                types = vec!["*/*".to_string()];
            }
            // End of checks
            let binary: bool = if types[0].starts_with("image") || types[0].starts_with("font") ||
            file_path.ends_with(".ttf"){
                true
            }
            else{
                false
            };
            let f_type = &types[0];
            visual::debug(&format!("file_path = {}", file_path));
            if binary == false {
                if let Ok(buf) = tokio::fs::read(&file_path).await {
                    if let Ok(string) = String::from_utf8(buf.clone()) {
                        stream.write_all(
                        format!("HTTP/1.1 200 OK\r\n{}Content-Length:{}\r\nContent-Type:{}\r\n\r\n{}",
                            if file_path.as_str() == "./web/index.html"{
                            "Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'\r\n"
                            }else{""},string.len(), f_type, string)
                        .as_bytes())
                        .unwrap();
                    } else {
                        let string = String::from_utf8_lossy(&buf).to_string();
                        stream.write_all(
                        format!("HTTP/1.1 200 OK\r\nContent-Length:{}\r\nContent-Type:{}\r\n\r\n{}",
                            string.len(), f_type, string)
                        .as_bytes())
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
async fn handle_custom_request(request: &str, _db: Arc<Mutex<rusqlite::Connection>>, lang: Arc<Language>) -> String{
    // request example: /?msat/version&method=POST+1&version=10&args=20
    
    let _request_parsed = match Request::from_str(request).parse(){
        Ok(v) => v,
        Err(_) => {
            return lang.english_or("<error><p>Server couldn't parse request</p></error>", 
                "<error><p>Serwer nie mógł przetworzyć zapytania</p></error>");
        }
    };

    if *lang == Language::Polish{
        return "<error><p>Nie byliśmy w stanie zdobyć żadnych informacji</p></error>".to_string();
    }
    else{
        return "<error><p>We coudln't get any data from server</p></error>".to_string();
    }
}

fn not_found(tcp: &mut TcpStream) {
    if let Err(error) = tcp.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n<h1>404 - Not Found</h1>"){
        visual::error(Some(error), "Error Occured while sending 404 to client");
    }
    else{
        visual::debug("Returned 404 to Client");
    }
}

#[allow(dead_code)]
fn get_types(line: String) -> Vec<String> {
    let split_line = line.split_whitespace().collect::<Vec<&str>>();
    let mut types: Vec<String> = vec![];
    for s in split_line {
        if !s.starts_with("Accept:") {
            types = s.split(',')
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
        }
    }
    types
}

#[allow(dead_code)]
fn database_insert_success_msg(lang: &Language) -> String{
    return if lang == &Language::Polish{
        "<success><p>Pomyślnie dodano dane do bazy danych</p></success>".to_string()
    }
    else{
        "<success><p>Successfully added data to database</p></success>".to_string()
    };
}

#[allow(dead_code)]
fn database_insert_error_msg(lang: &Language) -> String{
    return if lang == &Language::Polish{
        "<error><p>Wystąpił błąd podczas dodawania danych do bazy danych, sprawdź czy zapytanie jest poprawne, 
            a w ostateczności skontaktuj się z administratorem</p></error>".to_string()
    }
    else{
        "<error><p>Error occured while adding data to database, check if request is correct and if it is, then ask admin</p></error>".to_string()
    };
}
pub fn weekd_to_string(lang: &Language, weekd: u8) -> String{
    match weekd{
        1 => lang.english_or("Monday"   ,"Poniedziałek" ),
        2 => lang.english_or("Tuesday"  ,"Wtorek"       ),
        3 => lang.english_or("Wednesday","Środa"        ),
        4 => lang.english_or("Thursday" ,"Czwartek"     ),
        5 => lang.english_or("Friday"   ,"Piątek"       ),
        6 => lang.english_or("Saturday" ,"Sobota"       ),
        7 => lang.english_or("Sunday"   ,"Niedziela"    ),
        _ => lang.english_or("Unknown"  ,"Nieznany"),
    }
}
