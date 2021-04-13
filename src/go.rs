use std::time::Duration;

use crossbeam_channel::Sender;
use gilrs::Gilrs;

use crate::{commands::{Command, WriteCommand}, controller_manager::ControllerManager};

pub fn go(sender: Sender<Box<dyn Command>>) {
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
        loop {
            ControllerManager::prepare(&mut gilrs);
            let gamepad = gilrs.gamepad(id);

            let write_command =
                WriteCommand::new(1234, 6, 0, 1, controller_manager.poll(&gamepad));

            //if write_command.byte_data() != last_command.byte_data() {
                //println!("{:X?}", write_command.byte_data());
                match sender.send(Box::new(write_command)) {
                    Err(e) => println!("Unable to send data to thread: {}", e),
                    Ok(_) => {}
                }
            //}
    
            std::thread::sleep(loop_sleep_duration);
        }
    }
}