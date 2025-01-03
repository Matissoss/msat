use crate::types::Configuration;

pub async fn get() -> Result<Option<Configuration>, ()>{
    if let Ok(v) = tokio::fs::read_to_string("data/config.toml").await{
        if let Ok(value) = toml::from_str::<Configuration>(&v){
            if value == Configuration::default(){
                Ok(None)
            }
            else{
                Ok(None)
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
