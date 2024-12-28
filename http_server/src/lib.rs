use std::{
    io::{
        Read,
        Write
    }, 
    net::{
        IpAddr,
        Ipv4Addr,
        TcpListener,
        TcpStream
    }, 
    sync::Arc
};

use tokio::{
    fs,
    sync::Mutex,
};

pub async fn init(ip_addr: IpAddr){
    let final_address = format!("{}:8000", ip_addr.to_string());
    let listener : TcpListener = match TcpListener::bind(final_address){
        Ok(v) => v,
        Err(_) => {
            std::process::exit(-1)
        }
    };
    println!("initialized");
    loop{
        for s in listener.incoming(){
            if let Ok(mut stream) = s{
                let mut buffer = [0u8; 2048];
                if let Ok(len) = stream.read(&mut buffer){
                    if len == 0{
                        continue;
                    }
                    else{
                        let request = String::from_utf8_lossy(&buffer).to_string();
                        let lines = request
                            .lines()
                            .filter(|s| s.is_empty() == false)
                            .collect::<Vec<&str>>();
                        let mut types : Vec<String> = vec![];
                        let mut file_path : String = String::new();
                        for line in lines{
                            let request = line.split_whitespace()
                                .map(|s| s.to_string())
                                .collect::<Vec<String>>();
                            print_vector(&request);
                            if request.contains(&"GET".to_string()){ 
                                let split_line : Vec<String> = request.clone().into_iter()
                                    .filter(|s| !s.starts_with("GET") && s.starts_with('/'))
                                    .collect();
                                for w in split_line{
                                    if w == "/"{
                                        file_path = "web/index.html".to_string();
                                    }
                                    else{
                                        file_path = format!("web{}", w)
                                    }
                                }
                            }
                            if request.contains(&"Accept:".to_string()){
                                let split_line : Vec<String> = request.into_iter()
                                    .filter(|s| !s.starts_with("Accept:"))
                                    .collect();
                                for w in split_line{
                                    types = split_string_by(&w, ',');
                                }
                            }
                        }
                        if types.len() == 0{
                            types = vec!["*/*".to_string()];
                        }
                        // End of checks
                        let binary : bool = types[0].starts_with("image");
                        let f_type = &types[0];
                        println!("FILE_PATH = {}\n===", file_path);

                        if binary == false{
                            if let Ok(buf) = tokio::fs::read(file_path).await{
                                if let Ok(string) = String::from_utf8(buf.clone()){
                                    stream.write_all(
                                        format!("HTTP/1.1 200 OK\r\nContent-Length:{}\r\nContent-Type:{}\r\n\r\n{}",
                                            string.len(), f_type, string)
                                        .as_bytes())
                                    .unwrap();
                                }
                                else{
                                    let string = String::from_utf8_lossy(&buf).to_string();
                                    stream.write_all(
                                        format!("HTTP/1.1 200 OK\r\nContent-Length:{}\r\nContent-Type:{}\r\n\r\n{}",
                                            string.len(), f_type, string).as_bytes())
                                    .unwrap()
                                };
                            }
                            else{
                                not_found(&mut stream);
                            }
                        }
                        else{
                            if let Ok(buf) = tokio::fs::read(file_path).await{
                                let http_header = 
                                    format!("HTTP/1.1 200 OK\r\nContent-Length:{}\r\nContent-Type:{}\r\nConnection: keep-alive\r\n\r\n",
                                    buf.len(), f_type);
                                stream.write_all(http_header.as_bytes()).unwrap();
                                let mut vector = Vec::with_capacity(buf.len() + http_header.len());
                                vector.extend_from_slice(buf.as_slice());
                                vector.extend_from_slice(http_header.as_bytes());
                                stream.write_all(&buf).unwrap();
                            }
                            else{
                                not_found(&mut stream);
                            }
                        }
                    }
                }
            }
            else{
                continue;
            }
        }
    }
}
fn not_found(tcp: &mut TcpStream){
    match tcp.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n<h1>404 - Not Found</h1>"){
        Ok(_) => println!("Success"),
        Err(e) => eprintln!("Error: {}",e)
    };
}

fn print_vector(vec: &Vec<String>)
{
    let mut finstr = "[".to_string();
    for e in vec{
        finstr.push_str(e);
        finstr.push(';');
    }
    finstr.push(']');
    println!("{}", finstr);
}

fn get_types(line: String) -> Vec<String>{
    let split_line = line.split_whitespace()
        .collect::<Vec<&str>>();
    let mut types : Vec<String> = vec![];
    for s in split_line{
        if !s.starts_with("Accept:"){
            types = split_string_by(s, ',');
        }
    }
    types
}
fn split_string_by(string: &str, chr: char) -> Vec<String>{
    let mut temp_buf = vec![];
    let mut finvec = vec![];
    for c in string.chars().collect::<Vec<char>>(){
        if c != chr{
            temp_buf.push(c);
        }
        else if c == ' '{
            continue;
        }
        else{
            finvec.push(String::from_iter(temp_buf.iter()));
            temp_buf = vec![];
            continue;
        }
    }
    finvec
}

#[cfg(test)]
mod tests{
    use super::*;
    #[tokio::test]

    async fn start(){
        init(IpAddr::from(Ipv4Addr::from([127,0,0,1]))).await;
    }
}
