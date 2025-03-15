use aws_sdk_ec2::model::SecurityGroupRule;
use config::Config;
use log::{error, info, warn};
use std::collections::HashMap;
use std::net::IpAddr;

const ENV_PREFIX: &str = "RUSTAPP";
const KEY: &str = "AWSSECGROUP";

async fn remove_rule(client: &aws_sdk_ec2::Client, group_id: &str, security_group_rule_id: &str) {
    let builder = client.revoke_security_group_ingress();
    let res = builder
        .security_group_rule_ids(security_group_rule_id)
        .group_id(group_id)
        .send()
        .await;
    match res {
        Ok(_) => info!("Removed Ingress rule for {security_group_rule_id:}"),
        Err(e) => error!("Removed Ingress rule failed {e:?}"),
    }
}

async fn wipe_ips(client: &aws_sdk_ec2::Client) {
    let builder = client.describe_security_group_rules();
    let res: Vec<SecurityGroupRule> = builder.send().await.unwrap().security_group_rules.unwrap();
    for row in res.iter() {
        let rule_ip = row.cidr_ipv4();
        match rule_ip {
            Some(_) => {
                if !row.is_egress.unwrap() {
                    remove_rule(
                        &client,
                        row.group_id().unwrap(),
                        row.security_group_rule_id().unwrap(),
                    )
                    .await
                }
            }
            None => {
                info!("Skipped rule, no rule_ip");
            }
        }
    }
}

async fn add_ip(client: &aws_sdk_ec2::Client, own_ip: &str, port: i32, security_group_id: &str) {
    // let builder_rev = client.revoke_security_group_ingress();
    // let revock = builder_rev.cidr_ip(own_ip).set_gr
    let builder = client.authorize_security_group_ingress();
    let dummy = builder
        .cidr_ip(own_ip)
        .set_group_id(Some(security_group_id.to_string()))
        .set_ip_protocol(Some("tcp".to_string()))
        .set_from_port(Some(port))
        .set_to_port(Some(port));
    let result = dummy.send().await;
    match &result {
        Ok(output) => info!("Setting new rule, {output:?}"),
        Err(error) => {
            if error.to_string().contains("already exists") {
                warn!("Rule for {}:{} already set!", own_ip, port);
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
        let settings: HashMap<String, String> = Config::builder()
            .add_source(
                config::Environment::with_prefix(ENV_PREFIX)
                    .try_parsing(true)
                    .separator("_"),
            )
            .build()
            .unwrap()
            .try_deserialize::<HashMap<String, String>>()
            .unwrap();
        AppConfig {
            security_group_id: settings.get(&KEY.to_lowercase()).unwrap().to_string(),
            ports: vec![22i32, 3306i32, 5000i32],
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let conf = AppConfig::new();

    let public_ip: IpAddr = get_ip().await.unwrap();
    let security_group_id: String = conf.security_group_id;

    let shared_config = aws_config::load_from_env().await;
    let client: aws_sdk_ec2::Client = aws_sdk_ec2::Client::new(&shared_config);
    wipe_ips(&client).await;

    let ports = conf.ports;
    for port in ports {
        add_ip(
            &client,
            format!("{:}/32", public_ip.to_string()).as_str(),
            port,
            security_group_id.as_str(),
        )
        .await;
    }
    std::process::exit(0);
}
