///===========
/// Constants declaration
///===========
use std::sync::LazyLock;
use std::net::IpAddr;

pub const INFO    : &str = "[  INFO  ] ";
pub const DEBUG   : &str = "[  DEBUG  ]";
pub const ERROR   : &str = "[  ERROR  ]";
pub const SUCCESS : &str = "[   OK   ] ";
pub const VERSION : u16  = 50;
pub const SUPPORTED_VERSIONS : [u16; 1] = [50];
pub const CLEAR   : &str = 
    if cfg!(windows)
    {
        "cls"
    }
    else{
        "clear"
    };
// Lazylocks

pub static ARGS : LazyLock<Vec<String>> = LazyLock::new(|| {
    std::env::args().collect()
});
pub static COLOR_ALLOWED : LazyLock<bool> = LazyLock::new(|| {
    ARGS.contains(&"--color".to_string())
});
pub static DEBUG_MODE : LazyLock<bool> = LazyLock::new(|| {
    ARGS.contains(&"--debug".to_string())
});
pub static LOCAL_IP: LazyLock<IpAddr> = LazyLock::new(|| 
    {
        IpAddr::from([127, 0, 0, 1])
    }
);

