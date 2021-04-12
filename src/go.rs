use std::{net::{TcpStream, UdpSocket}, rc::Rc, sync::Arc, time::Duration};

use gilrs::Gilrs;

use crate::{commands::{Command, WriteCommand}, controller_manager::ControllerManager};

pub fn go(stream: &mut UdpSocket) {
    let mut gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads
    let mut gamepad_id = None;
    for (id, gamepad) in gilrs.gamepads() {
        gamepad_id = Some(id);
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    if let Some(id) = gamepad_id {
        let loop_sleep_duration = Duration::from_millis(10);
        let controller_manager = ControllerManager::new();
        let mut last_command = WriteCommand::new(0, 0, 0 ,0, Vec::new());
        loop {
            ControllerManager::prepare(&mut gilrs);
            let gamepad = gilrs.gamepad(id);

            let write_command =
                WriteCommand::new(1234, 6, 0, 1, controller_manager.poll(&gamepad));

            if write_command.byte_data() != last_command.byte_data() {
                //println!("{:X?}", write_command.byte_data());
                match stream.send(write_command.byte_data()) {
                    Err(e) => println!("Unable to send data: {}", e),
                    Ok(_) => {}
                }
            }

            last_command = write_command;
    
            std::thread::sleep(loop_sleep_duration);
        }
    }
}