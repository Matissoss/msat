use crate::types::Configuration;

pub async fn get_password() -> Option<String>{
    if let Ok(v) = tokio::fs::read_to_string("data/config.toml").await{
        if let Ok(c) = toml::from_str::<Configuration>(&v){
            if c.password.is_empty(){
                return None;
            }
            else{
                return Some(c.password);
            }
        }
    }
    if let Ok(_) = std::fs::write("data/config.toml",toml::to_string(&Configuration::default()).unwrap_or("".to_string())){
    };
    return None;
}

