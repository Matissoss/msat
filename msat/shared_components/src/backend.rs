///============================
/// Contains core of msat's
/// backend logic like parsing
/// requests and handling them
///============================

// Global Imports
use rusqlite::{
    Connection as Database,
    OpenFlags  as Flags,
    Error      as SQLiteError
};
use chrono::{Timelike,Datelike};
use tokio::{
    fs, sync::{
        MutexGuard,
        Mutex,
        Semaphore
    }
};
use toml;
use std::{
    collections::HashMap, sync::{
        Arc, LazyLock
    }
};
// Local Imports 
use crate::{consts::VERSION, visual};
use crate::types::*;
// static/const declaration
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Timestamp{
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    secs: u8
}
impl Timestamp{
    fn can_proceed(&self, timeout_secs: u32) -> bool{
        let now : u32 = (self.hour as u32 * 60 * 60) + (self.minute as u32 * 60) + self.secs as u32;
        Timestamp::from(now + timeout_secs) < Timestamp::now()
    }
    fn now() -> Timestamp{
        let now = &chrono::Local::now();
        Timestamp{
            year  : (now.year  () & 0xFFFF).try_into().unwrap(),
            month : (now.month () & 0xFF).try_into().unwrap(),
            day   : (now.day   () & 0xFF).try_into().unwrap(),
            hour  : (now.hour  () & 0xFF).try_into().unwrap(),
            minute: (now.minute() & 0xFF).try_into().unwrap(),
            secs  : (now.second() & 0xFF).try_into().unwrap()
        }
    }
}
impl From<u32> for Timestamp{
    fn from(value: u32) -> Self {
        let hour = (value / 3600) as u8;
        let minute = ((value % 3600) / 60) as u8;
        let second = (value % 60) as u8;
        Self{
            year: (chrono::Local::now().year() & 0xFFFF).try_into().unwrap(),
            month: (chrono::Local::now().month() & 0xFF).try_into().unwrap(),
            day: (chrono::Local::now().day() & 0xFF).try_into().unwrap(),
            hour,
            minute,
            secs: second
        }
    }
}

// statics
pub static GLOBAL_STATIC_SEMAPHORE : LazyLock<Arc<Semaphore>> = LazyLock::new(|| {
    Arc::new(Semaphore::new(0))
});
pub static CLASSES : LazyLock<Mutex<u16>> = LazyLock::new(|| {
    Mutex::new(0)
});
pub static LESSONHOURS : LazyLock<Mutex<u16>> = LazyLock::new(|| {
    Mutex::new(0)
});
pub static WEEKDAY : LazyLock<Mutex<u8>> = LazyLock::new(|| {
    Mutex::new(0)
});
pub static TIMEOUT : u32 = 30;

// Struct Initialization

#[allow(unused)]
#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestType{
    GET,
    POST,
    Other(String),
    #[default]
    Unknown
}
#[allow(unused)]
#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Request{
    pub request: String,
}
#[allow(unused)]
#[derive(Default, Clone, PartialEq, Eq)]
pub struct ParsedRequest{
    pub req_type: RequestType,
    pub req_numb: u8,
    pub args: HashMap<String, String>
}

impl Request{
    #[allow(warnings)]
    pub fn from_str(request: &str) -> Request{
        Request{request: request.to_string()}
    }
    pub fn parse(&self) -> Result<ParsedRequest, ServerError>{
        if !self.request.starts_with(&format!("/?msat/{}", VERSION)) || !self.request.starts_with(&format!("msat/{}" , VERSION))
        {
            return Err(ServerError::UnknownRequest);
        }
        let vector = self.request.split('&').collect::<Vec<&str>>();
        if vector.len() == 1{
            return Err(ServerError::UnknownRequest);
        }
        let mut to_return = ParsedRequest::default();
        let mut finhashmap = HashMap::new();
        for word in vector{
            if let Some((key, value)) = word.split_once('='){
                finhashmap.insert(key, value);
            }
        }
        if let Some(value) = finhashmap.get("method"){
            if let Some((method, numb)) = value.split_once('+'){
                match method.to_uppercase().as_str(){
                    "GET"  => to_return.req_type = RequestType::GET,
                    "POST" => to_return.req_type = RequestType::POST,
                    _      => to_return.req_type = RequestType::Other(method.to_string())
                }
                to_return.req_numb = numb.parse().unwrap_or(0);
            }
        }
        Ok(to_return)
    }
}

// Functions

pub async fn get_config() -> Option<Config>{
    return match fs::read_to_string("data/config.toml").await{
        Ok(v) => {
            match toml::from_str::<Config>(&v){
                Ok(conf) => Some(conf),
                Err(_)   => None
            }
        }
        Err(_) => {
            if let Ok(b) = fs::try_exists("data/config.toml").await{
                if !b{
                    if let Err(err) = fs::write("data/config.toml", toml::to_string(&Config::default()).unwrap_or_default()).await{
                        visual::error(Some(err), "couldn't create config.toml file");
                    }
                }
            }
            None
        }
    }
}

pub async fn get_password() -> Option<String>{
    return match fs::read_to_string("config.toml").await{
        Ok(v) => {
            match toml::from_str::<Config>(&v){
                Ok(conf) => {
                    if conf.password.is_empty(){
                        None
                    }
                    else{
                        Some(conf.password)
                    }
                }
                Err(_) => None
            }
        }
        Err(_) => None
    };
}

pub fn init_db() -> Result<Database, SQLiteError>{
    let db = Database::open_with_flags(
        "data/database.db",
        // Enter flags
        Flags::SQLITE_OPEN_FULL_MUTEX|
        Flags::SQLITE_OPEN_READ_WRITE|
        Flags::SQLITE_OPEN_CREATE
    )?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Classes(
            class_id   INTEGER PRIMARY KEY,
            class_name TEXT NOT NULL UNIQUE
        );
        "
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Classrooms(
            classroom_id INTEGER PRIMARY KEY,
            class_name   TEXT NOT NULL UNIQUE
        );"
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Teachers(
            teacher_id   INTEGER PRIMARY KEY,
            teacher_name TEXT NOT NULL UNIQUE
        );"
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Subjects(
            subject_id   INTEGER PRIMARY KEY,
            subject_name TEXT NOT NULL UNIQUE
        );"
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS LessonHours(
            lesson_hour   INTEGER PRIMARY KEY,
            start_hour    INTEGER NOT NULL CHECK(start_hour >= 0 AND start_hour < 24),
            start_minutes INTEGER NOT NULL CHECK(start_minutes >= 0 AND start_minutes < 60),
            end_hour      INTEGER NOT NULL CHECK(end_hour >= 0 AND end_hour < 24),
            end_minutes   INTEGER NOT NULL CHECK(end_minutes >= 0 AND end_minutes < 60)
        );"
        ,[])?;
    // semester_start and semester_end should be formatted in ISO8601 format
    db.execute(
        "CREATE TABLE IF NOT EXISTS Semesters(
            semester      INTEGER PRIMARY KEY,
            semester_name TEXT NOT NULL UNIQUE,
            start_date    TEXT NOT NULL,
            end_date      TEXT NOT NULL
        );"
        ,[])?;
    // start_date and end_date should be formatted as ISO8601 compatible date
    db.execute(
        "CREATE TABLE IF NOT EXISTS Years(
            academic_year  INTEGER PRIMARY KEY,
            year_name      TEXT NOT NULL UNIQUE,
            start_date     TEXT NOT NULL,
            end_date       TEXT NOT NULL
        );"
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Lessons (
            weekday       INTEGER NOT NULL,
            class_id      INTEGER NOT NULL,
            classroom_id  INTEGER NOT NULL,
            teacher_id    INTEGER NOT NULL,
            subject_id    INTEGER NOT NULL,
            lesson_hour   INTEGER NOT NULL,
            semester      INTEGER NOT NULL,
            academic_year INTEGER NOT NULL,
            PRIMARY KEY (class_id, weekday, lesson_hour, semester, academic_year),
            FOREIGN KEY (class_id)      REFERENCES Classes    (class_id),
            FOREIGN KEY (classroom_id)  REFERENCES Classrooms (classroom_id),
            FOREIGN KEY (teacher_id)    REFERENCES Teachers   (teacher_id),
            FOREIGN KEY (subject_id)    REFERENCES Subjects   (subject_id),
            FOREIGN KEY (lesson_hour)   REFERENCES LessonHours(lesson_hour),
            FOREIGN KEY (semester)      REFERENCES Semesters  (semester_num),
            FOREIGN KEY (academic_year) REFERENCES Years      (academic_year)
        );
        "
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Corridors(
            corridor      INTEGER PRIMARY KEY,
            corridor_name TEXT NOT NULL UNIQUE
        );
        "
        ,[])?;
    db.execute(
        "
        CREATE TABLE IF NOT EXISTS Breaks(
            break_num     INTEGER PRIMARY KEY,
            start_hour    INTEGER NOT NULL CHECK(start_hour >= 0 AND start_hour < 24),
            start_minutes INTEGER NOT NULL CHECK(start_minutes >= 0 AND start_minutes < 60),
            end_hour      INTEGER NOT NULL CHECK(end_hour >= 0 AND end_hour < 24),
            end_minutes   INTEGER NOT NULL CHECK(end_minutes >= 0 AND end_minutes < 60)
        );
        "
        ,[])?;
    db.execute(
        "CREATE TABLE IF NOT EXISTS Duties(
            weekday       INTEGER NOT NULL,
            break_num     INTEGER NOT NULL,
            teacher_id    INTEGER NOT NULL,
            place_id      INTEGER NOT NULL,
            semester      INTEGER NOT NULL,
            academic_year INTEGER NOT NULL,
            PRIMARY KEY (weekday, break_num, teacher_id, semester, academic_year),
            FOREIGN KEY (break_num)     REFERENCES Breaks    (break_num),
            FOREIGN KEY (teacher_id)    REFERENCES Teachers  (teacher_id),
            FOREIGN KEY (place_id)      REFERENCES Corridors (corridor),
            FOREIGN KEY (academic_year) REFERENCES Years     (academic_year),
            FOREIGN KEY (semester)      REFERENCES Semesters (semester)
        );
        "
        ,[])?;
    db.execute_batch("PRAGMA journal_mode = WAL")?;
    db.busy_timeout(std::time::Duration::from_secs(4))?;

    Ok(db)
}

pub fn get_year_and_semester(db: &rusqlite::Connection) -> Result<(u8, u8), rusqlite::Error>{
    let mut stmt1 = db.prepare("
    SELECT academic_year 
    FROM Years 
    WHERE start_date < ?1
    AND end_date     > ?1")?;
    let now = chrono::Local::now();
    let year = stmt1.query_row([now.to_rfc3339()], |row|{
        Ok(row.get::<usize, u8>(0).unwrap_or_default())
    })?;
    let mut stmt2 = db.prepare(
    "SELECT semester 
    FROM Semesters 
    WHERE start_date < ?1
    AND end_date     > ?1
    ")?;
    let semester = stmt2.query_row([now.to_rfc3339()], |row|{
        Ok(row.get::<usize, u8>(0).unwrap_or_default())
    })?;
    Ok((year, semester))
}

/// STATIC
pub async fn static_lesson_table(db: &MutexGuard<'_, rusqlite::Connection>) -> Result<Vec<JoinedLessonRaw>, ()>{
    if Timestamp::now().can_proceed(TIMEOUT){
        if let Ok(Ok(permit)) = tokio::time::timeout
        (std::time::Duration::from_secs(5), Arc::clone(&GLOBAL_STATIC_SEMAPHORE).acquire_owned()).await
        {
                // GET all data from Lessons table and take 'snapshot'
                let query = 
                "
                SELECT * FROM Lessons
                ";
                match db.prepare(query){
                    Ok(mut stmt) => {
                        let iter = stmt.query_map([], |row| {
                            Ok(
                                JoinedLessonRaw{
                                    weekday       : row.get(0).ok(),
                                    class         : row.get(1).ok(),
                                    classroom     : row.get(2).ok(),
                                    teacher       : row.get(3).ok(),
                                    subject       : row.get(4).ok(),
                                    lessonh       : row.get(5).ok(),
                                    semester      : row.get(6).ok(),
                                    academic_year : row.get(7).ok()
                                }
                            )
                        });
                        if let Ok(ok_iter) = iter{
                            let mut to_return = vec![];
                            let mut largest_class = 0;
                            let mut largest_lessonh = 0;
                            let mut largest_weekd = 0;
                            for raw_lesson in ok_iter.flatten(){
                                if largest_class < raw_lesson.class.unwrap_or_default(){
                                    largest_class = raw_lesson.class.unwrap_or_default();
                                }
                                if largest_lessonh < raw_lesson.lessonh.unwrap_or_default(){
                                    largest_lessonh = raw_lesson.lessonh.unwrap_or_default();
                                }
                                if largest_weekd < raw_lesson.weekday.unwrap_or_default(){
                                    largest_weekd = raw_lesson.weekday.unwrap_or_default();
                                }
                                to_return.push(raw_lesson);
                            }
                            *LESSONHOURS.lock().await = largest_lessonh;
                            *WEEKDAY.lock().await = largest_weekd;
                            *CLASSES.lock().await = largest_class;
                            match tokio::fs::try_exists("data/static").await{
                                Ok(true) => {}
                                Ok(false)|Err(_) => {
                                    if let Err(error) = tokio::fs::create_dir_all("data/static").await{
                                        visual::error(Some(error), "error occured while creating data/static directory");
                                    };
                                }
                            }
                            for lesson in &to_return{
                                if let 
                                (Some(class), Some(classroom), Some(teacher), Some(subject), Some(lessonh),Some(weekd), Some(academic), Some(sems)) = 
                                    (lesson.class,lesson.classroom,lesson.teacher,lesson.subject,lesson.lessonh,lesson.weekday,lesson.academic_year,
                                lesson.semester)
                                {
                                    match tokio::fs::write(format!("data/static/TEACHER:{}={}-{}|{}-{}",teacher,weekd,lessonh,academic,sems), 
                                        crate::static_data::serialize([class,classroom,subject])
                                    ).await
                                    {
                                        Ok   (_) => visual::debug("saved STATIC data"),
                                        Err(err) => visual::error(Some(err), "Couldn't save STATIC data"),
                                    }
                                    match tokio::fs::write(format!("data/static/CLASS:{}={}-{}|{}-{}", class, weekd, lessonh,academic,sems),
                                        crate::static_data::serialize([classroom,subject,teacher])).await
                                    {
                                        Ok (_)   => visual::debug("saved STATIC data"),
                                        Err(err) => visual::error(Some(err), "Couldn't save STATIC data"), 
                                    }
                                }
                            }

                            drop(permit);
                            return Ok(to_return);
                        }
                        else{
                            return static_old_table(db).await;
                        }
                    }
                    Err(_) => {
                        return static_old_table(db).await;
                    }
                }
        }
    }
    return static_old_table(db).await;
}
/// STATIC
pub async fn static_old_table(db: &rusqlite::Connection) -> Result<Vec<JoinedLessonRaw>, ()>{
    let mut to_return = vec![];
    let largest_lessonh = &*LESSONHOURS.lock().await;
    let largest_weekd   = &*WEEKDAY.lock().await;
    let largest_class   = &*CLASSES.lock().await;
    let (academic_year, semester) = get_year_and_semester(db).unwrap_or_default();

    for lc in 0..*largest_class{
        for lw in 0..*largest_weekd{
            for lh in 0..*largest_lessonh{
                if let Ok(b) = tokio::fs::read(format!("data/static/CLASS:{}={}-{}|{}-{}",lc,lw,lh,academic_year,semester)).await{
                        let mut slice: [u8; 6] = [0; 6];
                        for (ind, i) in b.into_iter().enumerate(){
                            slice[ind] = i;
                            if ind+1 == 6{
                                break;
                            }
                        }
                        let data = crate::static_data::deserialize(slice);
                        to_return.push(JoinedLessonRaw{
                            weekday: Some(lw),
                            teacher: Some(data[2]),
                            classroom: Some(data[1]),
                            subject: Some(data[0]),
                            lessonh: Some(lh),
                            class: Some(lc),
                            academic_year: Some(academic_year),
                            semester: Some(semester)
                        });
                }
            }
        }
    }

    Ok(to_return)
}

pub fn raw_lessons_to_lesson(vector: Vec<JoinedLessonRaw>, db: &rusqlite::Connection) -> Result<Vec<JoinedLesson>, rusqlite::Error>{
    let mut to_return = vec![];
    let mut already_visited_class     : HashMap<u16, String> = HashMap::new();
    let mut already_visited_classroom : HashMap<u16, String> = HashMap::new();
    let mut already_visited_teacher   : HashMap<u16, String> = HashMap::new();
    let mut already_visited_subject   : HashMap<u16, String> = HashMap::new();
    let mut already_visited_lessonh   : HashMap<u16, JoinedHour> = HashMap::new();
    let mut already_visited_semester  : HashMap<u8, String> = HashMap::new();
    let mut already_visited_year      : HashMap<u8, String> = HashMap::new();
    for element in vector{
        let mut current_to_lesson : JoinedLesson = JoinedLesson::default();
        if let Some(class_id) = element.class{
            if let Some(visited) = already_visited_class.get(&class_id){
                current_to_lesson.class = Some(visited.to_string());
            }
            else{
                let mut stmt = db.prepare("SELECT * FROM Classes WHERE class_id = ?1")?;
                let (id, name) = stmt.query_row([class_id], |row| {
                    Ok((
                        row.get::<usize, u16>(0).ok(), 
                        row.get::<usize, String>(1).ok()
                    ))
                })?;
                already_visited_class.insert(id.unwrap_or_default(), name.clone().unwrap_or_default());
                current_to_lesson.class = name;
            }
        }
        if let Some(classroom_id) = element.classroom{
            if let Some(visited) = already_visited_classroom.get(&classroom_id){
                current_to_lesson.classroom = Some(visited.to_string());
            }
            else{
                let mut stmt = db.prepare("SELECT * FROM Classrooms WHERE classroom_id = ?1")?;
                let (id, name) = stmt.query_row([classroom_id], |row| {
                    Ok((
                        row.get::<usize, u16>(0).ok(), 
                        row.get::<usize, String>(1).ok()
                    ))
                })?;
                already_visited_classroom.insert(id.unwrap_or_default(), name.clone().unwrap_or_default());
                current_to_lesson.classroom = name;
            }
        }
        if let Some(teacher_id) = element.teacher{
            if let Some(visited) = already_visited_teacher.get(&teacher_id){
                current_to_lesson.teacher = Some(visited.to_string());
            }
            else{
                let mut stmt = db.prepare("SELECT * FROM Teachers WHERE teacher_id = ?1")?;
                let (id, name) = stmt.query_row([teacher_id], |row| {
                    Ok((
                        row.get::<usize, u16>(0).ok(), 
                        row.get::<usize, String>(1).ok()
                    ))
                })?;
                already_visited_teacher.insert(id.unwrap_or_default(), name.clone().unwrap_or_default());
                current_to_lesson.teacher = name;
            }
        }
        if let Some(subject_id) = element.subject{
            if let Some(visited) = already_visited_subject.get(&subject_id){
                current_to_lesson.subject = Some(visited.to_string());
            }
            else{
                let mut stmt = db.prepare("SELECT * FROM Subjects WHERE subject_id = ?1")?;
                let (id, name) = stmt.query_row([subject_id], |row| {
                    Ok((
                        row.get::<usize, u16>(0).ok(), 
                        row.get::<usize, String>(1).ok()
                    ))
                })?;
                already_visited_subject.insert(id.unwrap_or_default(), name.clone().unwrap_or_default());
                current_to_lesson.subject = name;
            }
        }
        if let Some(lessonh) = element.lessonh{
            if let Some(visited) = already_visited_lessonh.get(&lessonh){
                current_to_lesson.lessonh = *visited;
            }
            else{
                let mut stmt = db.prepare("SELECT * FROM LessonHours WHERE lesson_hour = ?1")?;
                let (id, start_hour, start_minute, end_hour, end_minute) = stmt.query_row([lessonh], |row| {
                    Ok((
                        row.get::<usize, u16>(0).ok(), 
                        row.get::<usize, u8>(1).ok(),
                        row.get::<usize, u8>(1).ok(),
                        row.get::<usize, u8>(1).ok(),
                        row.get::<usize, u8>(1).ok()
                    ))
                })?;
                let current = JoinedHour{
                    lesson_hour: id,
                    start_hour,
                    start_minute,
                    end_hour,
                    end_minutes: end_minute
                };
                already_visited_lessonh.insert(id.unwrap_or_default(), current);
                current_to_lesson.lessonh = current;
            }
        }
        if let Some(semester) = element.semester{
            if let Some(visited) = already_visited_semester.get(&semester){
                current_to_lesson.semester = Some(visited.to_string());
            }
            else{
                let mut stmt = db.prepare("SELECT * FROM Semesters WHERE semester_num = ?1")?;
                let (id, name) = stmt.query_row([semester], |row| {
                    Ok((
                        row.get::<usize, u8>(0).ok(), 
                        row.get::<usize, String>(1).ok()
                    ))
                })?;
                already_visited_semester.insert(id.unwrap_or_default(), name.clone().unwrap_or_default());
                current_to_lesson.semester = name;
            }
        }
        if let Some(year) = element.academic_year{
            if let Some(visited) = already_visited_year.get(&year){
                current_to_lesson.academic_year = Some(visited.to_string());
            }
            else{
                let mut stmt = db.prepare("SELECT * FROM Years WHERE academic_year = ?1")?;
                let (id, name) = stmt.query_row([year], |row| {
                    Ok((
                        row.get::<usize, u8>(0).ok(), 
                        row.get::<usize, String>(1).ok()
                    ))
                })?;
                already_visited_year.insert(id.unwrap_or_default(), name.clone().unwrap_or_default());
                current_to_lesson.academic_year = name;
            }
        }
        to_return.push(current_to_lesson);
    }
    Ok(to_return)
}

/// DYNAMIC
pub fn get_lessons_by_teacher_id(teacher_id: u16, db: &rusqlite::Connection) -> Result<Vec<JoinedLesson>, rusqlite::Error> {
    let now         = chrono::Local::now();
    let now_iso8601 = now.to_rfc3339();
    let weekd       = now.weekday() as u8 + 1;
    let query = 
    "SELECT 
    Lessons.weekday, Classes.class_name, Classrooms.classroom_name, Subjects.subject_name,
    LessonHours.start_hour, LessonHours.start_minutes, LessonHours.end_hour, LessonHours.end_minutes, Lessons.lesson_hour
    FROM Lessons 
    JOIN Classes     ON Lessons.class_id      = Classes.class_id
    JOIN Classrooms  ON Lessons.classroom_id  = Classrooms.classroom_id 
    JOIN Subjects    ON Lessons.subject_id    = Subjects.subject_id
    JOIN LessonHours ON Lessons.lesson_hour   = LessonHours.lesson_hour
    JOIN Years       ON Lessons.academic_year = Years.academic_year
    JOIN Semesters   ON Lessons.semester      = Semesters.semester
    WHERE Lessons.teacher_id        < ?1 AND Lessons.weekday         = ?2
    AND   Semesters.start_date      < ?5 AND Semesters.end_date      > ?5
    AND   Years.start_date          < ?5 AND Years.end_date          > ?5
    ";
    let mut stmt = db.prepare(query)?;

    let iter = stmt.query_map([teacher_id.to_string(), weekd.to_string(), now_iso8601], |row|{
        Ok(
            JoinedLesson{
                weekday   : row.get(0).ok(),
                teacher   : None,
                class     : row.get(1).ok(),
                classroom : row.get(2).ok(),
                subject   : row.get(3).ok(),
                lessonh   : JoinedHour{
                    lesson_hour  : row.get(8).ok(),
                    start_hour   : row.get(4).ok(),
                    start_minute : row.get(5).ok(),
                    end_hour     : row.get(6).ok(),
                    end_minutes  : row.get(7).ok()
                },
                academic_year : None,
                semester      : None
            }
        )
    })?;
    let mut to_return = vec![];
    for joined_lesson in iter.flatten(){
        to_return.push(joined_lesson);
    }
    Ok(to_return)
}
/// DYNAMIC
pub fn get_lessons_by_class_id(class_id: u16, db: &rusqlite::Connection) -> Result<Vec<JoinedLesson>, rusqlite::Error> {
    let now         = chrono::Local::now();
    let now_iso8601 = now.to_rfc3339();
    let weekd       = now.weekday() as u8 + 1;
    let now_hour    = now.hour  () & 0xFF;
    let now_minute  = now.minute() & 0xFF;
    let query = 
    "SELECT 
    Lessons.weekday, Teachers.teacher_name, Classrooms.classroom_name, Subjects.subject_name,
    LessonHours.start_hour, LessonHours.start_minutes, LessonHours.end_hour, LessonHours.end_minutes,
    Years.start_date, Years.end_date, 
    FROM Lessons 
    JOIN Classrooms  ON Lessons.classroom_id  = Classrooms.classroom_id 
    JOIN Teachers    ON Lessons.teacher_id    = Teachers.teacher_id
    JOIN Subjects    ON Lessons.subject_id    = Subjects.subject_id
    JOIN LessonHours ON Lessons.lesson_hour   = LessonHours.lesson_hour
    JOIN Years       ON Lessons.academic_year = Years.academic_year
    JOIN Semesters   ON Lessons.semester      = Semesters.semester
    WHERE Lessons.class_id          < ?1 AND Lessons.weekday         = ?2
    AND   LessonHours.start_hour    < ?3 AND LessonHours.end_hour    > ?3
    AND   LessonHours.start_minutes < ?4 AND LessonHours.end_minutes > ?4
    AND   Semesters.start_date      < ?5 AND Semesters.end_date      > ?5
    ";
    let mut stmt = db.prepare(query)?;

    let iter = stmt.query_map([class_id.to_string(), weekd.to_string(), now_hour.to_string(), now_minute.to_string(), now_iso8601], |row|{
        Ok(
            JoinedLesson{
                weekday   : row.get(0).ok(),
                teacher   : row.get(1).ok(),
                class     : None,
                classroom : row.get(2).ok(),
                subject   : row.get(3).ok(),
                lessonh   : JoinedHour{
                    lesson_hour: None,
                    start_hour   : row.get(4).ok(),
                    start_minute : row.get(5).ok(),
                    end_hour     : row.get(6).ok(),
                    end_minutes  : row.get(7).ok()
                },
                academic_year : None,
                semester      : None
            }
        )
    })?;
    let mut to_return = vec![];
    for joined_lesson in iter.flatten(){
        to_return.push(joined_lesson);
    }
    Ok(to_return)
}

pub fn get_duties_for_teacher(teacher_id: u16, db: &rusqlite::Connection) -> Result<Vec<JoinedDuty>, rusqlite::Error>{
    let now         = chrono::Local::now();
    let now_iso8601 = now.to_rfc3339();
    let weekd       = now.weekday() as u8 + 1;
    let now_hour    = now.hour  () & 0xFF;
    let now_minute  = now.minute() & 0xFF;
    let query = "
    SELECT 
    Duties.weekday, Corridors.corridor_name, Breaks.start_hour, Breaks.start_minutes, Breaks.end_hour, Breaks.end_minutes
    FROM Duties
    JOIN Teachers  ON Duties.teacher_id    = Teachers.teacher_id
    JOIN Breaks    ON Duties.break_num     = Breaks.break_num
    JOIN Corridors ON Duties.place_id      = Corridors.corridor 
    JOIN Years     ON Duties.academic_year = Years.academic_year
    JOIN Semesters ON Duties.semester      = Semesters.semester
    WHERE Duties.teacher_id  = ?1 AND Duties.weekday     = ?2 
    AND Breaks.start_hour    < ?3 AND Breaks.end_hour    > ?3 
    AND Breaks.start_minutes < ?4 AND Breaks.end_minutes > ?4
    AND Semesters.start_date < ?5 AND Semesters.end_date > ?5
    AND Years.start_date     < ?5 AND Years.end_date     > ?5
    ";
    let mut stmt = db.prepare(query)?;
    let iter = stmt.query_map([teacher_id.to_string(), weekd.to_string(), now_hour.to_string(), now_minute.to_string(), now_iso8601], |row| {
        Ok(
            JoinedDuty{
                weekday       : row.get(0).ok(),
                place         : row.get(1).ok(),
                teacher       : None,
                semester      : None,
                academic_year : None,
                break_num: JoinedHour{
                    lesson_hour  : None,
                    start_hour   : row.get(2).ok(),
                    start_minute : row.get(3).ok(),
                    end_hour     : row.get(4).ok(),
                    end_minutes  : row.get(5).ok()
                }
            }
        )
    })?;
    let mut to_return = vec![];
    for joined_duty in iter.flatten(){
        to_return.push(joined_duty);
    }
    Ok(to_return)
}

pub enum MainpulationType{
    Delete(Delete),
    Insert(POST),
    Get   (GET)
}
pub enum GET{
    Lesson    {class: u16, lesson_hour: u8, weekd: u8, semester: u8, academic_year: u8},
    Year      {year  : u8},
    Semester  {semester   : u8},
    Subject   {subject_id : u16},
    Class     {class_id   : u16},
    Classroom {classroom_id : u16},
    Teacher   {teacher_id : u16},
    Corridor  {corridor_id: u16},
    LessonHour{lesson_hour: u8},
    Break     {break_hour : u8},
    Duty      {weekd: u8, break_num: u8, teacher_id: u16, semester: u8, academic_year: u8}
}
pub enum Delete{
    Lesson     {class: u16, weekd: u8, lessonh: u16, semester: u8, academic_year: u8},
    Year       {academic_year: u8},
    Semester   {semester: u8},
    Subject    {subject: u16},
    Class      {class: u16},
    Classroom  {classroom: u16},
    Teacher    {teacher: u16},
    LessonHour {lessonh: u16},
    Corridor   {corridor: u16},
    Duty       {weekday: u8, break_num: u8, teacher_id: u16, semester: u8, academic_year: u8}
}

type Lesson = (u8, u16, u16, u16, u16, u16, u8, u8);

pub enum POST{
    Lesson      (Option<Lesson>),
    Class       (Option<(u16, String)>),
    Classroom   (Option<(u16, String)>),
    Teacher     (Option<(u16, String)>),
    Subject     (Option<(u16, String)>),
    LessonHours (Option<(u16, u8, u8, u8, u8)>),
    Semester    (Option<(u8, String, String, String)>),
    Year        (Option<(u8, String, String, String)>),
    Corridors   (Option<(u16, String)>),
    Break       (Option<(u8, u8, u8, u8, u8)>),
    Duty        (Option<(u8, u8, u16, u16, u8, u8)>)
}

pub fn manipulate_database(manipulation: MainpulationType, db: &rusqlite::Connection) -> Result<String, rusqlite::Error>{
    match manipulation{
        MainpulationType::Delete(delete) =>{
            match delete{
                Delete::Subject { subject } => {
                    db.execute("DELETE FROM Subjects WHERE subject_name = ?1", [subject])?;
                    Ok("msat/201-Deleted".to_string())
                }
                Delete::Year { academic_year } => {
                    db.execute("DELETE FROM Years WHERE academic_year = ?1", [academic_year])?;
                    Ok("msat/201-Deleted".to_string())
                }
                Delete::Class { class } => {
                    db.execute("DELETE FROM Classes WHERE class_id = ?1", [class])?;
                    Ok("msat/201-Deleted".to_string())
                }
                Delete::Lesson { class, weekd, lessonh, semester, academic_year } => {
                    db.execute("DELETE FROM Classes 
                        WHERE class_id  = ?1 AND weekday = ?2 
                        AND lesson_hour = ?3 AND semester = ?4
                        AND academic_year = ?5"
                        , [class, weekd.into(), lessonh, semester.into(), academic_year.into()])?;
                    Ok("msat/201-Deleted".to_string())
                }
                Delete::Teacher { teacher } => {
                    db.execute("DELETE From Teachers WHERE teacher_id = ?1", [teacher])?;
                    Ok("msat/201-Deleted".to_string())
                }
                Delete::Semester { semester } => {
                    db.execute("DELETE FROM Semesters WHERE semester = ?1", [semester])?;
                    Ok("msat/201-Deleted".to_string())
                }
                Delete::Corridor { corridor } => {
                    db.execute("DELETE FROM Corridors WHERE corridor = ?1", [corridor])?;
                    Ok("msat/201-Deleted".to_string())
                }
                Delete::Classroom { classroom } => {
                    db.execute("DELETE FROM Classrooms WHERE classroom_id = ?1", [classroom])?;
                    Ok("msat/201-Deleted".to_string())
                }
                Delete::LessonHour { lessonh } => {
                    db.execute("DELETE FROM LessonHours WHERE lesson_hour = ?1", [lessonh])?;
                    Ok("msat/201-Deleted".to_string())
                }
                Delete::Duty { weekday, break_num, teacher_id, semester, academic_year } => {
                    db.execute("DELETE FROM Duties 
                        WHERE weekday     = ?1 
                        AND break_num     = ?2 
                        AND teacher_id    = ?3 
                        AND semester      = ?4
                        AND academic_year = ?5", [weekday.into(), break_num.into(), teacher_id, semester.into(), academic_year.into()])?;
                    Ok("msat/201-Deleted".to_string())
                }
            }
        }
        MainpulationType::Insert(post) => {
            match post{
                POST::Year(Some((academic_year, year_name, start_date, end_date))) => {
                    db.execute("INSERT INTO Years (academic_year, year_name, start_date, end_date) 
                        VALUES (?1, ?2, ?3, ?4)
                        ON CONFLICT (academic_year) DO UPDATE SET 
                        year_name = excluded.year, start_date = excluded.start_date, end_date = excluded.end_date"
                        ,[academic_year.to_string(), year_name, start_date, end_date]
                    )?;
                }
                POST::Duty(Some((weekd, break_num, teacher_id, place_id, semester, academic_year))) => {
                    db.execute("INSERT INTO Duties (weekday, break_num, teacher_id, place_id, semester, academic_year)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                    ON CONFLICT (weekday, break_num, teacher_id, semester, academic_year) 
                    DO UPDATE SET 
                    place_id = excluded.place_id"
                        ,[weekd.into(), break_num.into(), teacher_id, place_id, semester.into(), academic_year.into()]
                    )?;
                }
                POST::Class(Some((class_id, class_name))) => {
                    db.execute("INSERT INTO Classes (class_id, class_name) 
                        VALUES (?1, ?2)
                        ON CONFLICT (class_id)
                        DO UPDATE SET class_name = excluded.class_name"
                        , [class_id.to_string(), class_name])?;
                }
                POST::Break(Some((break_num, start_hour, start_minute, end_hour, end_minute))) => {
                    db.execute("INSERT INTO Breaks (break_num, start_hour, start_minutes, end_hour, end_minutes) 
                        VALUES (?1, ?2, ?3, ?4)
                        ON CONFLICT (break_num)
                        DO UPDATE SET start_hour = excluded.start_hour, start_minutes = excluded.start_minutes,
                        end_hour = excluded.end_hour, end_minutes = excluded.end_minutes"
                        , [break_num, start_hour, start_minute, end_hour, end_minute])?;
                }
                POST::Lesson(Some((weekd, class_id, classroom_id, teacher_id, subject_id, lessonh, semester, academic_year))) => {
                    db.execute(
                    "INSERT INTO Lessons (weekday, class_id, classroom_id, teacher_id, subject_id, lesson_hour, semester, academic_year)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    ON CONFLICT (weekday, class_id, lesson_hour, semester, academic_year)
                    DO UPDATE SET 
                    classroom_id = excluded.classroom_id, 
                    teacher_id = excluded.teacher_id, 
                    subject_id = excluded.subject_id
                    ", 
                    [weekd.into(), class_id, classroom_id, teacher_id, subject_id, lessonh, semester.into(), academic_year.into()])?;
                }
                POST::Teacher(Some((teacher_id, teacher_name))) => {
                    db.execute("INSERT INTO Teachers (teacher_id, teacher_name) 
                        VALUES (?1, ?2)
                        ON CONFLICT (teacher_id)
                        DO UPDATE SET teacher_name = excluded.teacher_name"
                        , [teacher_id.to_string(), teacher_name])?;
                }
                POST::Subject(Some((subject_id, subject_name))) => {
                    db.execute("INSERT INTO Subjects (subject_id, subject_name) 
                        VALUES (?1, ?2)
                        ON CONFLICT (subject_id)
                        DO UPDATE SET subject_name = excluded.subject_name"
                        , [subject_id.to_string(), subject_name])?;
                }
                POST::Semester(Some((semester, semester_name, start_date, end_date))) => {
                    db.execute("INSERT INTO Semesters (semester, semester_name, start_date, end_date)
                    VALUES (?1, ?2, ?3, ?4)
                    ON CONFLICT (semester)
                    DO UPDATE SET 
                    semester_name = excluded.semester_name, 
                    start_date = excluded.start_date, 
                    end_date = excluded.end_date"
                        , [semester.to_string(), semester_name, start_date, end_date])?;
                }
                POST::Classroom(Some((classroom_id, classroom_name))) => {
                    db.execute("INSERT INTO Classrooms (classroom_id, classroom_name) 
                        VALUES (?1, ?2)
                        ON CONFLICT (classroom_id)
                        DO UPDATE SET classroom_name = excluded.classroom_name"
                        , [classroom_id.to_string(), classroom_name])?;
                }
                POST::Corridors(Some((corridor_id, corridor_name))) => {
                    db.execute("INSERT INTO Corridors (corridor, corridor_name) 
                        VALUES (?1, ?2)
                        ON CONFLICT (corridor)
                        DO UPDATE SET corridor_name = excluded.corridor_name"
                        , [corridor_id.to_string(), corridor_name])?;
                }
                POST::LessonHours(Some((lesson_num, start_hour, start_minute, end_hour, end_minute))) => {
                    db.execute("INSERT INTO LessonHours (lesson_hour, start_hour, start_minutes, end_hour, end_minutes)
                    VALUES (?1, ?2, ?3, ?4, ?5)
                    ON CONFLICT (lesson_hour)
                    DO UPDATE SET 
                    start_hour    = excluded.start_hour,
                    end_hour      = excluded.end_hour,
                    start_minutes = excluded.start_minutes,
                    end_minutes   = excluded.end_minutes",
                    [lesson_num, start_hour.into(), start_minute.into(), end_hour.into(), end_minute.into()])?;
                }
                _ => {
                    return Ok("msat/500-Internal-Server-Error&error=error+occured+while+inserting+values".to_string());
                }
            }
            Ok("msat/201-Created".to_string())
        }
        MainpulationType::Get(get) => {
            match get{
                GET::Teacher { teacher_id } => {
                    let mut stmt = db.prepare("SELECT teacher_name FROM Teachers WHERE teacher_id = ?1")?;
                    let string = stmt.query_row([teacher_id], |row| {
                        Ok(
                            row.get::<usize, String>(0).unwrap_or_default()
                        )
                    })?;
                    Ok(format!("msat/200-OK&teacher_name={}", string.to_single('_')))
                }
                GET::Class { class_id } => {
                    let mut stmt = db.prepare("SELECT class_name FROM Classes WHERE class_id = ?1")?;
                    let name = stmt.query_row([class_id], |row|{
                        Ok(
                            row.get::<usize, String>(0).unwrap_or_default()
                        )
                    })?;
                    Ok(format!("msat/200-OK&class_name={}",name))
                }
                GET::Classroom { classroom_id } => {
                    let mut stmt = db.prepare("SELECT classroom_name FROM Classrooms WHERE classroom_id = ?1")?;
                    let name = stmt.query_row([classroom_id], |row|{
                        Ok(
                            row.get::<usize, String>(0).unwrap_or_default()
                        )
                    })?;
                    Ok(format!("msat/200-OK&classroom_name={}",name))
                }
                GET::Subject { subject_id } => {
                    let mut stmt = db.prepare("SELECT subject_name FROM Subjects WHERE subject_id = ?1")?;
                    let name = stmt.query_row([subject_id], |row|{
                        Ok(
                            row.get::<usize, String>(0).unwrap_or_default()
                        )
                    })?;
                    Ok(format!("msat/200-OK&subject_name={}",name))
                }
                GET::Corridor { corridor_id } => {
                    let mut stmt = db.prepare("SELECT corridor_name FROM Corridors WHERE corridor = ?1")?;
                    let name = stmt.query_row([corridor_id], |row|{
                        Ok(
                            row.get::<usize, String>(0).unwrap_or_default()
                        )
                    })?;
                    Ok(format!("msat/200-OK&corridor_name={}",name))
                }
                GET::Year { year } => {
                    let mut stmt = db.prepare("SELECT year_name, start_date, end_date FROM Years WHERE academic_year = ?1")?;
                    let (year_name, start_date, end_date) : (String, String, String) = stmt.query_row([year], |row| {
                        Ok((
                            row.get(0).unwrap_or_default(),
                            row.get(1).unwrap_or_default(),
                            row.get(2).unwrap_or_default()
                        ))
                    })?;
                    Ok(format!("msat/200-OK&year_name={}&start_date={}&end_date={}", 
                            year_name.to_single('_'), start_date.to_single('_'), end_date.to_single('_')))
                }
                GET::Lesson { class, lesson_hour, weekd, semester, academic_year } => {
                    let mut stmt = db.prepare("SELECT classroom_id, teacher_id, subject_id FROM Lessons 
                    WHERE class_id    = ?1 
                    AND weekday       = ?2
                    AND semester      = ?3 
                    AND academic_year = ?4 
                    AND lesson_hour   = ?5")?;
                    let (classroom, teacher, subject) : (String, String, String) = 
                    stmt.query_row([class, lesson_hour.into(), weekd.into(), semester.into(), academic_year.into()], |row| {
                        Ok((
                                row.get(0).unwrap_or_default(),
                                row.get(1).unwrap_or_default(),
                                row.get(2).unwrap_or_default()
                        ))
                    })?;
                    Ok(format!("msat/200-OK&classroom_name={}&teacher_name={}&subject_name={}", classroom, teacher, subject))
                }
                GET::Semester { semester } => {
                    let mut stmt = db.prepare("SELECT semester_name, start_date, end_date 
                        FROM Semesters 
                        WHERE semester = ?1")?;
                    let (semester_name, start_date, end_date) : (String, String, String) =
                        stmt.query_row([semester], |row| {
                            Ok((
                                    row.get(0).unwrap_or_default(),
                                    row.get(1).unwrap_or_default(),
                                    row.get(2).unwrap_or_default()
                            ))
                        })?;
                    Ok(format!("msat/200-OK&semester_name={}&start_date={}&end_date={}", 
                        semester_name.to_single('_'), start_date.to_single('_'), end_date.to_single('_')))
                }
                GET::LessonHour { lesson_hour } => {
                    let mut stmt = db.prepare("SELECT start_hour, start_minutes, end_hour, end_minutes 
                        FROM LessonHours
                        WHERE lesson_hour = ?1")?;
                    let (start_hour, start_minutes, end_hour, end_minutes) : (u8, u8, u8, u8) 
                    = stmt.query_row([lesson_hour], |row| {
                        Ok((
                                row.get(0).unwrap_or_default(),
                                row.get(1).unwrap_or_default(),
                                row.get(2).unwrap_or_default(),
                                row.get(3).unwrap_or_default()
                        ))
                    })?;
                    Ok(
                        format!("msat/200-OK&start_time={:02}:{:02}&end_time={:02}:{:02}", 
                        start_hour, start_minutes, end_hour, end_minutes))
                }
                GET::Break { break_hour } => {
                    let mut stmt = db.prepare("SELECT start_hour, start_minutes, end_hour, end_minutes 
                        FROM Breaks
                        WHERE break_num = ?1")?;
                    let (start_hour, start_minutes, end_hour, end_minutes) : (u8, u8, u8, u8) 
                    = stmt.query_row([break_hour], |row| {
                        Ok((
                                row.get(0).unwrap_or_default(),
                                row.get(1).unwrap_or_default(),
                                row.get(2).unwrap_or_default(),
                                row.get(3).unwrap_or_default()
                        ))
                    })?;
                    Ok(
                        format!("msat/200-OK&start_time={:02}:{:02}&end_time={:02}:{:02}", 
                        start_hour, start_minutes, end_hour, end_minutes))
                }
                GET::Duty { weekd, break_num, teacher_id, semester, academic_year } => {
                    let mut stmt = db.prepare(
                        "SELECT Corridors.place_id 
                        FROM Duties 
                        JOIN Corridors ON Duties.place_id = Corridors.corridor
                        WHERE weekday = ?1
                        AND break_num = ?2 
                        AND teacher_id = ?3 
                        AND semester = ?4
                        AND academic_year = ?5"
                    )?;
                    let result = stmt.query_row([weekd.into(), break_num.into(), teacher_id, semester.into(), academic_year.into()], |row| {
                        Ok(
                            row.get::<usize, String>(0).unwrap_or_default()
                        )
                    })?;
                    Ok(format!("msat/200-OK&duty_place={}", result.to_single('_')))
                }
            }
        }
    }
}
