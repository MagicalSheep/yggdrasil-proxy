mod model;
mod proxy;
mod entity;
mod web;
mod repository;
mod utils;

use warp::Filter;
use std::io::Read;
use std::net::SocketAddr;
use chrono::Local;
use lazy_static::lazy_static;
use log::{error, info};
use pretty_env_logger::env_logger;
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::io::Write;
use std::process::exit;

use crate::model::{Config, Meta};
use crate::web::{filters, handlers};

static IMPLEMENTATION_NAME: &str = "Yggdrasil API Reverse Proxy By MagicalSheep";
static VERSION: &str = "0.1.0";

macro_rules! exit {
    ($err:expr) => {
        error!("{}", $err);
        exit(0);
    };
}

lazy_static! {
    static ref CONFIG: Config = load_config();
    static ref PRIVATE_KEY: RsaPrivateKey = load_private_key();
    static ref PUBLIC_KEY: String = load_public_key(); // just for meta display, so it is string type
}

fn pre_check() {
    let mut need_exit = false;
    let fp = "config.yaml";
    if !std::path::Path::new(fp).exists() {
        info!("No configuration file found, create it...");
        let config = Config::new();
        let config_yaml = match serde_yaml::to_string(&config) {
            Ok(res) => { res }
            Err(err) => { exit!(err); }
        };
        let mut file = match std::fs::File::create(fp) {
            Ok(f) => { f }
            Err(err) => { exit!(err); }
        };
        match file.write_all(config_yaml.as_bytes()) {
            Ok(_) => { info!("Create configuration file successfully"); }
            Err(err) => { exit!(err); }
        }
        need_exit = true;
    }
    let k_fp = "private_key.pem";
    if !std::path::Path::new(k_fp).exists() {
        info!("No private key file found, create it...");
        let mut rng = rand::thread_rng();
        let bits = 4096;
        info!("Generating a PEM-encoded PKCS#8 private key (4096 bits), please wait...");
        let private_key = match RsaPrivateKey::new(&mut rng, bits) {
            Ok(res) => { res }
            Err(err) => { exit!(err); }
        };
        let key = match private_key.to_pkcs8_pem(LineEnding::default()) {
            Ok(res) => { res }
            Err(err) => { exit!(err); }
        };
        info!("Generate private key successfully");
        let mut file = match std::fs::File::create(k_fp) {
            Ok(f) => { f }
            Err(err) => { exit!(err); }
        };
        match file.write_all(key.as_bytes()) {
            Ok(_) => { info!("Create private key file successfully"); }
            Err(err) => { exit!(err); }
        }
        need_exit = true;
    }
    if need_exit {
        info!("Please fill in your configuration file and then restart the proxy");
        exit(0);
    }
}

fn load_config() -> Config {
    let mut file = match std::fs::File::open("config.yaml") {
        Ok(res) => { res }
        Err(err) => { exit!(err); }
    };
    let mut yaml_str = String::new();
    if let Err(err) = file.read_to_string(&mut yaml_str) { exit!(err); }
    match serde_yaml::from_str(&yaml_str) {
        Ok(res) => { res }
        Err(err) => { exit!(err); }
    }
}

fn load_private_key() -> RsaPrivateKey {
    let mut file = match std::fs::File::open("private_key.pem") {
        Ok(res) => { res }
        Err(err) => { exit!(err); }
    };
    let mut private_key = String::new();
    if let Err(err) = file.read_to_string(&mut private_key) { exit!(err); }
    match RsaPrivateKey::from_pkcs8_pem(&private_key) {
        Ok(ret) => { ret }
        Err(err) => { exit!(err); }
    }
}

fn load_public_key() -> String {
    let private_key = &*PRIVATE_KEY;
    match RsaPublicKey::from(private_key).to_public_key_pem(LineEnding::default()) {
        Ok(res) => { res }
        Err(err) => { exit!(err); }
    }
}

fn init_log() {
    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
    env_logger::Builder::from_env(env)
        .format(|buf, record| {
            let level = { buf.default_styled_level(record.level()) };
            writeln!(
                buf,
                "{} {} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                format_args!("{:>5}", level),
                record.module_path().unwrap_or("<unnamed>"),
                &record.args()
            )
        })
        .init();
}

#[tokio::main]
async fn main() {
    init_log();
    pre_check();
    load_config();
    load_private_key();
    load_public_key();

    if let Err(err) = repository::init(&CONFIG.data_source).await { exit!(err); }

    let log = warp::log::custom(|info| {
        info!(
            "{} {} {}",
            info.method(),
            info.path(),
            info.status(),
        );
    });
    let routes = filters::authenticate()
        .or(filters::fresh())
        .or(filters::validate())
        .or(filters::invalidate())
        .or(filters::logout())
        .or(filters::join())
        .or(filters::has_join())
        .or(filters::profile())
        .or(filters::profiles())
        .or(filters::meta())
        .or(filters::certificates())
        .with(log)
        .recover(handlers::err_handle);

    let addr = SocketAddr::new(
        CONFIG.address.parse().expect("Parse address failed"),
        CONFIG.port,
    );
    warp::serve(routes).run(addr).await;
}
