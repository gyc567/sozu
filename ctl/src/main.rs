#[macro_use] extern crate clap;
extern crate mio_uds;
extern crate rand;
extern crate sozu_command_lib as sozu_command;

mod command;

use mio_uds::UnixStream;
use clap::{App,Arg,SubCommand};

use sozu_command::config::Config;
use sozu_command::channel::Channel;
use sozu_command::data::{ConfigMessage,ConfigMessageAnswer};

use command::{add_application,remove_application,dump_state,load_state,save_state,soft_stop,hard_stop,upgrade,status,metrics,
  remove_backend, add_backend, remove_frontend, add_frontend, add_certificate, remove_certificate, query_application,
  logging_filter};

use std::str::FromStr;

fn main() {
  let matches = App::new("sozuctl")
                        .version(crate_version!())
                        .about("hot reconfigurable proxy")
                        .arg(Arg::with_name("config")
                            .short("c")
                            .long("config")
                            .value_name("FILE")
                            .help("Sets a custom config file")
                            .takes_value(true)
                            .required(option_env!("SOZU_CONFIG").is_none()))
                        .subcommand(SubCommand::with_name("shutdown")
                                    .about("shuts down the proxy")
                                    .arg(Arg::with_name("hard").long("hard")
                                         .help("shuts down the proxy without waiting for connections to finish")))
                        .subcommand(SubCommand::with_name("upgrade")
                                    .about("upgrades the proxy"))
                        .subcommand(SubCommand::with_name("status")
                                    .about("gets information on the running workers"))
                        .subcommand(SubCommand::with_name("metrics")
                                    .about("gets statistics on the master and its workers"))
                        .subcommand(SubCommand::with_name("state")
                                    .about("state management")
                                    .subcommand(SubCommand::with_name("save")
                                                .arg(Arg::with_name("file")
                                                    .short("f")
                                                    .long("file")
                                                    .value_name("state file")
                                                    .help("Save state to that file")
                                                    .takes_value(true)
                                                    .required(true)))
                                    .subcommand(SubCommand::with_name("load")
                                                .arg(Arg::with_name("file")
                                                    .short("f")
                                                    .long("file")
                                                    .value_name("state file")
                                                    .help("Save state to that file")
                                                    .takes_value(true)))
                                    .subcommand(SubCommand::with_name("dump")))
                        .subcommand(SubCommand::with_name("application")
                                                .about("application management")
                                                .subcommand(SubCommand::with_name("remove")
                                                  .arg(Arg::with_name("id")
                                                      .short("i")
                                                      .long("id")
                                                      .value_name("application identifier")
                                                      .takes_value(true)
                                                      .required(true)))
                                                .subcommand(SubCommand::with_name("add")
                                                  .arg(Arg::with_name("id")
                                                      .short("i")
                                                      .long("id")
                                                      .value_name("application identifier")
                                                      .takes_value(true)
                                                      .required(true))
                                                  .arg(Arg::with_name("sticky_session")
                                                      .long("sticky session")
                                                      .value_name("the frontend should do sticky session")
                                                      .takes_value(true)
                                                      .required(false))))
                        .subcommand(SubCommand::with_name("backend")
                                                .about("backend management")
                                                .subcommand(SubCommand::with_name("remove")
                                                  .arg(Arg::with_name("id")
                                                      .short("i")
                                                      .long("id")
                                                      .value_name("app id of the backend")
                                                      .takes_value(true)
                                                      .required(true))
                                                  .arg(Arg::with_name("instance-id")
                                                      .long("instance-id")
                                                      .value_name("id of the backend")
                                                      .takes_value(true)
                                                      .required(true))
                                                  .arg(Arg::with_name("ip")
                                                      .long("ip")
                                                      .value_name("ip of the backend")
                                                      .takes_value(true)
                                                      .required(true))
                                                  .arg(Arg::with_name("port")
                                                      .long("port")
                                                      .short("p")
                                                      .value_name("port of the backend")
                                                      .takes_value(true)
                                                      .required(true)))
                                                .subcommand(SubCommand::with_name("add")
                                                  .arg(Arg::with_name("id")
                                                      .short("i")
                                                      .long("id")
                                                      .value_name("app id of the backend")
                                                      .takes_value(true)
                                                      .required(true))
                                                  .arg(Arg::with_name("instance-id")
                                                      .long("instance-id")
                                                      .value_name("id of the backend")
                                                      .takes_value(true)
                                                      .required(true))
                                                  .arg(Arg::with_name("ip")
                                                      .long("ip")
                                                      .value_name("ip of the backend")
                                                      .takes_value(true)
                                                      .required(true))
                                                  .arg(Arg::with_name("port")
                                                      .long("port")
                                                      .short("p")
                                                      .value_name("port of the backend")
                                                      .takes_value(true)
                                                      .required(true))))
                        .subcommand(SubCommand::with_name("frontend")
                                                .about("frontend management")
                                                .subcommand(SubCommand::with_name("add")
                                                  .arg(Arg::with_name("id")
                                                      .short("i")
                                                      .long("id")
                                                      .value_name("app id of the frontend")
                                                      .takes_value(true)
                                                      .required(true))
                                                  .arg(Arg::with_name("hostname")
                                                      .short("host")
                                                      .long("hostname")
                                                      .value_name("hostname of the frontend")
                                                      .takes_value(true)
                                                      .required(true))
                                                  .arg(Arg::with_name("path_begin")
                                                      .long("path_begin")
                                                      .value_name("URL prefix of the frontend")
                                                      .takes_value(true)
                                                      .required(false))
                                                  .arg(Arg::with_name("certificate")
                                                      .long("certificate")
                                                      .value_name("path to a certificate file")
                                                      .takes_value(true)
                                                      .required(false)))
                                                .subcommand(SubCommand::with_name("remove")
                                                  .arg(Arg::with_name("id")
                                                      .short("i")
                                                      .long("id")
                                                      .value_name("app id of the frontend")
                                                      .takes_value(true)
                                                      .required(true))
                                                  .arg(Arg::with_name("hostname")
                                                      .short("host")
                                                      .long("hostname")
                                                      .value_name("hostname of the frontend")
                                                      .takes_value(true)
                                                      .required(true))
                                                  .arg(Arg::with_name("path_begin")
                                                      .long("path_begin")
                                                      .value_name("URL prefix of the frontend")
                                                      .takes_value(true)
                                                      .required(false))
                                                  .arg(Arg::with_name("certificate")
                                                      .long("certificate")
                                                      .value_name("path to a certificate file")
                                                      .takes_value(true)
                                                      .required(false))))
                        .subcommand(SubCommand::with_name("certificate")
                                                .about("certificate management")
                                                .subcommand(SubCommand::with_name("add")
                                                  .arg(Arg::with_name("certificate")
                                                      .long("certificate")
                                                      .value_name("path to the certificate")
                                                      .takes_value(true)
                                                      .required(true))
                                                  .arg(Arg::with_name("certificate chain")
                                                      .short("chain")
                                                      .long("certificate-chain")
                                                      .value_name("path to the certificate chain")
                                                      .takes_value(true)
                                                      .required(true))
                                                  .arg(Arg::with_name("key")
                                                      .long("key")
                                                      .value_name("path to the key")
                                                      .takes_value(true)
                                                      .required(false)))
                                                .subcommand(SubCommand::with_name("remove")
                                                  .arg(Arg::with_name("certificate")
                                                      .long("certificate")
                                                      .value_name("path to the certificate")
                                                      .takes_value(true)
                                                      .required(true))))
                        .subcommand(SubCommand::with_name("query")
                                                .about("configuration state verification")
                                                .subcommand(SubCommand::with_name("applications")
                                                  .arg(Arg::with_name("id")
                                                      .short("i")
                                                      .long("id")
                                                      .value_name("application identifier")
                                                      .takes_value(true)
                                                      .required(false))))
                        .subcommand(SubCommand::with_name("logging")
                                                .about("change logging level")
                                                .arg(Arg::with_name("level")
                                                  .short("l")
                                                  .long("level")
                                                  .value_name("logging level")
                                                  .takes_value(true)
                                                  .required(true)))
                        .get_matches();
 
  let config_file = match matches.value_of("config"){
                      Some(config_file) => config_file,
                      None => option_env!("SOZU_CONFIG").expect("could not find `SOZU_CONFIG` env var at build"),
                    };

  let config = Config::load_from_path(config_file).expect("could not parse configuration file");
  let stream = UnixStream::connect(&config.command_socket).expect("could not connect to the command unix socket");
  let mut channel: Channel<ConfigMessage,ConfigMessageAnswer> = Channel::new(stream, 10000, 20000);
  channel.set_nonblocking(false);

  match matches.subcommand() {
    ("shutdown", Some(sub)) => {
      let hard_shutdown = sub.is_present("hard");
      if hard_shutdown {
        hard_stop(&mut channel);
      } else {
        soft_stop(&mut channel);
      }
    },
    ("upgrade", Some(_)) => {
      upgrade(&mut channel);
    },
    ("status", Some(_)) => {
      status(&mut channel);
    },
    ("metrics", Some(_)) => {
      metrics(&mut channel);
    },
    ("state", Some(sub))    => {
      match sub.subcommand() {
        ("save", Some(state_sub)) => {
          let file = state_sub.value_of("file").expect("missing target file");
          save_state(&mut channel, file);
        },
        ("load", Some(state_sub)) => {
          let file = state_sub.value_of("file").expect("missing target file");
          load_state(&mut channel, file);
        },
        ("dump", _) => {
          dump_state(&mut channel);
        },
        _                   => println!("unknown state management command")
      }
    },
    ("application", Some(sub)) => {
      match sub.subcommand() {
        ("remove", Some(app_sub)) => {
          let id = app_sub.value_of("id").expect("missing id");
          remove_application(&mut channel, id);
        }
        ("add", Some(app_sub)) => {
          let id = app_sub.value_of("id").expect("missing id");
          let sticky_session  = app_sub.value_of("sticky_session").and_then(|b| bool::from_str(b).ok()).unwrap_or(false);
          add_application(&mut channel, id, sticky_session);
        }
        _ => println!("unknown backend management command")
      }
    },
    ("backend", Some(sub)) => {
      match sub.subcommand() {
        ("remove", Some(backend_sub)) => {
          let id = backend_sub.value_of("id").expect("missing id");
          let instance_id = backend_sub.value_of("instance_id").expect("missing instance id");
          let ip = backend_sub.value_of("ip").expect("missing backend ip");
          let port: u16 = backend_sub.value_of("port").expect("mssing backend port").parse().unwrap();
          remove_backend(&mut channel, id, instance_id, ip, port);
        }
        ("add", Some(backend_sub)) => {
          let id = backend_sub.value_of("id").expect("missing id");
          let instance_id = backend_sub.value_of("instance_id").expect("missing instance id");
          let ip = backend_sub.value_of("ip").expect("missing backend ip");
          let port: u16 = backend_sub.value_of("port").expect("mssing backend port").parse().unwrap();
          add_backend(&mut channel, id, instance_id, ip, port);
        }
        _ => println!("unknown backend management command")
      }
    },
    ("frontend", Some(sub)) => {
      match sub.subcommand() {
        ("remove", Some(frontend_sub)) => {
          let id              = frontend_sub.value_of("id").expect("missing id");
          let hostname        = frontend_sub.value_of("hostname").expect("missing frontend hostname");
          let path_begin      = frontend_sub.value_of("path_begin").unwrap_or("");
          let certificate     = frontend_sub.value_of("certificate");
          remove_frontend(&mut channel, id, hostname, path_begin, certificate);
        },
        ("add", Some(frontend_sub)) => {
          let id              = frontend_sub.value_of("id").expect("missing id");
          let hostname        = frontend_sub.value_of("hostname").expect("missing frontend hostname");
          let path_begin      = frontend_sub.value_of("path_begin").unwrap_or("");
          let certificate     = frontend_sub.value_of("certificate");
          add_frontend(&mut channel, id, hostname, path_begin, certificate);
        }
        _ => println!("unknown backend management command")
      }
    },
    ("certificate", Some(sub)) => {
      match sub.subcommand() {
        ("add", Some(cert_sub)) => {
          let certificate = cert_sub.value_of("certificate").expect("missing certificate path");
          let chain       = cert_sub.value_of("certificate-chain").expect("missing certificate chain path");
          let key         = cert_sub.value_of("key").unwrap_or("missing key path");
          add_certificate(&mut channel, certificate, chain, key);
        }
        ("remove", Some(cert_sub)) => {
          let certificate = cert_sub.value_of("certificate").expect("missing certificate path");
          remove_certificate(&mut channel, certificate);
        },
        _ => println!("unknown backend management command")
      }
    },
    ("query", Some(sub)) => {
      match sub.subcommand() {
        ("applications", Some(frontend_sub)) => {
          let id              = frontend_sub.value_of("id");
          query_application(&mut channel, id);
        },
        _ => println!("unknown query command")
      }
    },
    ("logging", Some(sub)) => {
      let level = sub.value_of("level").expect("missing logging level");
      logging_filter(&mut channel, level);
    },
    _                => println!("unknown subcommand")
  }

}
