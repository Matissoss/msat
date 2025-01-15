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
        Semaphore
    }
};
use toml::from_str;
use std::{
    collections::HashMap, fs::File, sync::{
        Arc, LazyLock
    }
};
// Local Imports 
use crate::{consts::VERSION, visual};
use crate::types::*;
// static/const declaration
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Timestamp{
    hour: u8,
    minute: u8,
    secs: u8
}
impl Timestamp{
    fn can_proceed(&self, timeout_secs: u32) -> bool{
        let now : u32 = (self.hour as u32 * 60 * 60) + (self.minute as u32 * 60) + self.secs as u32;
        return Timestamp::from(now + timeout_secs) < Timestamp::now()
    }
    fn now() -> Timestamp{
        let now = &chrono::Local::now();
        return Timestamp{
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
        return Self{
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
    pub fn from_str(request: &str) -> Request{
        return Request{request: request.to_string()}
    }
    pub fn parse(&self) -> Result<ParsedRequest, ()>{
        if self.request.starts_with(&format!("/?msat/{}", VERSION)) == false || 
           self.request.starts_with(&format!("msat/{}" , VERSION)) == false
        {
            return Err(());
        }
        let vector = self.request.split('&').collect::<Vec<&str>>();
        if vector.len() == 1{
            return Err(());
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

        return Err(());
    }
}

// Functions

pub async fn get_config() -> Option<Configuration>{
    return match fs::read_to_string("data/config.toml").await{
        Ok(v) => {
            match toml::from_str::<Configuration>(&v){
                Ok(conf) => Some(conf),
                Err(_)   => None
            }
        }
        Err(_) => {
            if let Ok(b) = fs::try_exists("data/config.toml").await{
                if b == false{
                    let _ = fs::write("data/config.toml", "").await;
                }
            }
            None
        }
    }
}

pub async fn get_password() -> Option<String>{
    return match fs::read_to_string("data/config.toml").await{
        Ok(v) => {
            match from_str::<Configuration>(&v){
                Ok(conf) => {
                    if conf.password == ""{
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

    return Ok(db);
}


/// STATIC
pub async fn static_lesson_table(db: &MutexGuard<'_, rusqlite::Connection>) -> Result<Vec<JoinedLessonRaw>, ()>{
    if Timestamp::now().can_proceed(TIMEOUT){
        if let Ok(permit) = tokio::time::timeout
        (std::time::Duration::from_secs(5), Arc::clone(&GLOBAL_STATIC_SEMAPHORE).acquire_owned()).await
        {
            if let Ok(perm) = permit{
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
                                    weekday       : row.get(0).conv(),
                                    class         : row.get(1).conv(),
                                    classroom     : row.get(2).conv(),
                                    teacher       : row.get(3).conv(),
                                    subject       : row.get(4).conv(),
                                    lessonh       : row.get(5).conv(),
                                    semester      : row.get(6).conv(),
                                    academic_year : row.get(7).conv()
                                }
                            )
                        });
                        if let Ok(ok_iter) = iter{
                            let mut to_return = vec![];
                            for raw_lesson in ok_iter{
                                if let Ok(ok_raw_lesson) = raw_lesson{
                                    to_return.push(ok_raw_lesson);
                                }
                            }
                            if let Ok(output_file) = File::open("static_data/LESSON_TABLE.raw.static.cbor"){
                                match serde_cbor::to_writer(output_file, &to_return){
                                    Ok(_) => {
                                        visual::info("Saved STATIC Lesson Table");
                                    }
                                    Err(_) => {
                                        visual::info("Couldn't save STATIC Lesson Table to static_data/LESSON_TABLE.raw.static.cbor");
                                    }
                                }
                            }
                            drop(perm);
                            return Ok(to_return);
                        }
                        else{
                            return static_old_table();
                        }
                    }
                    Err(_) => {
                        return static_old_table();
                    }
                }
            }
        }
    }
    return static_old_table();
}
/// STATIC
pub fn static_old_table() -> Result<Vec<JoinedLessonRaw>, ()>{
    if let Ok(file_content) = std::fs::File::open("static_data/LESSON_TABLE.raw.static.cbor"){
        match serde_cbor::from_reader::<Vec<JoinedLessonRaw>, File>(file_content)
        {
            Ok(v) => return Ok(v),
            Err(_) => return Err(())
        }
    };
    return Err(());
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
                        row.get::<usize, u16>(0).conv(), 
                        row.get::<usize, String>(1).conv()
                    ))
                })?;
                already_visited_class.insert(id.clone().unwrap_or_default(), name.clone().unwrap_or_default());
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
                        row.get::<usize, u16>(0).conv(), 
                        row.get::<usize, String>(1).conv()
                    ))
                })?;
                already_visited_classroom.insert(id.clone().unwrap_or_default(), name.clone().unwrap_or_default());
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
                        row.get::<usize, u16>(0).conv(), 
                        row.get::<usize, String>(1).conv()
                    ))
                })?;
                already_visited_teacher.insert(id.clone().unwrap_or_default(), name.clone().unwrap_or_default());
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
                        row.get::<usize, u16>(0).conv(), 
                        row.get::<usize, String>(1).conv()
                    ))
                })?;
                already_visited_subject.insert(id.clone().unwrap_or_default(), name.clone().unwrap_or_default());
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
                        row.get::<usize, u16>(0).conv(), 
                        row.get::<usize, u8>(1).conv(),
                        row.get::<usize, u8>(1).conv(),
                        row.get::<usize, u8>(1).conv(),
                        row.get::<usize, u8>(1).conv()
                    ))
                })?;
                let current = JoinedHour{
                    lesson_hour: id,
                    start_hour,
                    start_minute,
                    end_hour,
                    end_minutes: end_minute
                };
                already_visited_lessonh.insert(id.clone().unwrap_or_default(), current);
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
                        row.get::<usize, u8>(0).conv(), 
                        row.get::<usize, String>(1).conv()
                    ))
                })?;
                already_visited_semester.insert(id.clone().unwrap_or_default(), name.clone().unwrap_or_default());
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
                        row.get::<usize, u8>(0).conv(), 
                        row.get::<usize, String>(1).conv()
                    ))
                })?;
                already_visited_year.insert(id.clone().unwrap_or_default(), name.clone().unwrap_or_default());
                current_to_lesson.academic_year = name;
            }
        }
        to_return.push(current_to_lesson);
    }
    return Ok(to_return);
}

/// DYNAMIC
pub fn get_lessons_by_teacher_id(teacher_id: u16, db: &rusqlite::Connection) -> Result<Vec<JoinedLesson>, rusqlite::Error> {
    let now         = chrono::Local::now();
    let now_iso8601 = now.to_rfc3339();
    let weekd       = now.weekday() as u8 + 1;
    let now_hour    = now.hour  () & 0xFF;
    let now_minute  = now.minute() & 0xFF;
    let query = 
    "SELECT 
    Lessons.weekday, Classes.class_name, Classrooms.classroom_name, Subjects.subject_name,
    LessonHours.start_hour, LessonHours.start_minutes, LessonHours.end_hour, LessonHours.end_minutes,
    Years.start_date, Years.end_date, 
    FROM Lessons 
    JOIN Classes     ON Lessons.class_id      = Classes.class_id
    JOIN Classrooms  ON Lessons.classroom_id  = Classrooms.classroom_id 
    JOIN Subjects    ON Lessons.subject_id    = Subjects.subject_id
    JOIN LessonHours ON Lessons.lesson_hour   = LessonHours.lesson_hour
    JOIN Years       ON Lessons.academic_year = Years.academic_year
    JOIN Semesters   ON Lessons.semester      = Semesters.semester
    WHERE Lessons.teacher_id        < ?1 AND Lessons.weekday         = ?2
    AND   LessonHours.start_hour    < ?3 AND LessonHours.end_hour    > ?3
    AND   LessonHours.start_minutes < ?4 AND LessonHours.end_minutes > ?4
    AND   Semesters.start_date      < ?5 AND Semesters.end_date      > ?5
    ";
    let mut stmt = db.prepare(query)?;

    let iter = stmt.query_map([teacher_id.to_string(), weekd.to_string(), now_hour.to_string(), now_minute.to_string(), now_iso8601], |row|{
        Ok(
            JoinedLesson{
                weekday   : row.get(0).conv(),
                teacher   : None,
                class     : row.get(1).conv(),
                classroom : row.get(2).conv(),
                subject   : row.get(3).conv(),
                lessonh   : JoinedHour{
                    lesson_hour: None,
                    start_hour   : row.get(4).conv(),
                    start_minute : row.get(5).conv(),
                    end_hour     : row.get(6).conv(),
                    end_minutes  : row.get(7).conv()
                },
                academic_year : None,
                semester      : None
            }
        )
    })?;
    let mut to_return = vec![];
    for joined_lesson in iter{
        if let Ok(v) = joined_lesson{
            to_return.push(v);
        }
    }
    return Ok(to_return);
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
                weekday   : row.get(0).conv(),
                teacher   : row.get(1).conv(),
                class     : None,
                classroom : row.get(2).conv(),
                subject   : row.get(3).conv(),
                lessonh   : JoinedHour{
                    lesson_hour: None,
                    start_hour   : row.get(4).conv(),
                    start_minute : row.get(5).conv(),
                    end_hour     : row.get(6).conv(),
                    end_minutes  : row.get(7).conv()
                },
                academic_year : None,
                semester      : None
            }
        )
    })?;
    let mut to_return = vec![];
    for joined_lesson in iter{
        if let Ok(v) = joined_lesson{
            to_return.push(v);
        }
    }
    return Ok(to_return);
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
    let mut stmt = db.prepare(&query)?;
    let iter = stmt.query_map([teacher_id.to_string(), weekd.to_string(), now_hour.to_string(), now_minute.to_string(), now_iso8601], |row| {
        Ok(
            JoinedDuty{
                weekday       : row.get(0).conv(),
                place         : row.get(1).conv(),
                teacher       : None,
                semester      : None,
                academic_year : None,
                break_num: JoinedHour{
                    lesson_hour  : None,
                    start_hour   : row.get(2).conv(),
                    start_minute : row.get(3).conv(),
                    end_hour     : row.get(4).conv(),
                    end_minutes  : row.get(5).conv()
                }
            }
        )
    })?;
    let mut to_return = vec![];
    for joined_duty in iter{
        if let Ok(duty) = joined_duty{
            to_return.push(duty);
        }
    }
    Ok(to_return)
}

pub enum MainpulationType{
    Delete(Delete),
    Insert(POST),
    Update(POST),
    Create(POST)
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
    Corridor   {corridor: u16}
}

pub enum POST{
    Lesson      (Option<(u8, u16, u16, u16, u16, u16, u8, u8)>),
    Class       (Option<(u16, String)>),
    Classroom   (Option<(u16, String)>),
    Teacher     (Option<(u16, String)>),
    Subject     (Option<(u16, String)>),
    LessonHours (Option<(u16, u8, u8, u8, u8)>),
    Semester    (Option<(u8, String, String, String)>),
    Year        (Option<(u8, String, String, String)>),
    Corridors   (Option<(u16, String)>),
    Break       (Option<(u8, u8, u8, u8)>),
    Duty        (Option<(u8, u8, u16, u16, u8, u8)>)
}

pub async fn manipulate(manipulation: MainpulationType, db: &rusqlite::Connection) -> Result<String, rusqlite::Error>{
    match manipulation{
        MainpulationType::Delete(delete) =>{
            match delete{
                Delete::Subject { subject } => {
                    db.execute("DELETE FROM Subjects WHERE subject_name = ?1", [subject])?;
                    return Ok("msat/201-Deleted".to_string());
                }
                Delete::Year { academic_year } => {
                    db.execute("DELETE FROM Years WHERE academic_year = ?1", [academic_year])?;
                    return Ok("msat/201-Deleted".to_string());
                }
                Delete::Class { class } => {
                    db.execute("DELETE FROM Classes WHERE class_id = ?1", [class])?;
                    return Ok("msat/201-Deleted".to_string());
                }
                Delete::Lesson { class, weekd, lessonh, semester, academic_year } => {
                    db.execute("DELETE FROM Classes 
                        WHERE class_id  = ?1 AND weekday = ?2 
                        AND lesson_hour = ?3 AND semester = ?4
                        AND academic_year = ?5"
                        , [class, weekd.into(), lessonh, semester.into(), academic_year.into()])?;
                    return Ok("msat/201-Deleted".to_string());
                }
                Delete::Teacher { teacher } => {
                    db.execute("DELETE From Teachers WHERE teacher_id = ?1", [teacher])?;
                    return Ok("msat/201-Deleted".to_string());
                }
                Delete::Semester { semester } => {
                    db.execute("DELETE FROM Semesters WHERE semester = ?1", [semester])?;
                    return Ok("msat/201-Deleted".to_string());
                }
                Delete::Corridor { corridor } => {
                    db.execute("DELETE FROM Corridors WHERE corridor = ?1", [corridor])?;
                    return Ok("msat/201-Deleted".to_string());
                }
                Delete::Classroom { classroom } => {
                    db.execute("DELETE FROM Classrooms WHERE classroom_id = ?1", [classroom])?;
                    return Ok("msat/201-Deleted".to_string());
                }
                Delete::LessonHour { lessonh } => {
                    db.execute("DELETE FROM LessonHours WHERE lesson_hour = ?1", [lessonh])?;
                    return Ok("msat/201-Deleted".to_string());
                }
            }
        }
        MainpulationType::Insert(post) => {
            todo!();
        }
        MainpulationType::Update(post) => {
            todo!();
        }
        MainpulationType::Create(post) => {
            todo!();
        }
    }
}

// Tests 

#[cfg(test)]
pub mod tests{
    use super::*;
    #[test]
    fn timestamp(){
        let timeout = 30;
        let time = Timestamp{
            hour  : 18,
            minute: 30,
            secs  : 10
        };
        println!("RES: {}", Timestamp::can_proceed(&time, timeout));
        //assert_eq!(true, Timestamp::can_proceed(&time, timeout));
    }
}
