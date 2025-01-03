///===============================
///             cli.rs
/// part responsible for printing
/// and display
///===============================

// Local Imports
pub const SUCCESS : &str = "[     OK     ]";
pub const ERROR   : &str = "[     ERR     ]";
pub const VERSION : &str = "10";

pub fn print_dashboard(){
    println!(
"=========================
          msat {}
=========================",
    VERSION);
}

pub fn print_error<E>(info: &str, error: E)
    where E: std::fmt::Display
{
    println!("{} {}: {}", ERROR, info, error);
}
pub fn print_success(info: &str){
    println!("{} {}", SUCCESS, info);
}
pub fn critical_error<E>(info: &str, error: E) -> !
    where E: std::fmt::Display
{
    println!("{} {}: {}", ERROR, info, error);
    std::process::exit(-1);
}
