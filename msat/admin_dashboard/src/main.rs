///====================================
///         admin_dashboard
/// This file is responsible for http server
/// made from scratch with TCP protocol
///=========================================

// Global imports
use std::{
    collections::BTreeMap, io::{Read, Write}, net::{IpAddr, TcpListener, TcpStream}, sync::Arc
};
use tokio::sync::{Mutex, Semaphore};
use rusqlite;

// Local Imports 
use shared_components::{
    backend::{self, Request, ParsedRequest}, consts::*, types::*, visual
};
use web::dashboard;
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
                            else if w.starts_with("/?") && !w.starts_with("/?lang="){
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
async fn handle_custom_request(request: &str, db: Arc<Mutex<rusqlite::Connection>>, lang: Arc<Language>) -> String{
    // request example: /?msat/version&method=POST+1&version=10&args=20
    let request = String::from_iter(request.chars().collect::<Vec<char>>()[2..].iter());
    let parsed_request : ParsedRequest = Request::from_str(&request).parse().unwrap_or_default();
    if parsed_request == ParsedRequest::default(){
        return "Error".to_string();
    }
    let args = parsed_request.args;
    let (method, method_num) = match args.get("method"){
        Some(value) => {
            if let Some((method, method_num)) = value.split_once('+'){
                if let Ok(method_num) = method_num.parse::<u8>(){
                    (method, method_num)
                }
                else{
                    if *lang == Language::Polish{
                        return "<error>
                            <p>Numer metody nie mógł być przerobiony na liczbę 8-bitową</p></error>"
                            .to_string();
                    }
                    else{
                        return "<error><p>Method Number couldn't be parsed to 8-bit u_int</p></error>"
                            .to_string();
                    }
                }
            }
            else{
                if *lang == Language::Polish{
                    return "<error><p>Nie udało się przetworzyć metody</p></error>".to_string();
                }
                else{
                    return "<error><p>Couldn't parse method</p></error>".to_string();
                }
            }
        }
        None => {
            if *lang == Language::Polish{
                return "<error>
                <p>Nie udało się znaleźć pola nazwanego 'metody'</p></error>".to_string();
            }
            else{
                return "<error><p>Couldn't find argument named 'method'</p>
                </error>".to_string();
            }
        }
    };

    if let Some(password) = args.get("password"){
        if let Some(set_password) = shared_components::backend::get_password().await{
            if set_password != *password
            {
                if method == "PER" && method_num == 1{
                    return web::login(*lang);
                }
                else{
                    if *lang == Language::Polish{
                        return "<error>
                        <p>Wprowadzone złe hasło</p></error>".to_string();
                    }
                    else    {
                    return "<error><p>Wrong password was entered</p>
                        </error>".to_string();
                    }
                }
            }
            else{
                if method == "PER" && method_num == 1 && set_password == *password{
                    return "true".to_string();
                }
            }
        }
        else{
            if *lang == Language::Polish{
                return "<error><p>Nie udało się uzyskać hasła, spytaj administratora</p></error>".to_string();
            }
            else{
                return "<error><p>Couldn't get password, ask admin</p></db_row></error>".to_string();
            }
        }
    }
    else{
        if *lang == Language::Polish{
            return "<error><p>Nie znaleziono hasła</p></error>".to_string();
        }
        else{
            return "<error><p>Couldn't find password</p></error>".to_string();
        }
    }
    match method{
        "GET" => {
            match method_num{
                1 => {
                    let class_id = args.get("arg1");
                    let database = db.lock().await;
                    let query = 
                        "SELECT Lessons.week_day Classes.class_name,
                        LessonHours.start_time, LessonHours.end_time,
                        Teachers.first_name, Teachers.last_name,
                        Subjects.subject_name, Classrooms.classroom_name,
                        Lessons.lesson_num, Lessons.teacher_id
                        FROM Lessons 
                        WHERE class_id = ?1
                        JOIN Classes ON Lessons.class_id = Classes.class_id
                        JOIN Teachers ON Lessons.teacher_id = Teachers.teacher_id
                        JOIN Subjects ON Lessons.subject_id = Subjects.subject_id
                        JOIN Classrooms ON Lessons.classroom_id = Classrooms.classroom_id 
                        JOIN LessonHours ON Lessons.lesson_hour = LessonHours.lesson_num
                        ";
                    type Lessons = Vec<(u8, String, u16, u16, String, String, String, String, u8, u16)>;
                    if let Ok(mut stmt) = database.prepare(&query){
                        if let Ok(iter) = stmt.query_map([class_id], |row| {
                            Ok((
                                    row.get::<usize, u8>    (0).unwrap_or_default(),
                                    row.get::<usize, String>(1).unwrap_or_default(),
                                    row.get::<usize, u16>   (2).unwrap_or_default(),
                                    row.get::<usize, u16>   (3).unwrap_or_default(),
                                    row.get::<usize, String>(4).unwrap_or_default(),
                                    row.get::<usize, String>(5).unwrap_or_default(),
                                    row.get::<usize, String>(6).unwrap_or_default(),
                                    row.get::<usize, String>(7).unwrap_or_default(),
                                    row.get::<usize, u8>    (8).unwrap_or_default(),
                                    row.get::<usize, u16>   (9).unwrap_or_default()
                            ))
                        })
                        {
                            let filtered_iter : Lessons = 
                                iter.filter(|s| s.is_ok()).map(|s| s.unwrap()).filter(|(wd, cn, st, et,fin,ln,sn,cln,len,ti)| 
                                *wd!=0&&!cn.is_empty()&&*st!=0&&*et!=0&&!fin.is_empty()&&!ln.is_empty()&&!sn.is_empty()&&
                                !cln.is_empty()&&*len!=0&&ti!=&0
                            ).collect();
                            let mut to_return : String = String::from("<class>");                    
                            let mut sorted_map : BTreeMap<(u8, u8), ((u16, u16), (String, String), String, String, u16)> = BTreeMap::new();
                            for (wd, _, lhs, lhe, tfn, tln, ssn, ccn, lln, ti) in filtered_iter{
                                sorted_map.insert((wd, lln), ((lhs, lhe), (tfn, tln), ssn, ccn, ti));
                            }
                            for (wd, lh) in sorted_map.keys(){
                                to_return.push_str(
                                    &format!("<weekd><p><strong>{}</strong></p>", weekd_to_string(&*lang, *wd))
                                );
                                if let Some(((lhs, lhe), (tfn, tln), ssn, ccn, ti)) = sorted_map.get(&(*wd, *lh)){
                                    to_return.push_str(&format!(
                                    "<lesson lh='{lh}' wd='{wd}' ssn='{ssn}' ti='{ti}' ccn='{ccn}'>
                                    <p><strong>{} {}</strong><p>{}</p></p><p>{}</p><p>{} {}</p><p>{}</p></lesson>",
                                    lang.english_or("Lesson", "Lekcja"), lh,
                                    (*lhs, *lhe).msat_to_string(),
                                    ssn, tfn, tln, ccn
                                    ))
                                }
                                to_return.push_str("</weekd>")
                            }
                            to_return.push_str("</class>");
                        }
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
                            let mut to_return : String = format!("<db_col>
                                <db_row>
                                <p>{}</p>
                                <p>{}</p>
                                <p>{}</p>
                                </db_row>",
                                if *lang == Language::Polish{
                                    "ID Nauczyciela"
                                }
                                else{
                                    "Teacher ID"
                                },
                                if *lang == Language::Polish{
                                    "Imię"
                                }
                                else{
                                    "First Name"
                                },
                                if *lang == Language::Polish{
                                    "Nazwisko"
                                }
                                else{
                                    "Last Name"
                                }
                            );
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
                                break_num: row.get(0).unwrap_or(0),
                                teacher_id: row.get(1).unwrap_or(0),
                                break_place: row.get(2).unwrap_or(0.to_string()),
                                week_day: row.get(3).unwrap_or(0),
                            })
                        })
                        {
                            let filtered_iter : Vec<Duty> 
                                = iter.filter(|s| s.is_ok())
                                .map(|s| s.unwrap())
                                .filter(|s| s.break_num!=0&&s.teacher_id!=0&&&s.break_place!=""&&s.week_day!=0)
                                .collect();
                            let mut to_return : String = format!("<db_col>
                                <db_row>
                                <p>{}</p>
                                <p>{}</p>
                                <p>{}</p>
                                <p>{}</p>
                                </db_row>",
                                if *lang == Language::Polish{
                                    "Numer Przerwy"
                                }
                                else{
                                    "Break Number"
                                },
                                if *lang == Language::Polish{
                                    "ID Nauczyciela"
                                }
                                else{
                                    "Teacher ID"
                                },
                                if *lang == Language::Polish{
                                    "Miejsce Przerwy"
                                }
                                else{
                                    "Break Place"
                                },
                                if *lang == Language::Polish{
                                    "Dzień Tygodnia"
                                }
                                else{
                                    "Week Day"
                                }
                            );
                            for e in filtered_iter{
                                to_return.push_str(
                                    format!("<db_row>
                                    <p>{}</p>
                                    <p>{}</p>
                                    <p>{}</p>
                                    <p>{}</p></db_row>", e.break_num,e.teacher_id,e.week_day,e.week_day).as_str()
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
                            let mut to_return : String = format!("<db_col>
                                <db_row>
                                <p>{}</p>
                                <p>{}</p>
                                </db_row>",
                                if *lang == Language::Polish{
                                    "ID Przedmiotu"
                                }
                                else{
                                    "Subject ID"
                                },
                                if *lang == Language::Polish{
                                    "Nazwa Przedmiotu"
                                }
                                else{
                                    "Subject Name"
                                }
                            );
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
                            let mut to_return : String = format!("<db_col>
                                <db_row>
                                <p>{}</p>
                                <p>{}</p>
                                </db_row>",
                                if *lang == Language::Polish{
                                    "ID Klasy"
                                }
                                else{
                                    "Class ID"
                                },
                                if *lang == Language::Polish{
                                    "Nazwa Klasy"
                                }
                                else{
                                    "Class Name"
                                }
                            );
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
                            let mut to_return : String = format!("<db_col>
                                <db_row>
                                <p>{}</p>
                                <p>{}</p>
                                </db_row>",
                                if *lang == Language::Polish{
                                    "Numer Klasy (Pomieszczenie)"
                                }
                                else{
                                    "Classroom Number"
                                },
                                if *lang == Language::Polish{
                                    "Nazwa Klasy (Pomieszczenie)"
                                }
                                else{
                                    "Classroom Name"
                                }
                            );
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
                            let mut to_return : String = format!("<db_col>
                                <db_row>
                                <p>{}</p>
                                <p>{}</p>
                                <p>{}</p>
                                </db_row>",
                                if *lang == Language::Polish{
                                    "Numer Lekcji"
                                }
                                else{
                                    "Lesson Number"
                                },
                                if *lang == Language::Polish{
                                    "Godzina Rozpoczęcia"
                                }
                                else{
                                    "Start Time"
                                },
                                if *lang == Language::Polish{
                                    "Godzina Zakończenia"
                                }
                                else{
                                    "End Time"
                                }
                            );
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
                8 => {
                    let database = db.lock().await;
                    let query = "SELECT * FROM BreakHours";
                    if let Ok(mut stmt) = database.prepare(&query){
                        if let Ok(iter) = stmt.query_map([], |row| {
                            Ok(BreakHours{
                                break_num: row.get(0).unwrap_or(0),
                                start_time: row.get(1).unwrap_or(0),
                                end_time: row.get(2).unwrap_or(0)
                            })
                        })
                        {
                            let filtered_iter : Vec<BreakHours> 
                                = iter.filter(|s| s.is_ok())
                                .map(|s| s.unwrap())
                                .filter(|s| s.break_num!=0&&s.start_time!=0&&s.end_time!=0)
                                .collect();
                            let mut to_return : String = format!("<db_col>
                                <db_row>
                                <p>{}</p>
                                <p>{}</p>
                                <p>{}</p>
                                </db_row>",
                                if *lang == Language::Polish{
                                    "Numer Przerwy"
                                }
                                else{
                                    "Break Number"
                                },
                                if *lang == Language::Polish{
                                    "Godzina Rozpoczęcia"
                                }
                                else{
                                    "Start Time"
                                },
                                if *lang == Language::Polish{
                                    "Godzina Zakończecia"
                                }
                                else{
                                    "End Time"
                                }
                            );
                            for e in filtered_iter{
                                to_return.push_str(
                                    format!("<db_row>
                                    <p>{}</p>
                                    <p>{}</p>
                                    <p>{}</p></db_row>", e.break_num,e.start_time,e.end_time).as_str()
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
            match method_num{
                0 => {
                    let query = "INSERT INTO BreakHours 
                        (break_num, start_time, end_time)
                        VALUES (?1, ?2, ?3)
                        ON CONFLICT (break_num)
                        DO UPDATE SET start_time = excluded.start_time, end_time = excluded.end_time";
                    if let (Some(break_num), Some(start_time), Some(end_time)) 
                        = (args.get("arg1"), args.get("arg2"), args.get("arg3"))
                    {
                        if let (Some((_, value)), Some((_, value1)), Some((_, value2))) = 
                        (break_num.split_once('='),start_time.split_once('='),end_time.split_once('='))
                        {
                           let database = db.lock().await;
                           match database.execute(&query, [value.into(), value1, value2]){
                                Ok(_) => return database_insert_error_msg(&*lang),
                                Err(_) => return database_insert_success_msg(&*lang)
                           }
                        }
                    };
                    if *lang == Language::Polish{
                        return "<error><p>Napotkano błąd</p></error>".to_string()
                    }
                    else{
                        return "<error><p>Error occured</p></error>".to_string()
                    }
                }
                1 => {
                    let query = "INSERT INTO Lessons 
                            (week_day, class_id, classroom_id, subject_id, teacher_id, lesson_hour) 
                            VALUES (?1,?2,?3,?4,?5,?6)
                            ON CONFLICT (class_id, lesson_hour, week_day) 
                            DO UPDATE SET classroom_id = excluded.classroom_id, subject_id = excluded.subject_id,
                            teacher_id = excluded.teacher_id;";
                    if let (Some(class_id), Some(classroom_id), Some(subject_id), Some(teacher_id), Some(lesson_num), Some(week_day)) =
                    (args.get("arg1"), args.get("arg2"), args.get("arg3"), args.get("arg4"), args.get("arg5"), args.get("arg6"))
                    {
                        if let (Some((_, class)), Some((_, classroom)), Some((_, subject)), Some((_, teacher)), Some((_, lesson)), Some((_, weekd))) =
                        (class_id.split_once('='), classroom_id.split_once('='), subject_id.split_once('='), teacher_id.split_once('='),
                         lesson_num.split_once('='), week_day.split_once('=')) 
                        {
                            let database = db.lock().await;
                            
                            return match database.execute(&query, [weekd, class, classroom, subject, teacher, lesson])
                            {
                                Ok(_) => database_insert_success_msg(&*lang),
                                Err(_) => database_insert_error_msg(&*lang)
                            }
                            
                        }
                    }
                    else{
                        return database_insert_error_msg(&*lang);
                    }
                }
                2 => {
                    if let (Some(teacher_id), Some(first_name), Some(last_name)) = (args.get("arg1"), args.get("arg2"), args.get("arg3"))
                    {
                        if let (Some((_, teacher)), Some((_, first_name1)), Some((_, last_name1))) = 
                        (teacher_id.split_once('='), first_name.split_once('='), last_name.split_once('='))
                        {
                            let query = "INSERT INTO Teachers (teacher_id, first_name, last_name) VALUES (?1, ?2, ?3) 
                            ON CONFLICT (teacher_id) DO UPDATE SET first_name = excluded.first_name, last_name = excluded.last_name";
                            let database = db.lock().await;
                            match database.execute(&query, [&teacher, first_name1, last_name1]){
                                Ok(_) => return database_insert_success_msg(&*lang),
                                Err(_) => return database_insert_error_msg(&*lang)
                            }
                        }
                    }
                    else{
                        return database_insert_error_msg(&*lang);
                    }
                }
                3 => {
                    let query = "INSERT INTO Duties (lesson_hour, teacher_id, classroom_id, week_day) VALUES (?1, ?2, ?3, ?4)
                        ON CONFLICT (lesson_hour, teacher_id, week_day) DO UPDATE SET classroom_id = excluded.classroom_id";
                    if let (Some(lesson_hour), Some(teacher_id), Some(classroom_id), Some(week_day)) = 
                    (args.get("arg1"), args.get("arg2"), args.get("arg3"), args.get("arg4")) 
                    {
                        if let (Some((_, lesson)), Some((_, teacher)), Some((_, classroom)), Some((_, weekd))) =
                        (lesson_hour.split_once('='), teacher_id.split_once('='), classroom_id.split_once('='), week_day.split_once('='))
                        {
                            let database = db.lock().await;
                            match database.execute(&query, [lesson, teacher, classroom, weekd]){
                                Ok(_) => return database_insert_success_msg(&*lang),
                                Err(_) => return database_insert_error_msg(&*lang)
                            }
                        }
                    };
                }
                4 => {
                    let query = "INSERT INTO Subjects (subject_id, subject_name) VALUES (?1, ?2)
                        ON CONFLICT (subject_id) DO UPDATE SET subject_name = excluded.subject_name";
                    if let (Some(subject_id), Some(subject_name)) = (args.get("arg1"), args.get("arg2"))
                    {
                        if let (Some((_, id)), Some((_, name))) = (subject_id.split_once('='), subject_name.split_once('='))
                        {
                            let database = db.lock().await;
                            match database.execute(&query, [id, name]){
                                Ok(_) => return database_insert_success_msg(&*lang),
                                Err(_) => return database_insert_error_msg(&*lang)
                            }
                        }
                    };
                }
                5 => {
                    let query = "INSERT INTO Classes (class_id, class_name) VALUES (?1, ?2)
                        ON CONFLICT (class_id) DO UPDATE SET class_name = excluded.class_name";
                    if let (Some(class_id), Some(class_name)) = (args.get("arg1"), args.get("arg2")) 
                    {
                        if let (Some((_, id)), Some((_, name))) = (class_id.split_once('='), class_name.split_once('=')){
                            let database = db.lock().await;
                            match database.execute(&query, [id, name]){
                                Ok(_) => return database_insert_success_msg(&*lang),
                                Err(_) => return database_insert_error_msg(&*lang)
                            }
                        }
                    };
                }
                6 => {
                    let query = "INSERT INTO Classrooms (classroom_id, classroom_name) VALUES (?1, ?2)
                        ON CONFLICT (classroom_id) DO UPDATE SET classroom_name = excluded.classroom_name";
                    if let (Some(classroom_id), Some(classroom_name)) = (args.get("arg1"), args.get("arg2")) 
                    {
                        if let (Some((_, id)), Some((_, name))) = (classroom_id.split_once('='), classroom_name.split_once('=')){
                            let database = db.lock().await;
                            match database.execute(&query, [id, name]){
                                Ok(_) => return database_insert_success_msg(&*lang),
                                Err(_) => return database_insert_error_msg(&*lang)
                            }
                        }
                    };
                }
                7 => {
                    let query = "INSERT INTO LessonHours (lesson_num, start_time, end_time) VALUES (?1, ?2, ?3)
                        ON CONFLICT (lesson_num) DO UPDATE SET start_time = excluded.start_time, end_time = excluded.end_time";
                    if let (Some(lesson_num), Some(start_time), Some(end_time)) = (args.get("arg1"), args.get("arg2"), args.get("arg3"))
                    {
                        if let (Some((_, lesson)), Some((_, start)), Some((_, end))) = 
                        (lesson_num.split_once('='), start_time.split_once('='), end_time.split_once('='))
                        {
                            let database = db.lock().await;
                            match database.execute(&query, [lesson, start, end]){
                                Ok(_) => return database_insert_success_msg(&*lang),
                                Err(_) => return database_insert_error_msg(&*lang)
                            }
                        }
                    }
                }
                _ => {
                }
            }
        }
        "PER" => {
            match method_num{
                0 => {
                    let database = db.lock().await;
                    let none = &"0".to_string();
                    let teacher_id = args.get("arg1").unwrap_or(none);
                    println!("Teacher_id: {}", teacher_id);
                    let query = "SELECT  
                    Lessons.week_day, LessonHours.start_time, LessonHours.end_time, Lessons.lesson_hour,
                    Classrooms.classroom_name, Classes.class_name, Subjects.subject_name
                    FROM Lessons 
                    JOIN LessonHours ON Lessons.lesson_hour = LessonHours.lesson_num
                    JOIN Classrooms ON Lessons.classroom_id = Classrooms.classroom_id
                    JOIN Classes ON Lessons.class_id = Classes.class_id
                    JOIN Subjects ON Lessons.subject_id = Subjects.subject_id
                    WHERE teacher_id = ?1";
                    if let Ok(mut stmt) = database.prepare(&query){
                        if let Ok(iter) = stmt.query_map([teacher_id], |row| {
                            Ok((
                                    row.get::<usize, u8>     (0).unwrap_or_default(),
                                    row.get::<usize, u16>    (1).unwrap_or_default(),
                                    row.get::<usize, u16>    (2).unwrap_or_default(),
                                    row.get::<usize, u8>     (3).unwrap_or_default(),
                                    row.get::<usize, String> (4).unwrap_or_default(),
                                    row.get::<usize, String> (5).unwrap_or_default(),
                                    row.get::<usize, String> (6).unwrap_or_default(),

                            ))
                        })
                        {
                            let filtered_iter : Vec<(u8, u16, u16, u8, String, String, String)> 
                                = iter.filter(|s| s.is_ok())
                                .map(|s| s.unwrap())
                                .filter(|(wd, lhs, lhe, llh, ccn, cln, ssn)| 
                                    wd!=&0&&lhs!=&0&&lhe!=&0&&llh!=&0&&!ccn.is_empty()&&!cln.is_empty()&&!ssn.is_empty()
                                )
                                .collect();
                            let mut sorted_map : BTreeMap<(u8, u8), ((u16, u16), String, String, String)> = BTreeMap::new();
                            for (wd, lhs, lhe, llh,ccn, cln, ssn) in filtered_iter{
                                sorted_map.insert((wd, llh), ((lhs, lhe), ssn, cln, ccn));
                            }

                            let x = Orb::Data(("Fizyka".to_string(), "Sala Fizyczno-Chemiczna".to_string(), "Klasa 8".to_string()));
                            return dashboard("Mateus", Some((1, 1)), x, Language::Polish, sorted_map);
                        }
                    }
                    else if let Err(error) = database.prepare(&query){
                        println!("Error: {}", error);
                    };
                }
                1 => {
                    return "true".to_string();
                }
                2 => {
                    return web::post_login(&*lang);
                }
                3 => {
                    return web::admin_init(&*lang);
                }
                _ => {}
            }
        }
        _ => {
        }
    }
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

fn database_insert_success_msg(lang: &Language) -> String{
    return if lang == &Language::Polish{
        "<success><p>Pomyślnie dodano dane do bazy danych</p></success>".to_string()
    }
    else{
        "<success><p>Successfully added data to database</p></success>".to_string()
    };
}

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
