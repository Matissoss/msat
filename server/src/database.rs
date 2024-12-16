use tokio;
use crate::Configuration;
use crate::SchoolDay;

pub async fn init() -> Result<(), ()>{
    // Check for ALL files, if some miss, create them
    match std::fs::read_dir("data"){
        Ok(_) => {}
        Err(_) => {
            match std::fs::create_dir("data"){
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error creating directory: {}", e);
                    return Err(());
                }
            }
        }
    }
    let config : Option<Configuration> = match tokio::fs::read_to_string("data/config.toml").await{
        Ok(v) => {
            match toml::from_str::<Configuration>(&v){
                Ok(v1) => Some(v1),
                Err(e) => {
                    eprintln!("Couldn't parse data/config.toml to Configuration struct:\n{}", e);
                    None
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            if std::fs::exists("data/config.toml").unwrap_or(false) == false{
                match std::fs::write("data/config.toml", toml::to_string(&Configuration::default()).unwrap_or("".to_string())){
                    Ok(_) => {
                    }
                    Err(e1) => {
                        eprintln!("Cannot write content to file: {}", e1);
                    }
                }
            }
            None
        }
    };
    let class_nums : u8 = match config{
        Some(v) => {
            match v.number_of_classes{
                Some(v1) => {
                    v1
                }
                None => {8}
            }
        }
        None => {
            println!("Number Of Classes is not defined! Using 8 instead");
            8
        }
    };
    let dir_exists =match std::fs::read_dir("data/lessons"){
        Ok(_) => {true}
        Err(_) => {
            match std::fs::create_dir("data/lessons"){
                Ok(_) => {true}
                Err(e) => {
                    eprintln!("Error creating directory: {}", e);
                    false
                }
            }
        }
    };
    if dir_exists == true{
        for class in 1..class_nums+1{
            let sub_dir_exists = match std::fs::read_dir(format!("data/lessons/class{}", class)){
                Ok(_) => {true}
                Err(_) => {
                    match std::fs::create_dir(format!("data/lessons/class{}", class)){
                        Ok(_) => {true}
                        Err(e) => {
                            eprintln!("Error creating directory: {}", e);
                            false
                        }
                    }
                }
            };
            if sub_dir_exists{
                for day in 1..=7u8{
                    match std::fs::exists(format!("data/lessons/class{}/{}", class, day)){
                        Ok(v) => {
                            if v == false{
                                match std::fs::write(format!("data/lessons/class{}/{}.toml", class, day), 
                                    toml::to_string(&SchoolDay::default()).unwrap_or("".to_string())){
                                    Ok(_) => {},
                                    Err(e) => {
                                        eprintln!("Error creating file: {}", e);
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            match std::fs::write(format!("data/lessons/class{}/{}.toml", class, day), 
                                toml::to_string(&SchoolDay::default()).unwrap_or("".to_string())){
                                Ok(_) => {},
                                Err(e) => {
                                    eprintln!("Error creating file: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    else{
        return Err(());
    }
    return Ok(());
}
