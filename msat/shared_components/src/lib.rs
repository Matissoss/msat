// Splitted
pub mod cli;
pub mod config;
pub mod database;
pub mod types;
pub mod password;
pub mod utils;

// Other misc/util tools

use rusqlite::OpenFlags;
use std::sync::LazyLock;
use std::net::IpAddr;

pub static SQLITE_FLAGS : LazyLock<OpenFlags> = LazyLock::new(|| 
{
    OpenFlags::SQLITE_OPEN_CREATE|OpenFlags::SQLITE_OPEN_READ_WRITE|OpenFlags::SQLITE_OPEN_FULL_MUTEX 
});
pub const VERSION : u16  = 30;
pub const CLEAR   : &str = if cfg!(windows)
{
    "cls"
}
else{
    "clear"
};
pub static LOCAL_IP: LazyLock<IpAddr> = LazyLock::new(|| 
    {
        IpAddr::from([127, 0, 0, 1])
    }
);


pub fn split_string_by(string: &str, chr: char) -> Vec<String> {
    let mut temp_buf = vec![];
    let mut finvec = vec![];
    for c in string.chars().collect::<Vec<char>>() {
        if c != chr {
            temp_buf.push(c);
        } else {
            finvec.push(String::from_iter(temp_buf.iter()));
            temp_buf = vec![];
        }
    }
    if temp_buf.is_empty() == false{
        finvec.push(String::from_iter(temp_buf.iter()));
    }
    finvec
}
pub fn vec_to_string<T>(vec: Vec<T>) -> String 
where T: ToString
{
    let mut final_value = "".to_string();
    for element in vec{
        if final_value.as_str() == ""{
            final_value = format!("{}", element.to_string());
        }
        else{
            final_value = format!("{}+{}", final_value, element.to_string());
        }
    }
    return "".to_string()
}
