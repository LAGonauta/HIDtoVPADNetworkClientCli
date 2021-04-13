use std::time::Duration;

use crossbeam_channel::Sender;
use gilrs::Gilrs;

use crate::{commands::WriteCommand, controller_manager::ControllerManager, network::Message};

pub fn go(sender: Sender<Message>) {
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

            match sender.send(Message::UdpData(Box::new(write_command))) {
                Err(e) => println!("Unable to send data to thread: {}", e),
                Ok(_) => {}
            }
    
            std::thread::sleep(loop_sleep_duration);
        }
    }
}