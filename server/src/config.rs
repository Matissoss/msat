use crate::Configuration;


pub async fn get() -> Result<Option<Configuration>, ()>{
    match tokio::fs::read_to_string("data/config.toml").await{
        Ok(v) => {
            match toml::from_str::<Configuration>(&v){
                Ok(b) => {
                    if b != Configuration::default(){
                        Ok(Some(b))
                    }
                    else{
                        Ok(None)
                    }
                },
                Err(e) => {
                    eprintln!("{}", e);
                    Err(())
                }
            }
        }
        Err(_) => {
            match tokio::fs::write("data/config.toml", toml::to_string(&Configuration::default()).unwrap_or("".to_string())).await{
                Ok(_) => {}
                Err(_) => return Err(())
            }
            Err(())
        }
    }
}
