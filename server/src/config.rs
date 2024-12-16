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
        Err(e) => {
            eprintln!("{}", e);
            Err(())
        }
    }
}
