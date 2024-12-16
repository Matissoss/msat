use core::str;
use std::{
    collections::HashMap, fs, io::{
        Read, Write
    }, net::{
        IpAddr, Ipv4Addr, TcpListener, TcpStream
    }
};
use std::sync::Arc;
use serde::{Serialize,Deserialize};

#[derive(Clone,Debug,Default)]
enum Request{
    GET(u8, String),
    UPL(u8, String, String),
    #[default]
    Other
}

impl ToString for Request{
    fn to_string(&self) -> String {
        match &self{
            Self::GET(t, v) => {
                format!("GET {} : {}", t, v)
            },
            Self::UPL(t,_,v) => {
                format!("UPL {} : {}", t, v)
            },
            _ => "None".to_string()
        }
    }
}

#[derive(Serialize,Deserialize,Clone,Copy,Debug)]
struct Lesson{
    classroom  : u8,
    subject    : u8,
    teacher    : u8
}
#[derive(Serialize,Debug,Deserialize)]
struct SchoolDay{
    lessons : HashMap<u8, Lesson>
}

#[derive(Serialize,Deserialize,Clone,Debug, Default)]
struct Configuration{
    password : String,
    ip_addr  : Option<IpAddr>,
    teachers : HashMap<u8, String>,
    subjects : HashMap<u8, String>,
    classes  : HashMap<u8, String>
}

#[tokio::main]
async fn main() {
    let mut config_file : String = String::from("");
    if fs::exists("data").unwrap_or(false) == false{
        fs::create_dir("data").unwrap_or(());
    }
    else{
        config_file = match fs::read_to_string("data/config.toml"){
            Ok(v) => {v},
            Err(e) => {
                if !fs::exists("data/config.toml").unwrap_or(false){
                    fs::write("data/config.toml", 
                        toml::to_string(&Configuration::default()).unwrap_or("".to_string()))
                        .unwrap_or(());
                }
                else{
                    eprintln!("Couldn't read file in directory: data/config.toml: {}", e);
                }
                std::process::exit(-1);
            }
        };
    }
    let shared_config : Arc<Configuration> = match toml::from_str(&config_file){
        Ok(v) => Arc::new(v),
        Err(e) => {
            eprintln!("Couldn't convert file /data/config.toml to Configuration Struct: {}", e);
            std::process::exit(-1);
        }
    };
    let listener : TcpListener = match TcpListener::bind
        (format!("{}:8000", shared_config.ip_addr.unwrap_or(IpAddr::from(Ipv4Addr::from([127,0,0,1]))))){
            Ok(v) => v,
            Err(e) => {
                if shared_config.ip_addr.is_none(){
                    eprintln!(
                    "data/config.toml doesn't contain any IP Address, like: `127.0.0.1`;
                    Server automatically used this address with port 8000, but it wasn't able to connect : {}", 
                    e);
                    std::process::exit(-1);
                }
                eprintln!("Error connecting to address: `{}` : {}", shared_config.ip_addr.unwrap(), e);
                std::process::exit(-1);
            }
    };
    loop{
        for s in listener.incoming(){
            let stream : Option<TcpStream> = match s{
                Ok(v) => Some(v),
                Err(e) => {
                    eprintln!("Couldn't establish connection with TcpStream : {}", e);
                    None
                }
            };
            if stream.is_some(){
                tokio::spawn(async{
                    handle_connection(stream.unwrap()).await
                });
            }
        }
    }
}

async fn handle_connection(mut stream: TcpStream){
            let local_addr = stream.local_addr().unwrap();
            let addr_ip = local_addr.ip();
            let mut buffer = [0u8;1024];
            let lenread = stream.read(&mut buffer).unwrap();
            let mut passwd : Option<&str> = None;
            let request : Request = match &buffer[0..3]{
                b"GET" => {
                    Request::GET(buffer[4], String::from_utf8_lossy(&buffer[6..lenread]).to_string())
                },
                b"UPL"|b"POS" => {
                    let string = String::from_utf8_lossy(&buffer[6..lenread]).to_string();
                    for (i, chr) in string.chars().enumerate(){
                        let chars = string.chars().collect::<Vec<char>>();
                        if chr == 'p' && chars[i+1] == 's' && chars[i+2] == 'w'&&chars[i+3]=='d'&&chars[i+4]=='='{
                            let mut index : usize = i+5;
                            for chr1 in &chars[i+5..]{
                                if chr1 == &' '{
                                    break;
                                }
                                else{
                                    index+=1;
                                }
                            }
                            passwd = Some(str::
                                from_utf8(&buffer[(6+i+5)..6+index])
                                .unwrap_or(""));
                            break;
                        }
                    };
                    if passwd.is_none(){
                        stream.write_all(b"Couldn't get password").unwrap();
                        return;
                    }
                    Request::UPL(buffer[4], passwd.unwrap().to_string(),String::from_utf8_lossy(&buffer[6..lenread]).to_string())
                },
                _ => Request::Other
            };
            let parsed_out = parse_request(request.clone(), &passwd.unwrap_or("").to_string()).await;
            println!("---\nConnection Established with {}!\nRequest: {}\nLength of message: {}\nMessage:{}\nParsed:{}", 
                addr_ip,request.to_string(),lenread,String::from_utf8_lossy(&buffer[5..buffer.len()]),parsed_out);
            stream.write_all(parsed_out.as_bytes()).unwrap();
}

async fn parse_request(request: Request, input_password : &String) -> String {
    match request{
        Request::GET(t, value) => {
            match t{
                65 => {
                    fs::read_to_string("data/version.ver").unwrap_or("-1".to_string())
                },
                66 => {
                    "1".to_string()
                },
                76 => {
                    "-1".to_string()
                    // Plan lekcji dla użytkownika
                }
                77 => {
                    "-1".to_string()
                    // Informacje na temat dyżuru True/False
                }
                78 => {
                    "-1".to_string()
                    // Informacje na temat dyżuru String
                }
                _ => {"-1".to_string()}
            }
        },
        Request::UPL(t, p, v) => {
            let password = match get_password().await{
                Ok(v)=>v,
                Err(_)=>None
            };
            let can_progress = if password.is_some() && !input_password.is_empty(){
                &password.unwrap() == input_password
            }
            else{false};
            if can_progress{
                match t{
                    _ => {
                        "188".to_string()
                    }
                }
            }   
            else{"190".to_string()}},
        _ => {"198".to_string()}
    }
}

async fn get_password() -> Result<Option<String>, ()>{
    match fs::read_to_string("data/config.toml"){
        Ok(v) => {
            match toml::from_str::<Configuration>(&v){
                Ok(b) => {
                    if !b.password.is_empty(){
                        return Ok(Some(b.password));
                    }
                    else{
                        return Ok(None);
                    }
                }
                Err(_) => {
                    return Err(());
                }
            }
        }
        Err(_) => {
            return Err(());
        }
    }
}
async fn get_teachers() -> Result<Option<HashMap<u8, String>>, ()>{
    match fs::read_to_string("data/config.toml"){
        Ok(v) => {
            match toml::from_str::<Option<Configuration>>(&v){
                Ok(b) => {
                    if b.is_some(){
                        if !b.clone().unwrap().teachers.is_empty(){
                            return Ok(Some(b.unwrap().teachers));
                        }
                        else{
                            return Ok(None);
                        }
                    }else{
                        return Ok(None);
                    }
                },
                Err(_) => Err(())
            }
        },
        Err(_) => {
            return Err(());
        }
    }
}
async fn get_lessons() -> Result<Option<HashMap<u8,String>>, ()>{
    match fs::read_to_string("data/config.toml"){
        Ok(v) => {
            match toml::from_str::<Option<Configuration>>(&v){
                Ok(b) => {
                    if b.is_some(){
                        if !b.clone().unwrap().subjects.is_empty(){
                            return Ok(Some(b.unwrap().subjects));
                        }
                        else{
                            return Ok(None);
                        }
                    }
                    else{
                        return Ok(None);
                    }
                }
                Err(_) =>{
                    return Err(());
                }
            }
        }
        Err(_) => {return Err(());}
    }
}
