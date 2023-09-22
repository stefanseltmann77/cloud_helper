use std::collections::HashMap;
use std::net::IpAddr;

use config::Config;

const ENV_PREFIX: &str = "RUSTAPP";
const KEY: &str = "AWSSECGROUP";

async fn add_ip(own_ip: &str, port: i32, security_group_id: &str) {
    let shared_config = aws_config::load_from_env().await;
    let client: aws_sdk_ec2::Client = aws_sdk_ec2::Client::new(&shared_config);
    let builder = client.authorize_security_group_ingress();
    let dummy = builder.cidr_ip(own_ip).
        set_group_id(Some(security_group_id.to_string())).
        set_ip_protocol(Some("tcp".to_string())).
        set_from_port(Some(port)).
        set_to_port(Some(port))
        ;
    let result = dummy.send().await;
    match &result {
        Ok(output) => println!("Setting new rule,{:?}", output),
        Err(error) => {
            if error.to_string().contains("already exists") {
                println!("Rule for {}:{} already set!", own_ip, port);
            } else {
                println!("Error, {:?}", error)
            };
        }
    }
}

async fn get_ip() -> Option<IpAddr> {
    let public_ip = public_ip::addr().await;
    return public_ip;
}


struct AppConfig {
    security_group_id: String,
    ports: Vec<i32>,
}

impl AppConfig {
    fn new() -> AppConfig {
        let settings: HashMap<String, String> = Config::builder().
            add_source(config::Environment::with_prefix(ENV_PREFIX).
                try_parsing(true).separator("_")).
            build().unwrap().try_deserialize::<HashMap<String, String>>().
            unwrap();
        return AppConfig {
            security_group_id: settings.get(&KEY.to_lowercase()).unwrap().to_string(),
            ports: vec![22i32, 3306i32, 5000i32],
        };
    }
}

#[tokio::main]
async fn main() {
    let conf = AppConfig::new();
    let public_ip: IpAddr = get_ip().await.unwrap();
    let security_group_id: String = conf.security_group_id;
    let ports = conf.ports;
    for port in ports {
        add_ip(format!("{:}/32", public_ip.to_string()).as_str(), port, security_group_id.as_str()).await;
    }
    std::process::exit(0);
}

