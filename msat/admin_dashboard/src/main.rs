///====================================
///         admin_dashboard
/// This file is responsible for http server
/// made from scratch with TCP protocol
///=========================================

// Global imports
use std::{
    io::{Read, Write}, net::{IpAddr, TcpListener, TcpStream}, sync::Arc
};
use tokio::sync::Mutex;
use rusqlite::{self, OpenFlags};

// Local Imports 
use shared_components::{
    cli::{
        self, ERROR, SUCCESS
    }, database::init, password::get_password, split_string_by, types::*, LOCAL_IP, SQLITE_FLAGS
};

#[tokio::main]
#[allow(warnings)]
async fn main(){
    init_httpserver(*LOCAL_IP).await;
}

pub async fn init_httpserver(ip_addr: IpAddr) {
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
    let final_address = format!("{}:8000", ip_addr.to_string());
    let listener: TcpListener = match TcpListener::bind(final_address) {
        Ok(v) => v,
        Err(_) => std::process::exit(-1),
    };
    println!("Initialized HTTP SERVER");
    loop {
        for s in listener.incoming() {
            println!("REQUEST Incoming");
            if let Ok(stream) = s{
                let cloned_dbptr = Arc::clone(&database);
                tokio::spawn(async {
                    handle_connection(stream, cloned_dbptr).await;
                });
            }
            else if let Err(error) = s{
                eprintln!("TCPStream is Err: {}", error);
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
                                    Ok(_) => print!("\n---\nResponsed To Request"),
                                    Err(_) => print!("\n---\nCouldn't Respond")
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
                if let Some(v) = get_password().await{
                    if password != v{
                        return "<db_col><db_row><p>Złe Hasło/Wrong Password</p></db_row></db_col>".to_string();
                    }
                }
                else{
                    return "<db_col><db_row><p>Błąd odczytu hasła/Error getting password</p></db_row></db_col>".to_string()
                }
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
                                <p id = \"1\">Week Day</p>
                                <p id = \"2\">Teacher ID</p>
                                <p id = \"3\">Class ID</p>
                                <p id = \"4\">Classroom ID</p>
                                <p id = \"5\">Subject ID</p>
                                <p id = \"6\">Lesson Hour</p>
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
                                <p id='1'>Lesson Hour</p>
                                <p id='2'>Teacher ID</p>
                                <p id='3'>Classroom ID</p>
                                <p id='4'>Week Day</p>
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
                2 => {
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
