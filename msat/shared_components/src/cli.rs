///===============================
///             cli.rs
/// part responsible for printing
/// and display
///===============================

use colored::Colorize;
use std::sync::LazyLock;

// Local Imports
pub const SUCCESS : &str = "[     OK     ]";
pub const ERROR   : &str = "[    ERROR   ]";
pub const DEBUG   : &str = "[    DEBUG   ]";
pub const INFO    : &str = "[    INFO    ]";
pub static ARGS   : LazyLock<Vec<String>> = LazyLock::new(|| {
    std::env::args().collect()
});

use crate::VERSION;

pub fn main(){
    if ARGS.contains(&"--help".to_string()){
        println!(
            "
msat - tool for school administration,
made by MateusDev

Options:
--color - prints data in color,
--debug - gives some additional data"
        );
        std::process::exit(0);
    }
}

pub fn print_dashboard(){
    println!(
"=========================
          {} {}
=========================",
    "msat".yellow(),
    VERSION);
}

pub fn print_error<E>(info: &str, error: E)
    where E: std::fmt::Display
{
    if ARGS.contains(&"--color".to_string()){
        println!("{} {}: {}", ERROR.red(), info, error.to_string().on_red());
    }
    else{
        println!("{} {}: {}", ERROR, info, error);
    }
}
pub fn print_errwithout(info: &str){
    if ARGS.contains(&"--color".to_string()){
        println!("{} {}", ERROR.red(), info.red());
    }
    else{
        println!("{} {}", ERROR, info);
    }
}
pub fn print_success(info: &str){
    if ARGS.contains(&"--color".to_string()){
        println!("{} {}", SUCCESS.green(), info.green());
    }
    else{
        println!("{} {}", SUCCESS, info);
    }
}

pub fn print_info(info: &str){
    if ARGS.contains(&"--color".to_string()){
        println!("{} {}", INFO.blue(), info.bold());
    }
    else{
        println!("{} {}", INFO, info)
    }
}

pub fn debug_log(info: &str){
    if ARGS.contains(&"--debug".to_string()){
        if ARGS.contains(&"--color".to_string()){
            println!("{} {}", DEBUG.yellow(), info);
        }
        else{
            println!("{} {}", DEBUG, info)
        }
    }
}

pub fn critical_error<E>(info: &str, error: E) -> !
    where E: std::fmt::Display
{
    if ARGS.contains(&"--color".to_string()){
        println!("{} {}: {}", ERROR.black().on_red(), info, error.to_string().black().on_red());
    }
    else{
        println!("{} {}: {}", ERROR, info, error);
    }
    std::process::exit(-1);
}
