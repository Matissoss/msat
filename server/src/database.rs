use rusqlite::Connection;

pub async fn init() -> Result<(), ()>{
    match std::fs::read_dir("data"){
        Ok(_) => {},
        Err(_) => {
            match std::fs::create_dir("data"){
                Ok(_) => {}
                Err(e)=>{
                    eprintln!("12: {}", e);
                    return Err(());
                }
            }
        }
    }
    let database: Connection = match Connection::open("data/database.db"){
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error connecting to database: {}", e);
            return Err(());
        }
    };
    println!("[     OK     ] Opened database");
    match database.execute("CREATE TABLE IF NOT EXISTS lessons (
    week_day INTEGER NOT NULL,
    class_id INTEGER NOT NULL,
    classroom_id INTEGER NOT NULL,
    subject_id INTEGER NOT NULL,
    teacher_id INTEGER NOT NULL,
    lesson_number INTEGER NOT NULL,
    FOREIGN KEY(subject_id) REFERENCES subjects(id),
    FOREIGN KEY(class_id) REFERENCES classes(id),
    FOREIGN KEY(classroom_id) REFERENCES classrooms(id),
    FOREIGN KEY(teacher_id) REFERENCES teachers(id)
    );", ()){
        Ok(_) => {}
        Err(_) => {return Err(())}
    };
    println!("[     OK     ] Executed creating lessons");
    match database.execute("CREATE TABLE IF NOT EXISTS classrooms (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
    );", ()){
        Ok(_) => {}
        Err(_) => {return Err(())}
    };
    println!("[     OK     ] Executed creating classrooms");
    match database.execute("CREATE TABLE IF NOT EXISTS teachers (
    id INTEGER PRIMARY KEY,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL
    );", ()){
        Ok(_) => {}
        Err(_) => {return Err(())}
    };
    println!("[     OK     ] Executed creating teachers");
    match database.execute("CREATE TABLE IF NOT EXISTS duty (
    id INTEGER PRIMARY KEY,
    teacher_id INTEGER NOT NULL,
    break_number INTEGER NOT NULL,
    FOREIGN KEY(teacher_id) REFERENCES teachers(id)
    );", ()){
        Ok(_) => {}
        Err(_) => {return Err(())}
    };
    println!("[     OK     ] Executed creating duty");
    // date is saved in MMDD format
    match database.execute("CREATE TABLE IF NOT EXISTS hours (
    id INTEGER PRIMARY KEY,
    date TEXT NOT NULL, 
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL
    );", ()){
        Ok(_) => {}
        Err(_) => {return Err(())}
    };
    println!("[     OK     ] Executed creating hours");
    match database.execute("CREATE TABLE IF NOT EXISTS classes (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
    );", ()){
        Ok(_) => {}
        Err(_) => {return Err(())}
    };
    println!("[     OK     ] Executed creating classes");
    match database.execute("CREATE TABLE IF NOT EXISTS subjects (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
    );", ()){
        Ok(_) => {}
        Err(_) => {return Err(())}
    };
    println!("[     OK     ] Executed creating subjects");
    return Ok(());
}
