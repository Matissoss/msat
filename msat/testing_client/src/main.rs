/// This file should be interpreted as 'developer documentation'
/// as it contains all request, general syntax and sample IO
/// ------------------------------------------------------------
/// testing_client was made by MateusDev as part of msat project
///

fn main() {
    //      Questions Explained
    //  Q: How to form request?
    //  A: Request for msat follow simple rule:
    //  msat/<Version>&method=<Method>+<Method Number>&password=<Password>&args=<Args>
    //  msat - header for ALL requests, must be provided or it is INCORRECT request
    //  <Version> - version is provided with Github releases section: 
    //  it is u16, f.e.:
    //  if version is 0.2 it is 20,
    //  if version is 0.23 it is 23
    //  if version is 1.0 it is 100 
    //  General rule: version (u16) = version (float) * 100
    //  <Method> either POST or GET, nothing except these two values (THEY MUST BE UPPERCASE),
    //  <Method Number> - number specyfing what server will do. 0 is reserved for testing/debug
    //  purposes.
    //  <Password> - password that server uses to authenticate requests.
    //  <Args> - String/Number values separated by ','
    //  Q: Where does app_server function?
    //  A: Depends on config.toml file in data directory, but
    //  It ALWAYS is hosted on port 8888 
    //  Q: TCP/UDP?
    //  A: TCP 
    //
    //  =================> Few things:
    //  1. Version used in this documentation is '10', but it might be different depending on
    //     release,
    //  2. Rust types explained with C types:
    //      - u8  = byte,
    //      - u16 = unsigned short,
    //  =================> Code started
    //  This request as mentioned before is for debug purposes,
    //  should return: "msat/200-OK&get=Server-is-working!"
    //  =========================================================
            send_request("msat/10&method=GET+0&password=test&args=");
    //  ===========================================================
    //  This request GETs all lessons for one specific teacher_id
    //  Returns formatted data in following scheme:
    //  msat/200-OK/get=<class_id>+<classroom_id>+<subject_id>+<lesson_number>|<class_id-1>...
    //  Example with database rows:
    //  | class_id | classroom_id | subject_id | lesson_number | teacher_id |
    //  |-------------------------------------------------------------------|
    //  |    1     |      1      |     1      |      5        |      3     |
    //  |    2     |      2      |     2      |      1        |      1     |
    //  |    3     |      3      |     3      |      2        |      2     |
    //  |    4     |      4      |     4      |      3        |      5     |
    //  |    5     |      5      |     5      |      4        |      4     |
    //  |    6     |      6      |     6      |      6        |      1     |
    //  If request provides argument '1', it will return:
    //  
    //  IMPORTANT - instead of 2+2+2+1 or 6+6+6+6 server might return 
    //  Text/String representation if one is found in database
    //
    //  msat/200-OK&get=2+2+2+1|6+6+6+6
    //  If request provides argument '6', it will return:
    //  msat/204-No-Content
    //  If request provides wrong number (not 16-bit), server will return Parse Int Error
    //  ==============================================================
        send_request("msat/10&method=GET+1&password=test&args=1");
    //  ==============================================================
    //  This request automatically gets hour and minute of start/end in HHMM format, f.e. if hour
    //  is 13 and minute is 43 it will format as 1343 - HHMM. It is automated request
    //  Example with database rows:
    //  | lesson_number | start_time | end_time |
    //  |---------------------------------------|
    //  |      1       |    0745     |   0825   |
    //  |      2       |    0900     |   1000   |
    //  |      3       |    1200     |   1300   |
    //  If time in server's time zone is f.e. 8:00 (800), it will check which lesson it is (1),
    //  and it will return:
    //  msat/200-OK&get=745+825
    //  If time is 12:13 (1213) it will return:
    //  msat/200-OK&get=1200+1300 
    //  If time is 13:43 (1343) it will return:
    //  msat/204-No-Content
    //  ============================================================
        send_request("msat/10&method=GET+2&password=test&args=");
    //  ============================================================
    //  This request requires one argument, type: u16 for break number
    //  break number can be got in GET+6 request, which will be covered later
    //  QUICK NOTE: break_num == lesson_number (break_num is equal to lesson_number)
    //  f.e. if lesson number is 2, then break number is 2,
    //  lesson_number - break_num -> |increment by one| lesson_number - break_num
    //  Example with database rows (not connected with example in 2 case):
    //  |   break_num  | start_time | end_time |
    //  |---------------------------------------|
    //  |      1       |    0750     |   0800   |
    //  |      2       |    0900     |   1000   |
    //  |      3       |    1200     |   1215   |
    //  If request gives argument '3' it will return:
    //  msat/200-OK&get=1200+1215
    //  If request gives argument '4' it will return:
    //  msat/204-No-Content
    //  ============================================================
        send_request("msat/10&method=GET+3&password=test&args=3");
    //  ============================================================
    //  This request requires one argument, type: u16 for teacher_id
    //  it will return booleen (true/false) and if it is true then additional data 
    //  This request returns whether teacher is on duty or not
    //  Example with database rows (not actual database):
    //  |  teacher_id  |  break_num  |  duty_place_name  |    week_day   |
    //  |----------------------------------------------------------------|
    //  |      1       |     1      |   "Hallway-A"     |        4       |
    //  |      2       |     1      |   "Hallway-B"     |        4       |
    //  |      6       |     3      |   "Hallway-C"     |        5       |
    //  |      4       |     4      |   "Hallway-D"     |        6       |
    //  |      2       |     5      |   "Hallway-E"     |        7       |
    //  If request passes 1 for argument and it is Thursday (4 as u8), server will return:
    //  msat/200-OK&get=true+Hallway-A
    //  If request passes 1 for argument and it is Wednesday (3 as u8), server will return:
    //  msat/200-OK&get=false
    //  ============================================================
        send_request("msat/10&method=GET+4&password=test&args=1");
    //  ============================================================
    //  This request gets classroom & class teacher has/will have lessons with, requires
    //  one argument, type: u16 for teacher_id
    //  Example with database (not connected to actual database):
    //  |  teacher_id  | lesson_num  |    Classroom     |      Class     |
    //  |----------------------------------------------------------------|
    //  |      1       |     1      |   "Classroom-A"   |   "Class-7B"   |
    //  |      1       |     2      |   "Classroom-B"   |   "Class-8A"   |
    //  Server will try to get lesson_number, if it fails (f.e. it is break), it will get 
    //  break_num (go back to QUICK NOTE in request 3) and increment it by 1,
    //  f.e. If teacher_id is 1 and lesson_number will be recognized as 1, then server will
    //  return:
    //  msat/200-OK&get=false+Classroom-A+Class-7B
    //  f.e. If teacher_id is 1 and lesson_number won't be recognized, but break_num will as 1,
    //  then server will add 1 to break_num, giving lesson_number 2 and server will return:
    //  msat/200-OK&get=true+Classroom-B+Class-8A
    //  If argument type booleen in response will be true, then it means that server HAD TO
    //  INCREMENT value and it means that NEXT LESSON will be with Class-8A in classroom Classroom-A
    //  if argument type booleen in response will be false, then it means that server 
    //  DIDN't HAD TO increment lesson_number and that this lesson IS CURRENT
    //  ===========================================================
        send_request("msat/10&method=GET+5&password=test&args=1");
    //  ============================================================
    //  This request gets lesson_number/hour and doesn't require any other argument
    //  ===========================================================
        send_request("msat/10&method=GET+6&password=test&args=");
    //  ============================================================
    //  This request gets classroom using its ID, enter u16, get String
    //  ===========================================================
        send_request("msat/10&method=GET+7&password=test&args=1");
    //  ============================================================
    //  get class by ID, enter u16, get String
    //  ============================================================
        send_request("msat/10&method=GET+8&password=test&args=1");
    //  ============================================================
    //  get teacher by ID, enter u16, get String
    //  ============================================================
        send_request("msat/10&method=GET+9&password=test&args=1");
    //  ============================================================
    //  get break number represented as u8 
    //  ============================================================
        send_request("msat/10&method=GET+10&password=test&args=");
    //  ============================================================
    //
    //  POST requests - Coming Soon...
}


use std::net::TcpStream;
use std::io::{Read,Write};

fn send_request(request: &str){
    println!("---");
    let mut stream : TcpStream = TcpStream::connect("127.0.0.1:8888").unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = [0u8; 1024];
    let len = stream.read(&mut response).unwrap();
    println!("{}", request);
    let response_as_str = String::from_utf8_lossy(&response[0..len]).to_string();
    if response_as_str.len() == 0{
        println!("No Data");
    }
    else{
        println!("{}", response_as_str);
    }
}
