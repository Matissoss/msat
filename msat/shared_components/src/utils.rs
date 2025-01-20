///==============================================
///                 utils.rs
/// contains misc functions that i don't know
/// where to put
///==============================================

use std::{
    net::IpAddr, 
    str::FromStr,
    process::{
        Command,
        ExitStatus
    }
};

#[allow(warnings)]
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
                        Ok(ip)
                    }
                    else{
                        Err(())
                    }
                }
                else{
                    Err(())
                }
            }
            else{
                Err(())
            }
        }
        Err(error) => 
        {
            crate::visual::error(Some(error), "Error occured while getting public IP");
            Err(())
        }
    }
}

#[allow(warnings)]
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
    Ok(string)
}
#[allow(warnings)]
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
