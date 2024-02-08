use obirt::server::entities::MemPeers;

use clap::{Arg, Command};
use rsa::pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey};

use obirt::client;
use obirt::server::{auther, router};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .parse_env("LOG")
        .init();

    let matches = Command::new("Obirt")
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .subcommand(
            Command::new("client")
                .about("Connect to a server")
                .arg(
                    Arg::new("address")
                        .help("Server address")
                        .value_name("ADDRESS")
                        .index(1)
                        .required(true),
                )
                .arg(
                    Arg::new("auth-port")
                        .long("auth-port")
                        .help("Auth server port")
                        .value_name("AUTH_PORT")
                        .default_value("1120"),
                )
                .arg(
                    Arg::new("router-port")
                        .long("router-port")
                        .help("Router server port")
                        .value_name("ROUTER_PORT")
                        .default_value("9807"),
                )
                .arg(
                    Arg::new("interface")
                        .long("interface")
                        .short('i')
                        .help("Interface name")
                        .value_name("INTERFACE")
                        .default_value("obr0"),
                ),
        )
        .subcommand(Command::new("server").about("Start a server"))
        .subcommand(
            Command::new("keypair").about("Generate a new keypair").arg(
                Arg::new("force")
                    .long("force")
                    .short('f')
                    .help("Force the generation of new the keypair")
                    .value_name("FORCE")
                    .value_parser(clap::value_parser!(bool))
                    .default_value("false"),
            ),
        )
        .subcommand(Command::new("info").about("Show information about the connection"))
        .get_matches();

    match matches.subcommand() {
        Some(("client", command)) => {
            let server = command.get_one::<String>("address").unwrap();
            let auth_port = command.get_one::<String>("auth-port").unwrap();
            let router_port = command.get_one::<String>("router-port").unwrap();
            let interface = command.get_one::<String>("interface").unwrap();

            client::connect(server, auth_port, router_port, interface).await;
        }
        Some(("server", _)) => {
            let peers = MemPeers::default();

            tokio::join!(auther::start(&peers), router::start(&peers));
        }
        Some(("keypair", command)) => {
            let force = command.get_one::<bool>("force").unwrap();

            if keypair(*force).await {
                println!("Generated keypair");
            } else {
                println!("Keypair already exists");
            }
        }
        _ => {
            println!("No subcommand was used");
        }
    }
}

pub async fn keypair(force: bool) -> bool {
    if !force {
        if std::path::Path::new("./private.txt").exists() {
            println!("Private key already exists");
            return false;
        }

        if std::path::Path::new("./public.txt").exists() {
            println!("Public key already exists");
            return false;
        }
    }

    let private = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048).unwrap();
    private
        .write_pkcs1_pem_file("./private.txt", rsa::pkcs8::LineEnding::LF)
        .unwrap();
    let public = private.to_public_key();
    public
        .write_pkcs1_pem_file("./public.txt", rsa::pkcs8::LineEnding::LF)
        .unwrap();

    return true;
}
