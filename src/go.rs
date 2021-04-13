use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, thread, time::Duration};

use flume::{Receiver, Sender};
use gilrs::{GamepadId, Gilrs};

use crate::{commands::WriteCommand, controller_manager::ControllerManager, handle_factory::HandleFactory, network::Message};

pub fn go(sender: Sender<Message>, controller_receiver: Receiver<(i32, i16, i8)>) {
    let mut gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads and attach them
    let mut handle_factory = HandleFactory::new();
    let mut controllers = Vec::new();
    let (s, r) = flume::bounded(0);
    for (id, gamepad) in gilrs.gamepads() {
        let handle = handle_factory.next();
        match sender.send(Message::Attach((handle, s.clone()))) {
            Ok(_) => {
                match r.recv() {
                    Ok(val) => {
                        if val.0 < 0 || val.1 < 0 {
                            println!("Unable to attach controller, invalid slots.")
                        } else {
                            controllers.push(Controller { id: id, handle: handle, device_slot: val.0, pad_slot: val.1 });
                            println!("{} is {:?}. Attached!", gamepad.name(), gamepad.power_info());
                        }
                    }
                    Err(e) => println!("Unable to attach controller, error on receive.")
                };
            },
            Err(e) => println!("Unable to attach controller, error on send.")
        }

    }

    let controller_manager = ControllerManager::new();
    let loop_sleep_duration = Duration::from_millis(10);
    loop {
        while let Ok(val) = controller_receiver.try_recv() {
            if let Some(controller) = controllers.iter_mut().find(|e| e.handle == val.0) {
                controller.pad_slot = val.2;
                controller.device_slot = val.1;
            }
        }

        for controller in &controllers {
            ControllerManager::prepare(&mut gilrs);
            let gamepad = gilrs.gamepad(controller.id);

            let write_command =
                WriteCommand::new(controller.handle, controller.device_slot, controller.pad_slot, 1, controller_manager.poll(&gamepad));

            match sender.send(Message::UdpData(Box::new(write_command))) {
                Err(e) => println!("Unable to send data to thread: {}", e),
                Ok(_) => {}
            }
        }

        thread::sleep(loop_sleep_duration);
    }
}

struct Controller {
    pub id: GamepadId,
    pub handle: i32,
    pub device_slot: i16,
    pub pad_slot: i8
}