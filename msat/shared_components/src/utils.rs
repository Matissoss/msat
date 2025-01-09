///==============================================
///                 utils.rs
/// contains misc functions that i don't know
/// where to put
///==============================================

use std::{net::IpAddr, str::FromStr};

pub fn quick_match<T, E>(statement: Result<T, E>) -> Option<T>
{
    match statement{
        Ok(v) => return Some(v),
        Err(_) => return None
    }
}
pub fn format_two_digit_time(time1: u8, time2: u8) -> String{
    if time1 < 10 && time2 < 10{
        return format!("0{}0{}", time1, time2);
    }
    else if time1 < 10 && time2 > 10{
        return format!("0{}{}", time1, time2);
    }
    else if time1 > 10 && time2 < 10{
        return format!("{}0{}", time1, time2);
    }
    else{
        return format!("{}{}", time1, time2);
    }
}
pub fn format_mmdd(input: &str) -> Result<(u8, u8), ()>{
    if input.len() != 4{
        return Err(());
    }
    let month = match input[0..1].parse::<u8>(){
        Ok(v) => v,
        Err(_) => {
            return Err(());
        }
    };
    let day = match input[2..3].parse::<u8>(){
        Ok(v) => v,
        Err(_) => {
            return Err(());
        }
    };
    return Ok((month, day));
}

pub fn format_lessonh(hour: u16) -> String{
    if hour < 10{
        return format!("00:0{}", hour);
    }
    else if hour < 100{
        return format!("00:{}", hour);
    }
    else if hour < 1000{
        let hour_chars = hour.to_string().chars().collect::<Vec<char>>();
        format!("0{}:{}{}", hour_chars[0], hour_chars[1], hour_chars[2])
    }
    else if hour < 10000{
        let hour_chars = hour.to_string().chars().collect::<Vec<char>>();
        format!("{}{}:{}{}", hour_chars[0], hour_chars[1], hour_chars[2], hour_chars[3])
    }
    else{
        return format!("00:00");
    }
}

pub fn format_time(time: u32) -> String{
    if time < 10{
        return format!("0{}", time);
    }
    else{
        return format!("{}", time);
    }
}

use std::process::{Command, ExitStatus};

use crate::cli;
pub fn get_public_ip() -> Result<IpAddr, ()>{
    let curl_result = Command::new("curl")
        .arg("https://api.ipify.org/")
        .output();
    match curl_result{
        Ok(output) => {
            if ExitStatus::success(&output.status)
            {
                if let Ok(string) = String::from_utf8(output.stdout){
                    if let Ok(ip) = IpAddr::from_str(&string){
                        return Ok(ip)
                    }
                    else{
                        return Err(());
                    }
                }
                else{
                    return Err(())
                }
            }
            else{
                return Err(());
            }
        }
        Err(error) => 
        {
            cli::print_error("Error occured while getting public IP", error);
            return Err(());
        }
    }
}

pub fn encode_ip(ip: IpAddr, port: u16) -> Result<String, ()>{
    let buf : [u8; 4] = match ip{
        IpAddr::V4(v4) => v4.octets(),
        IpAddr::V6(_) => return Err(())
    };
    let mut string : String = String::new();
    for byte in buf{
        let mut as_hex = format!("{:x}", byte);
        if as_hex.len() == 1{
            as_hex = format!("0{}", as_hex);
        }
        string.push_str(&format!("{}{}", as_hex.len(), as_hex));
    }
    let port_hex = &format!("{:x}", port);
    string.push_str(&format!("_{}{}", port_hex.len(), port_hex));
    return Ok(string);
}

pub fn decode_ip(encoded_ip: String) -> Result<([u8; 4], u16), ()>{
    if let Some((ip_hex, port_hex)) = encoded_ip.split_once('_') {
        let mut ip_bytes = [0u8; 4];
        let mut start = 0;
        let mut ip_byte_index = 0;
        while ip_byte_index < 4 {
            start += 1;
            let hex_str = &ip_hex[start..start + 2];
            ip_bytes[ip_byte_index] = u8::from_str_radix(hex_str, 16).map_err(|_| ())?;
            start += 2;
            ip_byte_index += 1;
        }
        let port_len = port_hex[0..1].parse::<usize>().map_err(|_| ())?;
        let port = u16::from_str_radix(&port_hex[1..port_len+1], 16).map_err(|_| ())?;
        Ok((ip_bytes, port))
    } else {
        Err(())
    }
}
#[cfg(test)]
mod tests{
    use std::str::FromStr;

    use super::*;
    #[test]
    fn public_ip(){
        println!("{}", get_public_ip().unwrap());
    }
    #[test]
    fn encode(){
        println!("{}", encode_ip(IpAddr::from_str("192.168.43.48").unwrap(), 12000).unwrap());
        println!("{}", encode_ip(IpAddr::from_str("127.0.0.1").unwrap(), 8888).unwrap());
    }
    #[test]
    fn decode(){
        println!("{:?}", decode_ip("2c02a822b230_422b8".to_string()).unwrap());
        assert_eq!(Ok(([192, 168, 43, 48], 8888)), decode_ip("2c02a822b230_422b8".to_string()));
        assert_eq!(Ok(([127, 0, 0, 1], 8888)), decode_ip("27f200200201_422b8".to_string()));
        println!("{:?}", decode_ip("22522f2d62fe_422b8".to_string()));
    }
}
