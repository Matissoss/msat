use rusqlite::Connection;
use crate::{
    SUCCESS,
    ERROR,
    VERSION
};

pub async fn init() -> Result<(), ()>{
    match std::fs::read_dir("data"){
        Ok(_) => {},
        Err(_) => {
            match std::fs::create_dir("data"){
                Ok(_) => {}
                Err(e)=>{
                    eprintln!("{} Error creating directory \"data\": {}", ERROR, e);
                    return Err(());
                }
            }
        }
    }
    let database: Connection = match Connection::open("data/database.db"){
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} Error connecting to database: {}", ERROR, e);
            return Err(());
        }
    };
    let query = "CREATE TABLE IF NOT EXISTS LessonHours(
	lesson_num INTEGER PRIMARY KEY,
	start_time INTEGER NOT NULL,
	end_time INTEGER NOT NULL
    );
    CREATE TABLE IF NOT EXISTS Classes(
        class_id INTEGER PRIMARY KEY,
        class_name TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS Lessons(
        week_day INTEGER NOT NULL,
	class_id INTEGER NOT NULL,
	lesson_hour INTEGER NOT NULL,
	teacher_id INTEGER NOT NULL,
	subject_id INTEGER NOT NULL,
	classroom_id INTEGER NOT NULL,
	PRIMARY KEY (class_id, lesson_hour)
    );
    CREATE TABLE IF NOT EXISTS Teachers(
	teacher_id INTEGER PRIMARY KEY,
	full_name TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS Classrooms(
	classroom_id INTEGER PRIMARY KEY,
	classroom_name TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS Subjects(
	subject_id INTEGER PRIMARY KEY,
	subject_name TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS Duties(
	lesson_hour INTEGER NOT NULL,
	teacher_id INTEGER NOT NULL,
	classroom_id INTEGER NOT NULL,
        week_day INTEGER NOT NULL,
	PRIMARY KEY (lesson_hour, teacher_id, classroom_id),
	FOREIGN KEY (teacher_id) REFERENCES Teachers(teacher_id),
	FOREIGN KEY (classroom_id) REFERENCES Classrooms(classroom_id),
	FOREIGN KEY (lesson_hour) REFERENCES LessonHours(lesson_num)
    );";
    match database.execute(query, []){
        Ok(_) => {
            println!("{} Sucessfully Initialized Database", SUCCESS);
        }
        Err(_) => {
            return Err(());
        }
    }
    println!("{} Opened database", SUCCESS);
    return Ok(());
}
