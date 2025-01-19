//=================================
//           app_server
// This part is responsible
// for handling client requests.
//=================================
//            Credits
// This file was coded by MateusDev
// and is licensed under 
// X11 (MIT) License.
//=================================

// Global Imports
use std::{ 
    io::{
        Read, Write
    }, 
    net::{
        TcpListener, TcpStream
    }, 
    sync::Arc
};
use tokio::{
    sync::{
        Mutex, 
        Semaphore
    },
    time::{
        self,
        Duration
    }
};
use rusqlite::Connection as SQLite;
use colored::Colorize;

// Local Imports

use shared_components::{
    backend::{
        self, get_config, get_lessons_by_teacher_id, manipulate_database, MainpulationType, ParsedRequest, Request, RequestType
    }, 
    consts::*, 
    types::*, 
    utils, 
    visual
};

use shared_components::types::ServerError;

// Entry point
#[tokio::main]
async fn main() {
    visual::main();
    if std::process::Command::new(CLEAR).status().is_err(){
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
    let (ip, port, max_connections, max_timeout) = match get_config().await{
        Some(v) => (v.application_server.ip, v.application_server.port, v.application_server.max_connections, v.application_server.max_timeout_seconds),
        None => (*LOCAL_IP, 8888, 100, 10)
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
            visual::info(&format!("This is your public ip: {}", public_ip));
            visual::info(&format!("This Code should be entered by clients: {}", invite_code));
        }
        if let Err(error) = tokio::fs::write("invite.code", invite_code).await{
            visual::error(Some(error), "Error occured while saving to file 'data/invite.code'");
        }
        else{
            visual::success("Successfully saved to data/invite.code");
        }
    }

    let listener : TcpListener = match TcpListener::bind
        (format!("{}:{}", ip, port))
        {
            Ok(v) => v,
            Err(e) => 
            {
                visual::critical_error(Some(e), &format!("Error connecting to ip_address {}", ip));
            }
    };
    

    visual::debug(&format!("Listening on {}:8888", ip));
    
    // Start of actual program
    start_listening(listener, db, limit, Arc::new(max_timeout.into())).await;
    

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
                if let Ok(Ok(perm)) = time::timeout(Duration::from_secs(*cloned_timeout), cloned_limit.acquire_owned()).await{
                    let shared_db = Arc::clone(&db);
                    tokio::spawn(
                        async move{
                            if let Err(error) = handle_connection(stream, shared_db).await{
                                visual::error(Some(error.to_response()), "Error occured while handling exception");
                            }
                            else{
                                visual::success(&format!("Successfully handled request from TCP Addr: {}:{}", ip_address, port))
                            };
                        }
                    );
                    drop(perm);
                }
            }
            else{
                visual::error::<u8>(None, "TCPStream is None");
            }
        }
    }
}

async fn handle_connection(mut stream: TcpStream, db: Arc<Mutex<SQLite>>) -> Result<(), ServerError>{
    let mut data_sent = [0u8; 2048];
    let len = if let Ok(len) = stream.read(&mut data_sent){
        len
    }
    else{
        return Err(ServerError::ReadRequestError);
    };

    if let Ok(request) = Request::from_str(String::from_utf8_lossy(&data_sent[..len]).as_ref()).parse(){
        let response = match get_response(request, db).await{
            Ok(v) => v,
            Err(err) => return Err(err)
        };
        if stream.write_all(response.as_bytes()).is_err(){
            return Err(ServerError::WriteRequestError);
        }
        else{
            visual::success("Handled Request");
        }
    }
    Ok(())
}

async fn get_response(parsed_request: ParsedRequest, db: Arc<Mutex<SQLite>>) -> Result<String, ServerError>{
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
                                    if error == rusqlite::Error::QueryReturnedNoRows{
                                        return Ok("msat/204-No-Content".to_string());
                                    }
                                    return Err(ServerError::DatabaseError(error));
                                }
                            }
                        }
                        else{
                            return Err(ServerError::ParseArgError { args: [class_id, weekday, lesson_hour, semester, academic_year].iter().map(|s| s.to_string()).collect()});
                        }
                    }
                    else {
                        return Err(ServerError::ArgsMissing { 
                            expected: ["class_id", "weekday", "lesson_hour", "semester", "academic_year"].iter().map(|s| s.to_string()).collect() });
                    }
                }
                2 => {
                    if let (Some(weekd_str), Some(break_num_str), Some(teacher_str), Some(semester_str), Some(year_str)) 
                    = (args.get("weekday"), args.get("break_num"), args.get("teacher_id"), args.get("semester"), args.get("academic_year")) 
                    {
                        if let (Ok(weekd), Ok(break_num), Ok(teacher_id), Ok(semester), Ok(academic_year))
                        = (weekd_str.parse::<u8>(), break_num_str.parse::<u8>(), teacher_str.parse::<u16>(), 
                            semester_str.parse::<u8>(), year_str.parse::<u8>())
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
                                    return Err(ServerError::DatabaseError(error));
                                }
                            }
                        }
                        else{
                            return Err(ServerError::ParseArgError { args: [weekd_str, break_num_str, teacher_str, semester_str, year_str].iter().map(|s| s.to_string()).collect() });
                        }
                    }
                    else{
                        return Err(ServerError::ArgsMissing { expected: ["weekday", "break_num", "teacher_id", "semester", "academic_year"]
                            .iter().map(|s| s.to_string()).collect() });
                    }
                }
                3 => {
                    if let Some(teacherid_str) = args.get("teacher_id"){
                        if let Ok(teacher_id) = teacherid_str.parse::<u16>(){
                            match get_lessons_by_teacher_id(teacher_id, &*db.lock().await){
                                Ok(vec) => {
                                    let mut to_return = String::new();
                                    to_return.push_str("msat/200-OK");
                                    let mut amount = 0;
                                    for lesson in vec{
                                        if let Some(lessonh) = lesson.lessonh.lesson_hour{
                                            if amount < lessonh{
                                                amount = lessonh;
                                            }
                                            if let Some(class_id) = lesson.class{
                                                to_return.push_str(&format!("&class{}={}", lessonh, class_id.to_single('_')));
                                            }
                                            if let Some(classroom_id) = lesson.classroom{
                                                to_return.push_str(&format!("&classroom{}={}", lessonh, classroom_id.to_single('_')));
                                            }
                                            if let Some(subject) = lesson.subject{
                                                to_return.push_str(&format!("&subject{}={}", lessonh, subject.to_single('_')));
                                            }
                                            if let (Some(start_hour), Some(start_minute)) = (lesson.lessonh.start_hour, lesson.lessonh.start_minute){
                                                to_return.push_str(&format!("&start_date{}={:02}:{:02}", lessonh, start_hour, start_minute));
                                            }
                                            if let (Some(end_hour), Some(end_minute)) = (lesson.lessonh.end_hour, lesson.lessonh.end_minutes){
                                                to_return.push_str(&format!("&end_date{}={:02}:{:02}", lessonh, end_hour, end_minute));
                                            }
                                        }
                                    }
                                    to_return.push_str(&format!("&AMOUNT={}",amount));
                                    return Ok(to_return);
                                }
                                Err(error) => {
                                    if error == rusqlite::Error::QueryReturnedNoRows{
                                        return Ok("msat/204-No-Content".to_string());
                                    }
                                    return Err(ServerError::DatabaseError(error));
                                }
                            }

                        }
                        else{
                            return Err(ServerError::ParseIntError { arg: teacherid_str.to_string() });
                        }
                    }
                    else{
                        return Err(ServerError::ArgsMissing { expected: ["teacher_id"].iter().map(|s| s.to_string()).collect() });
                    }
                }
                _ => {}
            }
        }
        RequestType::POST => {
            match parsed_request.req_numb{
                // Lesson
                1 => {
                    if let (Some(weekday_str), Some(classid_str), Some(classroomid_str), 
                        Some(teacherid_str), Some(subjectid_str), Some(semester_str),
                        Some(academicyear_str), Some(lessonhour_str)) = 
                        (args.get("weekday"), args.get("class_id"), args.get("classroom_id"), args.get("teacher_id"), 
                         args.get("subject_id"), args.get("semester"), args.get("academic_year"), args.get("lesson_hour"))
                    {
                        if let (Ok(weekday), Ok(class_id), Ok(classroom_id), Ok(teacher_id), Ok(subject_id), Ok(semester), Ok(academic_year),
                            Ok(lesson_hour)) = 
                        (weekday_str.parse::<u8>(),classid_str.parse::<u16>(),classroomid_str.parse::<u16>(),teacherid_str.parse::<u16>(),
                        subjectid_str.parse::<u16>(), semester_str.parse::<u8>(), academicyear_str.parse::<u8>(), lessonhour_str.parse::<u16>())
                        {
                            match manipulate_database(
                            MainpulationType::Insert
                            (backend::POST::Lesson
                            (Some((weekday, class_id, classroom_id, teacher_id, subject_id, lesson_hour, semester, academic_year)))), &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    return Err(ServerError::DatabaseError(error));
                                }
                            }
                        }
                    }
                    else{
                        return Err(ServerError::ArgsMissing { 
                            expected: [
                            "weekday", 
                            "class_id", 
                            "classroom_id", 
                            "teacher_id", 
                            "subject_id", 
                            "semester", 
                            "academic_year", 
                            "lesson_hour"].iter().map(|s| s.to_string()).collect() 
                        });
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
                                    return Err(ServerError::DatabaseError(error));
                                }
                            };
                        }
                        else{
                            return Err(ServerError::ParseIntError { arg: academicyear_str.to_string() });
                        }
                    }
                    else{
                        return Err(ServerError::ArgsMissing { expected: ["academic_year", "year_name", "start_date", "end_date"].iter().map(|s| s.to_string()).collect() });
                    }
                }
                // Duty
                3 => {
                    if let (Some(weekday_str), Some(breaknum_str), Some(teacherid_str),Some(semester_str), Some(academicyear_str), Some(placeid_str)) = 
                    (args.get("weekday"), args.get("break_num"), args.get("teacher_id"), 
                     args.get("semester"), args.get("academic_year"), args.get("place_id"))
                    {
                        if let (Ok(weekday), Ok(break_num), Ok(teacher_id), Ok(semester), Ok(academic_year), Ok(place_id)) = 
                        (weekday_str.parse::<u8>(),breaknum_str.parse::<u8>(),teacherid_str.parse::<u16>(),
                        semester_str.parse::<u8>(), academicyear_str.parse::<u8>(),placeid_str.parse::<u16>())
                        {
                            match manipulate_database(MainpulationType::Insert
                                (backend::POST::Duty(Some((weekday, break_num, teacher_id, place_id, semester, academic_year)))), 
                                &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    return Err(ServerError::DatabaseError(error));
                                }
                            }
                        }
                        else{
                            return Err(ServerError::ParseArgError { 
                                args: [weekday_str, breaknum_str, teacherid_str, semester_str, academicyear_str, placeid_str].iter().map(|s| s.to_string()).collect() 
                            });
                        }
                    }
                    else{
                        return Err(ServerError::ArgsMissing { expected: ["weekday", "break_num", "teacher_id", "semester", "academic_year", "place_id"]
                            .iter().map(|s| s.to_string()).collect() });
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
                                    return Err(ServerError::DatabaseError(error));
                                }
                            }
                        }
                        else{
                            return Err(ServerError::ParseArgError { args: [breaknum_str, starthour_str, startminute_str, endhour_str, endminute_str]
                                .iter().map(|s| s.to_string()).collect() });
                        }
                    }
                    else{
                        return Err(ServerError::ArgsMissing { expected: ["break_num", "start_hour", "start_minute", "end_hour", "end_minute"]
                            .iter().map(|s| s.to_string()).collect() });
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
                                    return Err(ServerError::DatabaseError(err));
                                }
                            }
                        }
                        else{
                            return Err(ServerError::ParseIntError { arg: semester_str.to_string() });
                        }
                    }
                    else{
                        return Err(ServerError::ArgsMissing { expected: ["semester", "semester_name", "start_date", "end_date"].iter().map(|s| s.to_string()).collect() });
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
                                    return Err(ServerError::DatabaseError(error));
                                }
                            }
                        }
                        else{
                            return Err(ServerError::ParseArgError { args: [lessonnum_str, starthour_str, startminute_str, endhour_str, endminute_str]
                                .iter().map(|s| s.to_string()).collect() });
                        }
                    }
                    else{
                        return Err(ServerError::ArgsMissing { expected: ["lesson_num", "start_hour", "start_minute", "end_hour", "end_minute"]
                            .iter().map(|s| s.to_string()).collect() });
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
                                    return Err(ServerError::DatabaseError(error));
                                }
                            }
                        }
                        else{
                            return Err(ServerError::ParseIntError { arg: teacherid_str.to_string() });
                        }
                    }
                    else{
                        return Err(ServerError::ArgsMissing { expected: ["teacher_id", "teacher_name"].iter().map(|s| s.to_string()).collect() });
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
                                    return Err(ServerError::DatabaseError(error));
                                }
                            }
                        }
                    }
                    else{
                        return Err(ServerError::ArgsMissing { expected: ["class_id", "class_name"].iter().map(|s| s.to_string()).collect() });
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
                                    return Err(ServerError::DatabaseError(error));
                                }
                            }
                        }
                        else{
                            return Err(ServerError::ParseIntError { arg: teacherid_str.to_string() });
                        }
                    }
                    else{
                        return Err(ServerError::ArgsMissing { expected: ["classroom_id", "classroom_name"].iter().map(|s| s.to_string()).collect() });
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
                                    return Err(ServerError::DatabaseError(error));
                                }
                            }
                        }
                        else{
                            return Err(ServerError::ParseIntError { arg: teacherid_str.to_string() });
                        }
                    }
                    else{
                        return Err(ServerError::ArgsMissing { expected: ["subject_id", "subject_name"].iter().map(|s| s.to_string()).collect() });
                    }
                }
                // Corridors
                11 => {
                    if let (Some(teacherid_str), Some(teacher_name)) = (args.get("place_id"), args.get("place_name")){
                        if let Ok(teacher_id) = teacherid_str.parse::<u16>(){
                            match manipulate_database(
                                MainpulationType::Insert(backend::POST::Corridors(Some ((teacher_id, teacher_name.to_string())) )), &*db.lock().await)
                            {
                                Ok(v) => return Ok(v),
                                Err(error) => {
                                    return Err(ServerError::DatabaseError(error));
                                }
                            }
                        }
                        else{
                            return Err(ServerError::ParseIntError { arg: teacherid_str.to_string() });
                        }
                    }
                    else{
                        return Err(ServerError::ArgsMissing { expected: ["place_id", "place_name"].iter().map(|s| s.to_string()).collect() });
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    Err(ServerError::UnknownRequest)
}
