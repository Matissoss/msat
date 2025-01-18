pub fn serialize(data: [u16; 3]) -> [u8; 6]{
    let ((byte1, byte2), (byte3, byte4), (byte5, byte6)) = 
    (slice_boolarray(binrep(data[0])), slice_boolarray(binrep(data[1])), slice_boolarray(binrep(data[2])));
    return [fobin(byte1),fobin(byte2),fobin(byte3),fobin(byte4),fobin(byte5),fobin(byte6)];
}

fn fobin(ba: [bool; 8]) -> u8{
    let mut power_of_two = 0u8;
    let mut final_numb = 0u8;
    for &b in ba.iter().rev(){
        if b{
            final_numb |= 1 << power_of_two;
        }
        power_of_two+=1;
    }
    return final_numb;
}

fn slice_boolarray(ba: [bool; 16]) -> ([bool; 8], [bool;8]){
    let (mut no, mut nt) = ([false; 8], [false; 8]);
    let mut i = 0usize;
    for b in ba{
        if i < 8{
            no[i] = b;
        }
        else{
            nt[i-8] = b;
        }

        i+=1;
    }
    return (no, nt);
}

/// WORKS FINE
fn binrep(numb: u16) -> [bool; 16]{
    let mut to_return = [false; 16];
    let temp = format!("{:b}", numb);
    for (i, c) in format!("{}{}", std::iter::repeat('0').take(16-temp.len()).collect::<String>(), temp).chars().enumerate(){
        to_return[i] = c == '1';
    }
    return to_return;
}

fn not_zero(data: u8) -> u16{
    if data!=0{
        data.into()
    }
    else{
        1
    }
} 

pub fn deserialize(data: [u8; 6]) -> [u16; 3]{
    return [not_zero(data[0]) * not_zero(data[1]), not_zero(data[2]) * not_zero(data[3]), not_zero(data[4]) * not_zero(data[5])];
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn ser_de_test(){
        let t = true;
        let f = false;
        let numbers = [7u16, 120u16, 16u16];
        assert_eq!(8, fobin([false, false, false, false, true, false, false, false]));
        assert_eq!(binrep(16), 
            [false, false, false, false, false, false, false, false, false, false, false, true, false, false, false, false]);
        assert_eq!(([t,f,f,f,f,f,f,t], [t,f,f,f,f,f,f,f]), 
            slice_boolarray(
                [t,f,f,f,f,f,f,t,t,f,f,f,f,f,f,f]));
        let serialized = serialize(numbers);
        assert_eq!([0, 7, 0, 120, 0, 16], serialized);
        assert_eq!(numbers, deserialize(serialized));
    }
}
