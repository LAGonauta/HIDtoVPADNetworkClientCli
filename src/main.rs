use std::{sync::{Arc, atomic::Ordering}};
use clap::{Arg, App};

use atomic::Atomic;
use models::ApplicationState;

use std::net::IpAddr;

mod go;
mod network;
mod commands;
mod controller_manager;
mod models;

fn main() {
    let matches =
        App::new("Command line HIDtoVPAD network client")
            .version("v1.0.0")
            .arg(Arg::with_name("polling-rate")
                .short("p")
                .long("polling-rate")
                .help("Sets a custom polling rate. Must be between 20 and 1000 Hz.")
                .default_value("250")
                .validator(|val| {
                    match val.parse::<u32>() {
                        Ok(val) => {
                            if val < 20 {
                                return Err("Polling rate must be larger than 20 Hz".to_owned());
                            }

                            if val > 1000 {
                                return Err("Polling rate must be lower than 1000 Hz".to_owned());
                            }

                            Ok(())
                        },
                        Err(e) => {
                            Err(format!("Unable to parse polling-rate: {}", e))
                        }
                    }
                })
                .takes_value(true))
            .arg(Arg::with_name("ip")
                .help("Sets the IP address to connect, for example 192.168.2.3")
                .validator(|val| {
                    match val.parse::<IpAddr>() {
                        Err(e) => Err(format!("Unable to parse IP address. Error: {}", e)),
                        Ok(_) => Ok(())
                    }
                })
                .required(true))
            .get_matches();

    let _timer = Timer::new(1);

    let addr: IpAddr = matches.value_of("ip").unwrap().parse::<IpAddr>().unwrap();
    let polling_rate: u32 = matches.value_of("polling-rate").unwrap().parse::<u32>().unwrap();

    let (tcp_command_sender, tcp_command_receiver) = flume::unbounded();
    let (udp_command_sender, udp_command_receiver) = flume::bounded(0);

    let (reconection_notifier_sender, reconection_notifier_receiver) = flume::unbounded(); // use BUS

    let (rumble_sender, rumble_receiver) = flume::bounded(0);

    let application_state = Arc::new(Atomic::new(ApplicationState::Disconnected));

    let network_thread = network::start_thread(
        addr,
        tcp_command_sender.clone(),
        tcp_command_receiver,
        udp_command_receiver.clone(),
        reconection_notifier_sender,
        rumble_sender,
        application_state.clone());

    let go_thread = std::thread::spawn({
        let application_state = application_state.clone();
        move || {
            go::go(polling_rate,
                tcp_command_sender,
                udp_command_sender,
                reconection_notifier_receiver,
                rumble_receiver,
                application_state
            );
        }
    });

    ctrlc::set_handler({
        let application_state = application_state.clone();
        move || {
            application_state.store(ApplicationState::Exiting, Ordering::Relaxed);
            println!("### Press enter to finish ###");
        }
    })
    .expect("Error setting Ctrl-C handler");

    println!("### Press enter to exit ###");
    let _ = std::io::stdin().read_line(&mut String::new());
    println!("---> Exiting <---");

    application_state.store(ApplicationState::Exiting, Ordering::Relaxed);
    let _ = network_thread.join();
    let _ = go_thread.join();
}

struct Timer {
    value: u32
}

impl Timer {
    pub fn new(value: u32) -> Self {
        let result = Timer {
            value
        };

        result.set_timer();
        result
    }

    #[cfg(windows)]
    fn set_timer(&self) {
        unsafe {
            winapi::um::timeapi::timeBeginPeriod(self.value);
        }
    }

    #[cfg(windows)]
    fn unset_timer(&self) {
        unsafe {
            winapi::um::timeapi::timeEndPeriod(self.value);
        }
    }

    #[cfg(not(windows))]
    fn set_timer(&self) { }

    #[cfg(not(windows))]
    fn unset_timer(&self) { }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.unset_timer();
    }
}