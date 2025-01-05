use chrono::Datelike;
///=================================
///          database.rs
/// responsible for initializing 
/// database (SQLite) and manipulating 
/// it.
///=================================

// Global Imports
use rusqlite::Connection as SQLite;
use rusqlite::OpenFlags;
use chrono::Local;
use tokio::sync::MutexGuard;
use std::collections::HashMap;
// Local Imports
use crate::cli;
use crate::types::*;
use crate::utils::*;

pub async fn init(flags: OpenFlags) -> Result<SQLite, ()>{
    if let Err(_) = std::fs::read_dir("data"){
        if let Err(e) = std::fs::create_dir("data"){
            cli::print_error("Error occured while creating directory 'data'", e);
        };
    };

    let database: SQLite = match SQLite::open_with_flags("data/database.db", flags){
        Ok(v) => v,
        Err(e) => {
            cli::print_error("Error connecting to database", e);
            return Err(());
        }
    };
    let queries = 
    [
    "CREATE TABLE IF NOT EXISTS LessonHours(
	lesson_num INTEGER PRIMARY KEY,
	start_time INTEGER NOT NULL,
	end_time   INTEGER NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS Classes(
        class_id   INTEGER PRIMARY KEY,
        class_name TEXT NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS Lessons(
        week_day     INTEGER NOT NULL,
	class_id     INTEGER NOT NULL,
	lesson_hour  INTEGER NOT NULL,
	teacher_id   INTEGER NOT NULL,
	subject_id   INTEGER NOT NULL,
	classroom_id INTEGER NOT NULL,
	PRIMARY KEY  (class_id, lesson_hour, week_day)
    );",
    "CREATE TABLE IF NOT EXISTS Teachers(
	teacher_id INTEGER PRIMARY KEY,
	first_name TEXT NOT NULL,
        last_name  TEXT NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS Classrooms(
	classroom_id   INTEGER PRIMARY KEY,
	classroom_name TEXT NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS Subjects(
	subject_id   INTEGER PRIMARY KEY,
	subject_name TEXT NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS BreakHours(
        break_num  INTEGER PRIMARY KEY,
        start_time INTEGER NOT NULL,
        end_time   INTEGER NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS DutyPlaces(
        dutyplace_id    INTEGER PRIMARY KEY,
        duty_place_name TEXT NOT NULL
    );",
    "CREATE TABLE IF NOT EXISTS Duties(
	break_number INTEGER NOT NULL,
	teacher_id   INTEGER NOT NULL,
        week_day     INTEGER NOT NULL,
        duty_place   INTEGER NOT NULL,
	PRIMARY KEY  (break_number, teacher_id, week_day),
	FOREIGN KEY  (teacher_id)    REFERENCES Teachers   (teacher_id),
	FOREIGN KEY  (break_number)  REFERENCES BreakHours (break_num)
    );"
    ];
    for query in queries{
        match database.execute(&query, []){
            Ok(_)  => cli::print_success("Succesfully executed SQL command"),
            Err(e) => cli::print_error  ("Error occured while creating database", e)
        }
    }
    cli::print_success("Succesfully Initialized Database");
    return Ok(database);
}

pub fn get_lesson_hour(db: &MutexGuard<'_, SQLite>) -> Result<u8, ()>{
    let (month, day) : (u8, u8) = 
    (Local::now().month().try_into().unwrap_or_default(), Local::now().month().try_into().unwrap_or_default());
    let formatted = format_two_digit_time(month, day);
    let query = "SELECT * FROM LessonHours 
        WHERE start_time < CAST(?1 AS INTEGER) AND end_time > CAST(?2 AS INTEGER);";
    let mut stmt = match db.prepare(&query){
        Ok(v) => v,
        Err(_) => {
            return Err(());
        }
    };
    let result_iter = stmt.query_map([&formatted, &formatted],|row|{
        Ok(quick_match(row.get::<usize, u8>(0)))
    });
    match result_iter{
        Ok(iter) => {
            for element in iter{
                match element{
                    Ok(value) => {
                        match value{
                            Some(v) => return Ok(v),
                            None => return Ok(0)
                        }
                    }
                    Err(_) => return Err(())
                }
            }
        }
        Err(_) => {return Err(())}
    }

    return Ok(0);
}

pub fn get_break_num(db: &MutexGuard<'_, SQLite>) -> Result<u8, ()>{
    let (month, day) : (u8, u8) = 
    (Local::now().month().try_into().unwrap_or_default(), Local::now().day().try_into().unwrap_or_default());
    let formatted = format_two_digit_time(month, day);
    let query = "SELECT * FROM BreakHours 
        WHERE start_time < CAST(?1 AS INTEGER) AND end_time > CAST(?2 AS INTEGER);";
    let mut stmt = match db.prepare(&query){
        Ok(v) => v,
        Err(_) => {
            return Err(());
        }
    };
    let result_iter = stmt.query_map([&formatted, &formatted],|row|{
        Ok(quick_match(row.get::<usize, u8>(0)))
    });
    match result_iter{
        Ok(iter) => {
            for element in iter{
                match element{
                    Ok(value) => {
                        match value{
                            Some(v) => return Ok(v),
                            None => return Ok(0)
                        }
                    }
                    Err(_) => return Err(())
                }
            }
        }
        Err(_) => {return Err(())}
    }

    return Ok(0);
}

pub fn get_teacher_duty_bool(weekd: u8, teacher_id: u16, db: &MutexGuard<'_,SQLite>) -> Result<bool, ()>{
    let lesson_hour = match get_lesson_hour(&db){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    let mut stmt = match db.prepare("SELECT * FROM Duties 
        WHERE teacher_id = ?1 AND week_day = ?2 AND lesson_hour = ?3;"){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    let item = match stmt.query_row([teacher_id,(weekd).into(),lesson_hour.into()], |row|{
    Ok(quick_match(row.get::<usize, u16>(1)))}){
        Ok(v) => v,
        Err(e) => {
            match e{
                rusqlite::Error::QueryReturnedNoRows => {
                    return Ok(false);
                }
                _ => {
                    return Err(());
                }
            }
        }
    };
    match item{
        Some(v) => return Ok(v == teacher_id),
        None => return Err(())
    }
}
pub fn get_classroom(id: u16,db: &MutexGuard<'_, SQLite>) -> Result<String, ()>{
    let query = "SELECT * FROM Classrooms WHERE classroom_id = ?1;";
    let mut stmt = match db.prepare(&query){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    let element = match stmt.query_row([id], |row|{
        Ok(
            quick_match(row.get::<usize, String>(1))
        )
    }){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    match element{
        Some(v1) => {
            return Ok(v1);
        }
        None => return Err(())
    }
}
pub fn get_class(id: u16, db: &MutexGuard<'_, SQLite>) -> Result<String, ()>{
    let query = "SELECT * FROM Classes WHERE class_id = ?1;";
    let mut stmt = match db.prepare(&query){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    let element = match stmt.query_row([id], |row|{
        Ok(quick_match(row.get::<usize, String>(1)))
    }){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    match element{
        Some(v) => return Ok(v),
        None => return Err(())
    }
}
pub fn get_teacher(id: u16, db: &MutexGuard<'_, SQLite>) -> Result<String, ()>{
    let query = "SELECT * FROM Teachers WHERE teacher_id = ?1;";
    let mut stmt = match db.prepare(&query){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    let element = match stmt.query_row([id], |row|{
        Ok(
            quick_match(row.get::<usize, String>(1))
        )
    }){
        Ok(v) => v,
        Err(_) => return Err(())
    };
    match element{
        Some(v) => return Ok(v),
        None => return Err(())
    }
}

pub fn get_classes(db: &MutexGuard<'_, SQLite>) -> HashMap<u16, String>{
    match db.prepare("SELECT * FROM Classes"){
                                Ok(mut stmt) => {
                                    let iter = stmt.query_map([], |row| {
                                        Ok(
                                            Class{
                                                class_id: row.get(0).unwrap_or(0),
                                                class_name: row.get(1).unwrap_or("".to_string())
                                            }
                                        )
                                    });

                                    if let Ok(ok_iter) = iter{
                                        let filtered_iter = ok_iter.filter(|s| s.is_ok()&&s.is_ok())
                                            .map(|s| s.unwrap())
                                            .filter(|s| s.class_id!=0&&s.class_name.as_str()!="")
                                            .collect::<Vec<Class>>();
                                        let mut finhashmap : HashMap<u16, String> = HashMap::new();
                                        for class in filtered_iter{
                                            finhashmap.insert(class.class_id, class.class_name);
                                        }
                                        finhashmap
                                    }
                                    else{
                                        HashMap::new()
                                    }
                                }
                                Err(_) => HashMap::new()
                            }
}

pub fn get_teachers(db: &MutexGuard<'_, SQLite>) -> HashMap<u16, (String, String)>{
                                    match db.prepare("SELECT * FROM Teachers"){
                                Ok(mut stmt) => {
                                    let iter = stmt.query_map([], |row| {
                                        Ok(
                                            Teacher{
                                                teacher_id: row.get(0).unwrap_or(0),
                                                first_name: row.get(1).unwrap_or("".to_string()),
                                                last_name: row.get(2).unwrap_or("".to_string())
                                            }
                                        )
                                    });

                                    if let Ok(ok_iter) = iter{
                                        let filtered_iter = ok_iter.filter(|s| s.is_ok()&&s.is_ok())
                                            .map(|s| s.unwrap())
                                            .filter(|s| s.teacher_id!=0&&s.first_name.as_str()!=""&&s.last_name.as_str()!="")
                                            .collect::<Vec<Teacher>>();
                                        let mut finhashmap : HashMap<u16, (String, String)> = HashMap::new();
                                        for teacher in filtered_iter{
                                            finhashmap.insert(teacher.teacher_id, (teacher.first_name, teacher.last_name));
                                        }
                                        finhashmap
                                    }
                                    else{
                                        HashMap::new()
                                    }
                                }
                                Err(_) => HashMap::new()
                            }

}
pub fn get_subjects(db: &MutexGuard<'_, SQLite>) -> HashMap<u16, String>{
    match db.prepare("SELECT * FROM Subjects"){
                                Ok(mut stmt) => {
                                    let iter = stmt.query_map([], |row| {
                                        Ok(
                                            Subject{
                                                subject_id: row.get(0).unwrap_or(0),
                                                subject_name: row.get(1).unwrap_or("".to_string())
                                            }
                                        )
                                    });

                                    if let Ok(ok_iter) = iter{
                                        let filtered_iter = ok_iter.filter(|s| s.is_ok()&&s.is_ok())
                                            .map(|s| s.unwrap())
                                            .filter(|s| s.subject_id!=0&&s.subject_name.as_str()!="")
                                            .collect::<Vec<Subject>>();
                                        let mut finhashmap : HashMap<u16, String> = HashMap::new();
                                        for s in filtered_iter{
                                            finhashmap.insert(s.subject_id, s.subject_name);
                                        }
                                        finhashmap
                                    }
                                    else{
                                        HashMap::new()
                                    }
                                }
                                Err(_) => HashMap::new()
                            }
}

pub fn get_classrooms(db: &MutexGuard<'_,SQLite>) -> HashMap<u16, String>{
    match db.prepare("SELECT * FROM Classrooms"){
                                Ok (mut stmt) => {
                                    let iter = stmt.query_map([], |row| {
                                        Ok(
                                            Classroom{
                                                classroom_id: row.get(0).unwrap_or(0),
                                                classroom_name: row.get(1).unwrap_or("".to_string())
                                            }
                                        )
                                    });
                                    if let Ok(ok_iter) = iter{
                                        let filtered_iter = ok_iter.filter(|s| s.is_ok()&&s.is_ok())
                                            .map(|s| s.unwrap())
                                            .filter(|s| s.classroom_id!=0&&s.classroom_name.as_str()!="")
                                            .collect::<Vec<Classroom>>();
                                        let mut finhashmap : HashMap<u16, String> = HashMap::new();
                                        for classroom in filtered_iter{
                                            finhashmap.insert(classroom.classroom_id, classroom.classroom_name);
                                        }
                                        finhashmap
                                    }
                                    else{
                                        HashMap::new()
                                    }
                                }
                                Err(_) => HashMap::new()
                            }

}
