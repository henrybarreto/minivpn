use clap::{Arg, Command};

fn main() {
    env_logger::init();

    let matches = Command::new("Orbit")
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .subcommand(
            Command::new("connect")
                .about("Connect to a server")
                .arg(
                    Arg::new("address")
                        .help("Server address")
                        .value_name("ADDRESS")
                        .index(1)
                        .required(true),
                )
                .arg(
                    Arg::new("port")
                        .long("port")
                        .short('p')
                        .help("Server port")
                        .value_name("PORT")
                        .default_value("9807"),
                )
                .arg(
                    Arg::new("interface")
                        .long("interface")
                        .short('i')
                        .help("Interface name")
                        .value_name("INTERFACE")
                        .default_value("orb0"),
                ),
        )
        // info
        .subcommand(Command::new("info").about("Show information about the connection"))
        .get_matches();

    // Retrieve values of address and port arguments
    // let address = matches.value_source("address").unwrap();
    // let port = matches.value_source("port").unwrap();

    // Print the server information
    // println!("Server Address: {:?}", address);
    // println!("Server Port: {:?}", port);
}
