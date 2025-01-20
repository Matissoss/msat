//=========================================
//              admin_dashboard
//  This part is responsible for handling 
//  requests sent from browsers
//=========================================

// Global imports
use std::{
    net::IpAddr,
    collections::BTreeMap,
    sync::Arc
};
use tokio::{
    time::{
        self,
        Duration
    },
    net::{
        TcpListener,
        TcpStream,
    },
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    sync::{
        Mutex, 
        Semaphore
    }
};

// Local Imports 
use shared_components::{
    backend::{
        self, 
        get_config, 
        get_duties_for_teacher, 
        get_lessons_by_class_id, 
        get_lessons_by_teacher_id, 
        manipulate_database, 
        MainpulationType, 
        Request, 
        RequestType
    }, 
    consts::*, 
    types::*, 
    visual
};

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
                (c.http_server.ip, 
                 c.http_server.port, c.http_server.max_connections,
                 Arc::new(c.http_server.max_timeout_seconds.into()),
                 Arc::new(c.language))
        }
        None => {
            (*LOCAL_IP, 8000, 100, Arc::new(10), Arc::new(Language::default()))
        }
    };
    let limit = Arc::new(Semaphore::new(max_limit.into()));
    let final_address = format!("{}:{}", ip, port);
    visual::debug(&final_address);
    let listener: TcpListener = match TcpListener::bind(final_address).await {
        Ok(v) => v,
        Err(_) => std::process::exit(-1),
    };
    visual::success("Initialized HTTP Server");
    loop {
        if let Ok((stream, socketaddr)) = listener.accept().await {
            visual::debug(&format!("Request Incoming from {}:{}", socketaddr.ip(), socketaddr.port() ));
                    let cloned_dbptr    = Arc::clone(&database);
                    let cloned_permit   = Arc::clone(&limit);
                    let cloned_timeout  = Arc::clone(&max_timeout);
                    visual::debug("start operation");
                    if let Ok(Ok(perm)) = time::timeout(Duration::from_secs(*cloned_timeout), cloned_permit.acquire_owned()).await{
                        visual::debug("got permission");
                        let lang = Arc::clone(&lang);
                        tokio::spawn(async move {
                            handle_connection(stream, cloned_dbptr, Arc::clone(&lang)).await;
                        });
                        drop(perm);
                    }
                    else{
                        visual::debug("didn't got permission");
                    }
        }
    }
}
pub async fn handle_connection(mut stream: TcpStream, db_ptr: Arc<Mutex<rusqlite::Connection>>, lang: Arc<Language>) {
    visual::debug("start of processing");
    let mut buffer : [u8; 2048] = [0u8; 2048];
    if let Ok(len) = stream.read(&mut buffer).await {
        if len == 0 {
        }
        else{
        visual::debug("start");
        let request = String::from_utf8_lossy(&buffer[0..len]).to_string();
        visual::debug(&request);
        if *DEBUG_MODE{
            for l in request.lines(){
                if !l.is_empty()
                {
                        visual::debug(l);
                }
            }
        }
            let lines = request
                .lines()
                .filter(|s| !s.is_empty())
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
                        // Weird rust_analyzer error
                        #[allow(warnings)]
                        if w == "/" || w.starts_with("/?lang"){
                            visual::debug("./web/index.html");
                            file_path = "./web/index.html".to_string();
                        }
                        else {
                            if !w.starts_with("/?"){
                                file_path = format!("./web{}", w)
                            }
                            else if w.starts_with("/?msat") && !w.starts_with("/?lang="){
                                let cloned_dbptr = Arc::clone(&db_ptr);
                                let cloned_lang = Arc::clone(&lang);
                                let response = handle_custom_request(&w, cloned_dbptr, cloned_lang).await;
                                match stream.write_all(
                                    format!("HTTP/1.1 200 OK\r\nContent-Length:{}\r\nContent-Type: application/xml\r\n\r\n{}",
                                        response.len(), response).as_bytes()).await
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
            if types.is_empty(){
                types = vec!["*/*".to_string()];
            }
            // End of checks
            let binary: bool = types[0].starts_with("image") || types[0].starts_with("font") || file_path.ends_with(".ttf");
            let f_type = &types[0];
            visual::debug(&format!("file_path = {}", file_path));
            if !binary{
                if let Ok(buf) = tokio::fs::read(&file_path).await {
                    if let Ok(string) = String::from_utf8(buf.clone()) {
                        if let Err(err) = 
                            stream.write_all(
                                format!("HTTP/1.1 200 OK\r\nContent-Length:{}\r\nContent-Type:{}\r\n\r\n{}",
                                    string.len(), 
                                    f_type, 
                                    string
                                )
                                .as_bytes()
                            ).await
                        {
                            visual::error(Some(err), "Handled request");
                        };
                    } 
                    else {
                        not_found(&mut stream).await;
                    }
                }
            } 
            else 
            {
                #[allow(warnings)]
                if let Ok(buf) = tokio::fs::read(file_path).await {
                    let http_header = 
                    format!("HTTP/1.1 200 OK\r\nContent-Length:{}\r\nContent-Type:{}\r\nConnection: keep-alive\r\n\r\n",
                                    buf.len(), f_type);
                    if let Err(err) = stream.write_all(http_header.as_bytes()).await{
                        visual::error(Some(err), "Coudln't handle request");
                    };
                    let mut vector = Vec::with_capacity(buf.len() + http_header.len());
                    vector.extend_from_slice(buf.as_slice());
                    vector.extend_from_slice(http_header.as_bytes());
                    if let Err(err) = stream.write_all(&buf).await{
                        visual::error(Some(err), "Coudln't handle request");
                    };
                } 
                else {
                    not_found(&mut stream);
                }
            }
        }
    }
}
async fn handle_custom_request(request: &str, db: Arc<Mutex<rusqlite::Connection>>, lang: Arc<Language>) -> String{
    // request example: /?msat/version&method=POST+1&version=10&args=20
    
    let parsed_request = match Request::from_str(request).parse(){
        Ok(v) => v,
        Err(_) => {
            return lang.english_or("<error><p>Server couldn't parse request</p></error>", 
                "<error><p>Serwer nie mógł przetworzyć zapytania</p></error>");
        }
    };
    #[allow(warnings)]
    if let (Some(pswd), Some(set_pswd)) = (parsed_request.args.get("password"), get_config().await.and_then(|s| Some(s.password))){
        if pswd != &set_pswd || pswd.is_empty() && parsed_request.req_type != RequestType::Other("PAS".to_string()) && parsed_request.req_numb != 0{
                return lang.english_or("<error><p>Bad password</p></error>", "<error><p>Złe hasło</p></error>")
        }
    }
    else{
        return lang.english_or("<error><p>Authentication Error</p></error>", "<error><p>Błąd autentyntykacji</p></error>")
    }

    let args = parsed_request.args;

    match parsed_request.req_type{
        RequestType::GET => {
            match parsed_request.req_numb{
                1 => {
                    if let Some(class_id) = args.get("class_id") 
                    {
                        if let Ok(class) = class_id.parse::<u16>()
                        {
                            if let Ok(lessons) = get_lessons_by_class_id(class, &*db.lock().await){
                                type LessonData = (String, String, String, String, String);
                                let mut unwrapped_lessons : BTreeMap<(u8, u16), LessonData> = 
                                    BTreeMap::new();
                                for lesson in lessons{
                                    if let (Some(teacher), Some(classroom), Some(subject), Some(lessonh), Some(weekd), 
                                        Some(start_hour), Some(start_minute), Some(end_hour), Some(end_minute)) = 
                                    (lesson.teacher, lesson.classroom, lesson.subject, lesson.lessonh.lesson_hour, 
                                     lesson.weekday, lesson.lessonh.start_hour, lesson.lessonh.start_minute, 
                                     lesson.lessonh.end_hour, lesson.lessonh.start_minute)
                                    {
                                        unwrapped_lessons.insert(
                                        (weekd, lessonh), 
                                        (subject, classroom, teacher, 
                                         format!("{:2}:{:2}", start_hour, start_minute), 
                                         format!("{:2}:{:2}", end_hour, end_minute))
                                        );
                                    }
                                }
                                let mut current_weekd : u8 = 0;
                                let mut to_return     : String = "<ltable>".to_string();
                                
                                for (weekd, lessonh) in unwrapped_lessons.keys(){
                                    if &current_weekd != weekd{
                                        if current_weekd != 0{
                                            to_return.push_str("<wd>");
                                        }
                                        else{
                                            to_return.push_str("</wd><wd>");
                                        }
                                        current_weekd = *weekd;
                                    }
                                    if let Some((subject, classroom, teacher, start, end)) = 
                                        unwrapped_lessons.get(&(*weekd, *lessonh))
                                    {
                                        to_return.push_str(&format!("<les><p>{}</p><p>{}</p><p>{}</p><p>{}</p><p>{}</p></les>", 
                                                subject, classroom, teacher, start, end));
                                    }
                                }
                                to_return.push_str("</ltable>");
                                return to_return
                            }
                        }
                    }
                }
                2 => {
                    if let Some(teacher_str) = args.get("teacher_id")
                    {
                        if let Ok(teacher_id) = teacher_str.parse::<u16>()
                        {
                            if let Ok(duties) = get_duties_for_teacher(teacher_id, &*db.lock().await){
                                let mut filtered : BTreeMap<u16, (String, String, String)> = BTreeMap::new();
                                for d in duties{
                                    if let (Some(place), Some(breakn), Some(starth), Some(startm), Some(endh), Some(endm)) = 
                                    (d.place, d.break_num.lesson_hour, d.break_num.start_hour, d.break_num.start_minute, 
                                     d.break_num.end_hour, d.break_num.end_minutes)
                                    {
                                        filtered.insert(breakn, (place, format!("{:2}:{:2}", starth, startm), format!("{:2}:{:2}", endh, endm)));
                                    }
                                }
                                if filtered.is_empty(){
                                    return lang.english_or("<p>You don't have duties today!</p>", "<p>Nie masz dzisiaj dyżuru!</p>");
                                }
                                let mut to_return = "<duties>".to_string();
                                for breakn in filtered.keys(){
                                    if let Some((place, start, end)) = filtered.get(breakn){
                                        to_return.push_str(&format!("<entry><p>{}</p><p>{} - {}</p></entry>", place, start, end));
                                    }
                                }
                                return to_return;
                            }
                        }
                    }
                }
                3 => {
                    if let Some(teacherid_str) = args.get("teacher_id"){
                        if let Ok(teacher_id) = teacherid_str.parse::<u16>(){
                            if let Ok(lessons) = get_lessons_by_teacher_id(teacher_id, &*db.lock().await){
                                type LessonData = (String, String, String, String);
                                let mut unwrapped_lessons : BTreeMap<(String, u8, u16), LessonData> = 
                                    BTreeMap::new();
                                for lesson in lessons{
                                    if let (Some(class), Some(classroom), Some(subject), Some(lessonh), Some(weekd), 
                                        Some(start_hour), Some(start_minute), Some(end_hour), Some(end_minute)) = 
                                    (lesson.class, lesson.classroom, lesson.subject, lesson.lessonh.lesson_hour, 
                                     lesson.weekday, lesson.lessonh.start_hour, lesson.lessonh.start_minute, 
                                     lesson.lessonh.end_hour, lesson.lessonh.start_minute)
                                    {
                                        unwrapped_lessons.insert(
                                        (class, weekd, lessonh), 
                                        (subject, classroom, 
                                         format!("{:2}:{:2}", start_hour, start_minute), 
                                         format!("{:2}:{:2}", end_hour, end_minute))
                                        );
                                    }
                                }
                                let mut current_class : String = "".to_string();
                                let mut current_weekd : u8 = 0;
                                let mut to_return     : String = "<ltable>".to_string();
                                
                                for (class, weekd, lessonh) in unwrapped_lessons.keys(){
                                    if &current_class != class{
                                        current_weekd = 0;
                                        if current_class.is_empty(){
                                            to_return.push_str("<cla>");
                                        }
                                        else{
                                            to_return.push_str("</cla><cla>");
                                        }
                                        current_class = class.clone();
                                    }
                                    if &current_weekd != weekd{
                                        if current_weekd != 0{
                                            to_return.push_str("<wd>");
                                        }
                                        else{
                                            to_return.push_str("</wd><wd>");
                                        }
                                        current_weekd = *weekd;
                                    }
                                    if let Some((subject, classroom, start, end)) = 
                                        unwrapped_lessons.get(&(class.to_string(), *weekd, *lessonh))
                                    {
                                        to_return.push_str(&format!("<les><p>{}</p><p>{}</p><p>{}</p><p>{}</p></les>", 
                                                subject, classroom, start, end));
                                    }
                                }
                                to_return.push_str("</ltable>");
                                return to_return
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
                                Ok(v) => return v,
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
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
                                Ok(v) => return v,
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
                                }
                            };
                        }
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
                                Ok(v) => return v,
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
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
                                Ok(v) => return v,
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
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
                                Ok(v) => return v,
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
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
                                Ok(v) => return v,
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
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
                                Ok(v) => return v,
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
                                Ok(v) => return v,
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
                                Ok(v) => return v,
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
                                Ok(v) => return v,
                                Err(error) => {
                                    visual::error(Some(error), "Database Error");
                                }
                            }
                        }
                    }
                }
                // Corridors
                11 => {
                    if let (Some(teacherid_str), Some(teacher_name)) = (args.get("place_id"), args.get("place_name")){
                        if let Ok(teacher_id) = teacherid_str.parse::<u16>(){
                            match manipulate_database(
                                MainpulationType::Insert(backend::POST::Corridors(Some ((teacher_id, teacher_name.to_string())) )), &*db.lock().await)
                            {
                                Ok(v) => return v,
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
        RequestType::Other(val) => {
            #[allow(warnings)]
            match (val.as_str(), parsed_request.req_numb){
                ("PAS", 0) => {
                    if let (Some(password), Some(set_password)) = (&args.get("password"), get_config().await.and_then(|s| Some(s.password))){
                        return (**password == set_password).to_string();
                    }
                    todo!()
                }
                _ => {}
            }
        }
        _ => {}
    }


    lang.english_or(
        "<error><p>We coudln't get any data from server</p></error>", 
        "<error><p>Nie byliśmy w stanie zdobyć żadnych informacji</p></error>")
    
}

async fn not_found(tcp: &mut TcpStream) {
    if let Err(error) = tcp.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n<h1>404 - Not Found</h1>").await{
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

pub fn weekd_to_string(lang: &Language, weekd: u8) -> String{
    match weekd{
        1 => lang.english_or("Monday"   ,"Poniedziałek" ),
        2 => lang.english_or("Tuesday"  ,"Wtorek"       ),
        3 => lang.english_or("Wednesday","Środa"        ),
        4 => lang.english_or("Thursday" ,"Czwartek"     ),
        5 => lang.english_or("Friday"   ,"Piątek"       ),
        6 => lang.english_or("Saturday" ,"Sobota"       ),
        7 => lang.english_or("Sunday"   ,"Niedziela"    ),
        _ => lang.english_or("Unknown"  ,"Nieznany"     ),
    }
}
