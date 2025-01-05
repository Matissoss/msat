///==============================================
///                 utils.rs
/// contains misc functions that i don't know
/// where to put
///==============================================


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
pub fn format_time(time: u32) -> String{
    if time < 10{
        return format!("0{}", time);
    }
    else{
        return format!("{}", time);
    }
}

