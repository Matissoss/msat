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
use rusqlite::{self, OpenFlags};

// Local Imports 
use shared_components::{
    database::*, password::get_password, split_string_by, types::*, LOCAL_IP, SQLITE_FLAGS, cli, config
};

#[tokio::main]
#[allow(warnings)]
async fn main(){
    cli::main();
    init_httpserver().await;
}

pub async fn init_httpserver() {
    if let Err(_) = init(*SQLITE_FLAGS).await{
        std::process::exit(-1);
    };
    let database = Arc::new(Mutex::new(
            match rusqlite::Connection::open_with_flags("data/database.db",
                OpenFlags::SQLITE_OPEN_CREATE|OpenFlags::SQLITE_OPEN_FULL_MUTEX|OpenFlags::SQLITE_OPEN_READ_WRITE){
                Ok(v) => v,
                Err(_) => std::process::exit(-1)
            }
    ));

    let (ip, port, max_limit, max_timeout) : (IpAddr, u16, u16, Arc<u64>) = match config::get().await{
        Ok(c) => {
            if let Some(config) = c{
                (config.http_server.tcp_ip.unwrap_or(*LOCAL_IP), 
                 config.http_server.http_port, config.http_server.max_connections,
                 Arc::new(config.http_server.max_timeout_seconds.into()))
            }
            else{
                (*LOCAL_IP, 8000, 100, Arc::new(10))
            }
        }
        Err(_) => {
            (*LOCAL_IP, 8000, 100, Arc::new(10))
        }
    };
    let limit = Arc::new(Semaphore::new(max_limit.into()));
    let final_address = format!("{}:{}", ip.to_string(), port);
    let listener: TcpListener = match TcpListener::bind(final_address) {
        Ok(v) => v,
        Err(_) => std::process::exit(-1),
    };
    cli::print_success("Initialized HTTP Server");
    loop {
        for s in listener.incoming() {
            cli::debug_log("Request Incoming");
            if let Ok(stream) = s{
                let cloned_dbptr = Arc::clone(&database);
                let cloned_permit = Arc::clone(&limit);
                let cloned_timeout = Arc::clone(&max_timeout);
                if let Ok(_) = tokio::time::timeout(std::time::Duration::from_secs(*cloned_timeout), 
                    cloned_permit.acquire_owned()).await{
                    tokio::spawn(async {
                        handle_connection(stream, cloned_dbptr).await;
                    });
                }
            }
            else if let Err(error) = s{
                cli::print_error("TCPStream is Err", error);
            }
        }
    }
}
pub async fn handle_connection(mut stream: TcpStream, db_ptr: Arc<Mutex<rusqlite::Connection>>) {
    let mut buffer = [0u8; 2048];
    if let Ok(len) = stream.read(&mut buffer) {
        if len == 0 {
        } else {
            let request = String::from_utf8_lossy(&buffer[0..len]).to_string();
            for l in request.lines(){
                if !l.is_empty()
                {
                    cli::debug_log(l);
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
                                let response = handle_custom_request(&w, cloned_dbptr).await;
                                match stream.write_all(
                                    format!("HTTP/1.1 200 OK\r\nContent-Length:{}\r\nContent-Type: application/xml\r\n\r\n{}",
                                        response.len(), response).as_bytes())
                                {
                                    Ok(_) =>  cli::print_info("Handled Request"),
                                    Err(_) => cli::print_info("Couldn't Handle Request")
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
            cli::debug_log(&format!("file_path = {}", file_path));

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
async fn handle_custom_request(request: &str, db: Arc<Mutex<rusqlite::Connection>>) -> String{
    // request example: /?method=POST&version=10&args=20
    let request = String::from_iter(request.chars().collect::<Vec<char>>()[2..].iter());
    let mut request_type = "".to_string();
    let mut args : Vec<String> = vec![];
    let mut request_number = 0;
    let mut password : String = "".to_string();
    let req_split = split_string_by(&request, '&');

    for s in req_split{
        if s.starts_with("args="){
            args = 
                split_string_by(&strvec_to_str(&split_string_by(&s, '=')[1..].to_vec()),'+');
        }
        else if s.starts_with("method="){
            let request_arguments = split_string_by(&strvec_to_str(&split_string_by(&s, '=')[1..].to_vec()),'+');
            request_type = request_arguments[0].to_string();
            if let Ok(v) = str::parse::<u8>(&request_arguments[1]){
                request_number = v;
            }
        }
        else if s.starts_with("password="){
            let req_args = split_string_by(&s, '=');
            if req_args.len() <= 1 {
                return "<db_col><db_row><p>Brak Hasła/No Password</p></db_row></db_col>".to_string();
            }
            else{
                password = req_args[1].clone();
                match get_password().await{
                    Some(conf_password) => {
                        if password != conf_password{
                            return 
                                "
                                <db_col><db_row>
                                <p>Password Entered isn't same as one that is used on server</p>
                                <p>Hasło wprowadzone nie jest takie same jak ustawione na serwerze</p></db_row></db_col>
                                "
                                .to_string();
                        }
                    }
                    None => {
                        return "<db_col><db_row><p>Password isn't set, ask admin to set password</p>
                            <p>Hasło nie zostało ustawione, spytaj administratora by ustawił hasło</p></db_row></db_col>"
                            .to_string();
                    }
                }
            }
        }
    }
    println!("REQUEST: \"{}\", {}, \"{}\"", request_type, request_number, strvec_to_str(&args));
    match request_type.as_str(){
        "GET" => {
            match request_number{
                0 => {
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
                            let (classroom_hashmap, class_hashmap, subject_hashmap, teacher_hashmap) = 
                                (
                                    get_classrooms(&database),
                                    get_classes(&database),
                                    get_subjects(&database),
                                    get_teachers(&database)
                                );
                            if class_hashmap.len() == 0 || classroom_hashmap.len() == 0 || subject_hashmap.len() == 0{
                            return 
                            "<db_col><db_row><p>Nie znaleziono Danych</p><p>No Data found</p></db_row></db_col>".to_string();
                            }
                            let filtered_iter : Vec<Lesson> 
                                = iter.filter(|s| s.is_ok())
                                .map(|s| s.unwrap())
                                .filter(|s| 
                                s.class_id!=0&&s.lesson_hour!=0&&s.teacher_id!=0&&s.subject_id!=0
                                &&s.classroom_id!=0&&s.week_day!=0)
                                .collect();
                            let mut to_return : String = String::from("");                    
                            let mut sorted_map : BTreeMap<(u16, u8, u8), (u16, u16, u16)> = BTreeMap::new();
                            for lesson in filtered_iter{
                                let (ci, wd, lh, si, cli, ti) = 
                                (lesson.class_id, lesson.week_day, lesson.lesson_hour,
                                 lesson.subject_id, lesson.classroom_id, lesson.teacher_id);
                                sorted_map.insert((ti, wd, lh), (si, cli, ci));
                            }
                            let (mut lc, mut llh) = (&0, &0);
                            for (class, _, lessonh) in sorted_map.keys(){
                                if lc < class{
                                    lc = class;
                                }
                                if llh < lessonh {
                                    llh = lessonh;
                                }
                            }
                            for teacher in 1..=*lc{
                                let (first_name, last_name) : (String, String) = match teacher_hashmap.get(&teacher)
                                {
                                    Some(s) => s.clone(),
                                    None => {
                                        (teacher.to_string(), "".to_string())
                                    }
                                };
                                to_return.push_str(&format!("<class id='{teacher}'><p>{} {}</p><w>\n", 
                                        first_name, last_name));
                                for weekd in 1..=7u8{
                                    to_return.push_str(&format!("<weekd id='w{weekd}'><p>{}</p>\n", weekd_to_string(weekd)));
                                    for lesson_num in 1..=*llh{
                                        if let Some((si, cli, ci)) = sorted_map.get(&(teacher, weekd, lesson_num)){
                                            to_return.push_str(
                                            &format!
                                            ("<lesson><p><strong>Lekcja/Lesson{lesson_num}</strong></p>
                                             <p>{}</p><p>{}</p><p>{}</p></lesson>\n",
                                                    subject_hashmap.get(si).unwrap_or(&si.to_string()),
                                                    class_hashmap.get(ci).unwrap_or(&ci.to_string()),
                                                    classroom_hashmap.get(cli).unwrap_or(&cli.to_string())));
                                        }
                                    }
                                    to_return.push_str("</weekd>\n");
                                }
                                to_return.push_str("</w></class>\n");
                            }
                            return to_return;
                        };
                    };

                }
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
                            let (class_hashmap, classroom_hashmap, subject_hashmap, teacher_hashmap) = 
                                (
                                    get_classes   (&database),
                                    get_classrooms(&database),
                                    get_subjects  (&database),
                                    get_teachers  (&database)
                                );
                            let filtered_iter : Vec<Lesson> 
                                = iter.filter(|s| s.is_ok())
                                .map(|s| s.unwrap())
                                .filter(|s| 
                                s.class_id!=0&&s.lesson_hour!=0&&s.teacher_id!=0&&s.subject_id!=0
                                &&s.classroom_id!=0&&s.week_day!=0)
                                .collect();
                            let mut to_return : String = String::from("");                    
                            let mut sorted_map : BTreeMap<(u16, u8, u8), (u16, u16, u16)> = BTreeMap::new();
                            for lesson in filtered_iter{
                                let (ci, wd, lh, si, cli, ti) = 
                                (lesson.class_id, lesson.week_day, lesson.lesson_hour,
                                 lesson.subject_id, lesson.classroom_id, lesson.teacher_id);
                                sorted_map.insert((ci, wd, lh), (si, cli, ti));
                            }
                            let (mut lc, mut llh) = (&0, &0);
                            for (class, _, lessonh) in sorted_map.keys(){
                                if lc < class{
                                    lc = class;
                                }
                                if llh < lessonh {
                                    llh = lessonh;
                                }
                            }
                            for class in 1..=*lc{
                                to_return.push_str(&format!("<class id='{class}'><p>{}</p><w>\n", 
                                        class_hashmap.get(&class).unwrap_or(&class.to_string())));
                                for weekd in 1..=7u8{
                                    to_return.push_str(&format!("   <weekd id='w{weekd}'><p>{}</p>\n", weekd_to_string(weekd)));
                                    for lesson_num in 1..=*llh{
                                        if let Some((si, cli, ti)) = sorted_map.get(&(class, weekd, lesson_num)){
                                            let (first_name, last_name) : (String, String) = match teacher_hashmap.get(ti){
                                                Some(s) => s.clone(),
                                                None => {
                                                    (ti.to_string(), "".to_string())
                                                }
                                            };
                                            to_return.push_str(
                                                &format!("<lesson><p><strong>Lekcja/Lesson {}</strong></p><p>{}</p><p>{} {}</p><p>{}</p></lesson>\n",
                                                    lesson_num,
                                                    subject_hashmap.get(si).unwrap_or(&si.to_string()),
                                                    first_name, last_name,
                                                    classroom_hashmap.get(cli).unwrap_or(&cli.to_string())));
                                        }
                                    }
                                    to_return.push_str("</weekd>\n");
                                }
                                to_return.push_str("</w></class>\n");
                            }
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
                                <p id='1'>Teacher ID</p>
                                <p id='2'>First Name</p>
                                <p id='3'>Last Name</p>
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
                            let mut to_return : String = String::from("<db_col>
                                <db_row>
                                <p id='1'>Break Number</p>
                                <p id='2'>Teacher ID</p>
                                <p id='3'>Break Place</p>
                                <p id='4'>Week Day</p>
                                </db_row>");
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
                            let mut to_return : String = String::from("<db_col>
                                <db_row>
                                <p id='1'>Subject ID</p>
                                <p id='2'>Subject Name</p>
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
                                <p id='1'>Class ID</p>
                                <p id='2'>Class Name</p>
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
                                <p id='1'>Classroom ID</p>
                                <p id='2'>Classroom Name</p>
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
                                <p id='1'>Lesson Number</p>
                                <p id='2'>Start Time</p>
                                <p id='3'>End Time</p>
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
                            let mut to_return : String = String::from("<db_col>
                                <db_row>
                                <p id='1'>Break Number</p>
                                <p id='2'>Start Time</p>
                                <p id='3'>End Time</p>
                                </db_row>");
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
            match request_number{
                0 => {
                    if args.len() < 3{
                        return not_enough_arguments(3, args.len());
                    }
                    let query = "INSERT INTO BreakHours 
                        (break_num, start_time, end_time)
                        VALUES (?1, ?2, ?3)
                        ON CONFLICT (break_num)
                        DO UPDATE SET start_time = excluded.start_time, end_time = excluded.end_time";
                    let (break_num, start_time, end_time) = 
                        (
                            str::parse::<u8>(args[0].trim()), str::parse::<u16>(args[1].trim()),
                            str::parse::<u16>(args[2].trim())
                        );
                    if let (Ok(break_num1), Ok(start_time1), Ok(end_time1)) = (break_num, start_time, end_time){
                        let database = db.lock().await;
                        match database.execute(&query, [break_num1.into(), start_time1, end_time1]){
                            Ok(_) => {
                                return 
                                    "<db_col><db_row><p>Pomyślnie dodano dane do bazy danych</p>
                                    <p>Successfully added data to database</p></db_row></db_col>".to_string()
                            }
                            Err(_) => {
                            }
                        }
                    }
                    return "<db_col><db_row><p>Napotkano błąd</p><p>Error occured</p></db_row></db_col>".to_string()
                }
                1 => {
                    if args.len() < 6{
                        return not_enough_arguments(6, args.len());
                    }
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
                2 => {
                    if args.len() < 3{
                        return not_enough_arguments(3, args.len());
                    }
                    let query = "INSERT INTO Teachers (teacher_id, first_name, last_name) VALUES (?1, ?2, ?3)
                        ON CONFLICT (teacher_id) DO UPDATE SET first_name = excluded.first_name, last_name = excluded.last_name";
                    let (teacher_id, first_name, last_name) = (
                        str::parse::<u8>(args[0].trim()), &args[1], &args[2]
                        );
                    if teacher_id.is_ok()&!first_name.is_empty()&!last_name.is_empty() == true{
                        let database = db.lock().await;
                        match database.execute(&query, [&teacher_id.unwrap().to_string(), first_name, last_name]){
                            Ok(_) => return "<db_col><db_row><p>Success/Sukces</p></db_row></db_col>".to_string(),
                            Err(e) => return format!("<db_col><db_row><p>Error/Błąd</p></db_row><db_row><p>{}</p></db_row></db_col>", e.to_string())
                        }
                    }
                }
                3 => {
                    if args.len() < 4{
                        return not_enough_arguments(4, args.len());
                    }
                    let query = "INSERT INTO Duties (lesson_hour, teacher_id, classroom_id, week_day) VALUES (?1, ?2, ?3, ?4)
                        ON CONFLICT (lesson_hour, teacher_id, week_day) DO UPDATE SET classroom_id = excluded.classroom_id";
                    let (lesson_hour, teacher_id, classroom_id, week_day) = 
                        (
                            str::parse::<u8>(args[0].trim()), str::parse::<u8>(args[1].trim()), str::parse::<u8>(args[2].trim()), str::parse::<u8>(args[3].trim())
                        );
                    if lesson_hour.is_ok()&teacher_id.is_ok()&classroom_id.is_ok()&week_day.is_ok() == true{
                        let database = db.lock().await;
                        match database.execute(&query, [lesson_hour.unwrap(), teacher_id.unwrap(), classroom_id.unwrap(), week_day.unwrap()]){
                            Ok(_) => return "<db_col><db_row><p>Success/Sukces</p></db_row></db_col>".to_string(),
                            Err(e) => return format!("<db_col><db_row><p>Error/Błąd: {}</p></db_row></db_col>", e.to_string())
                        }
                    }
                }
                4 => {
                    if args.len() < 2{
                        return not_enough_arguments(2, args.len());
                    }
                    let query = "INSERT INTO Subjects (subject_id, subject_name) VALUES (?1, ?2)
                        ON CONFLICT (subject_id) DO UPDATE SET subject_name = excluded.subject_name";
                    let (subject_id, subject_name) = 
                        (str::parse::<u8>(args[0].trim()), &args[1]);
                    if !subject_name.is_empty()&&subject_id.is_ok() == true{
                        let database = db.lock().await;
                        match database.execute(&query, [&subject_id.unwrap().to_string(), subject_name]){
                            Ok(_) => return "<db_col><db_row><p>Sukces/Success</p></db_row></db_col>".to_string(),
                            Err(e) => return format!("<db_col><db_row><p>Błąd/Error {}</p></db_row></db_col>", e)
                        }
                    }
                }
                5 => {
                    if args.len() < 2{
                        return not_enough_arguments(2, args.len());
                    }
                    let query = "INSERT INTO Classes (class_id, class_name) VALUES (?1, ?2)
                        ON CONFLICT (class_id) DO UPDATE SET class_name = excluded.class_name";
                    let (class_id, class_name) = 
                        (str::parse::<u8>(args[0].trim()), &args[1]);
                    if class_id.is_ok()&!class_name.is_empty() == true{
                        let database = db.lock().await;
                        match database.execute(&query, [&class_id.unwrap().to_string(), class_name]){
                            Ok(_) => return "<db_col><db_row><p>Sukces/Success</p></db_row></db_col>".to_string(),
                            Err(e) => return format!("<db_col><db_row><p>Błąd/Error {}</p></db_row></db_col>", e)
                        }
                    }
                }
                6 => {
                    if args.len() < 2{
                        return not_enough_arguments(2, args.len());
                    }
                    let query = "INSERT INTO Classrooms (classroom_id, classroom_name) VALUES (?1, ?2)
                        ON CONFLICT (classroom_id) DO UPDATE SET classroom_name = excluded.classroom_name";
                    let (classroom_id, classroom_name) = 
                        (str::parse::<u8>(args[0].trim()), &args[1]);
                    if classroom_id.is_ok()&!classroom_name.is_empty() == true{
                        let database = db.lock().await;
                        match database.execute(&query, [&classroom_id.unwrap().to_string(), classroom_name]){
                            Ok(_) => return "<db_col><db_row><p>Sukces/Success</p></db_row></db_col>".to_string(),
                            Err(e) => return format!("<db_col><db_row><p>Błąd/Error {}</p></db_row></db_col>", e)
                        }
                    }
                }
                7 => {
                    if args.len() < 3{
                        return not_enough_arguments(3, args.len());
                    }
                    let query = "INSERT INTO LessonHours (lesson_num, start_time, end_time) VALUES (?1, ?2, ?3)
                        ON CONFLICT (lesson_num) DO UPDATE SET start_time = excluded.start_time, end_time = excluded.end_time";
                    let (lesson_num, start_time, end_time) = 
                        (str::parse::<u8>(args[0].trim()), str::parse::<u16>(args[1].trim()), str::parse::<u16>(args[2].trim()));
                    if lesson_num.is_ok()&start_time.is_ok()&end_time.is_ok() == true{
                        let database = db.lock().await;
                        match database.execute(&query, [lesson_num.unwrap().into(), start_time.unwrap(), end_time.unwrap()]){
                            Ok(_) => return "<db_col><db_row><p>Sukces/Success</p></db_row></db_col>".to_string(),
                            Err(e) => return format!("<db_col><db_row><p>Błąd/Error {}</p></db_row></db_col>", e)
                        }
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
    if let Err(error) = tcp.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n<h1>404 - Not Found</h1>"){
        cli::print_error("Error Occured while sending 404 to client", error);
    }
    else{
        cli::debug_log("Returned 404 to Client");
    }
}
fn not_enough_arguments(number_required: u8, number_entered: usize) -> String{
    return format!("<db_col><db_row><p>Użytkownik użył za małej liczby argumentów, oczekiwano: {number_required}, znaleziono: {number_entered}</p><p>User provided too little arguments, expected: {number_required}, found: {number_entered}</p></db_row></db_col>")
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
#[allow(dead_code)]
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

fn weekd_to_string(weekd: u8) -> String{
    match weekd{
        1 => "Mon.".to_string(),
        2 => "Tue.".to_string(),
        3 => "Wed.".to_string(),
        4 => "Thr.".to_string(),
        5 => "Fri.".to_string(),
        6 => "Sat.".to_string(),
        7 => "Sun.".to_string(),
        _ => "Unk.".to_string()
    }
}
