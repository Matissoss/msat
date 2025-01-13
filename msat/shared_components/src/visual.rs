///==============================
/// Part of msat responsible for
/// handling visual things like
/// displaying info
///==============================

use colored::Colorize;
use crate::consts;

pub fn main(){
    if consts::ARGS.contains(&"--help".to_string()){
        println!("Help");
        std::process::exit(0);
    }
}

pub fn info(inf: &str){
    if *consts::COLOR_ALLOWED == true{
        println!("{} {}", consts::INFO.bold().blue(), inf);
    }
    else{
        println!("{} {}", consts::INFO, inf);
    }
}
pub fn success(inf: &str){
    if *consts::COLOR_ALLOWED == true{
        println!("{} {}", consts::SUCCESS.bold().blue(), inf);
    }
    else{
        println!("{} {}", consts::SUCCESS, inf);
    }
}
pub fn critical_error<E>(err: Option<E>, inf: &str) -> !
where E: std::fmt::Display
{
    error(err, inf);
    std::process::exit(-1);
}

pub fn error<E>(err: Option<E>, inf: &str)
where E: std::fmt::Display
{
    if *consts::COLOR_ALLOWED == true{
        match err{
            Some(err) => {
                println!("{} {}: {}", consts::ERROR.red().bold(), inf, err);
            }
            None => {
                println!("{} {}", consts::ERROR.red().bold(), inf)
            }
        }
    }
    else{
        match err{
            Some(err) => {
                println!("{} {}: {}", consts::ERROR, inf, err);
            }
            None => {
                println!("{} {}", consts::ERROR, inf)
            }
        }
    }
}

pub fn debug(inf: &str){
    if *consts::DEBUG_MODE == true{
        println!("{} {}", consts::DEBUG, inf);
    }
}
