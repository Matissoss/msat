///=================================
///          database.rs
/// responsible for initializing 
/// database (SQLite)
///=================================

// Global Imports
use rusqlite::Connection as SQLite;
use rusqlite::OpenFlags;
// Local Imports
use crate::cli;

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
        break_num INTEGER PRIMARY KEY,
        start_time INTEGER PRIMARY KEY,
        end_time INTEGER PRIMARY KEY
    );",
    "CREATE TABLE IF NOT EXISTS Duties(
	break_number INTEGER NOT NULL,
	teacher_id   INTEGER NOT NULL,
        week_day     INTEGER NOT NULL,
        duty_place   TEXT NOT NULL,
	PRIMARY KEY  (break_number, teacher_id, week_day),
	FOREIGN KEY  (teacher_id)    REFERENCES Teachers   (teacher_id),
	FOREIGN KEY  (break_number)  REFERENCES BreakHours (break_num),
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
