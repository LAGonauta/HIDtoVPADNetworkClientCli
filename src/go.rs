use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, thread, time::Duration};

use flume::{Receiver, Sender};
use gilrs::{GamepadId, Gilrs};

use crate::{commands::{WriteCommand, AttachData}, controller_manager::ControllerManager, handle_factory::HandleFactory, network::Message};

pub fn go(sender: Sender<Message>, reconection_notifier: Receiver<()>, should_shutdown: Arc<AtomicBool>) {
    let mut gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads and attach them
    let mut handle_factory = HandleFactory::new();
    let mut controllers = Vec::new();
    let (s, r) = flume::bounded(0);
    let attach = |handle: i32, gamepad_id: GamepadId| -> Option<Controller> {
        match sender.send(Message::Attach(AttachData { handle, response: s.clone() })) {
            Ok(_) => {
                match r.recv() {
                    Ok(val) => {
                        match val {
                            Some(val) => {
                                if val.device_slot < 0 || val.pad_slot < 0 {
                                    println!("Unable to attach controller, invalid slots.")
                                } else {
                                    return Some(Controller { id: gamepad_id, handle: handle, device_slot: val.device_slot, pad_slot: val.pad_slot });
                                }
                            },
                            None => println!("Unable to attach controller, no response received.")
                        }
                    }
                    Err(e) => println!("Unable to attach controller, error on receive.")
                };
            },
            Err(e) => println!("Unable to attach controller, error on send.")
        };
        return None;
    };

    for (id, gamepad) in gilrs.gamepads() {
        let handle = handle_factory.next();
        match attach(handle, id) {
            Some(controller) => {
                println!("{} is {:?}. Attached!", gamepad.name(), gamepad.power_info());
                controllers.push(controller);
            },
            None => println!("{} is {:?}. Unable to attach...", gamepad.name(), gamepad.power_info())
        }
    }

    let controller_manager = ControllerManager::new();
    let loop_sleep_duration = Duration::from_millis(10);
    loop {
        if should_shutdown.load(Ordering::Relaxed) {
            return;
        }

        if reconection_notifier.try_recv().is_ok() {
            for controller in &mut controllers {
                match attach(controller.handle, controller.id) {
                    Some(new_data) => {
                        controller.pad_slot = new_data.pad_slot;
                        controller.device_slot = new_data.device_slot;
                        //println!("{} is {:?}. Attached!", gamepad.name(), gamepad.power_info());
                    },
                    None => {}//println!("{} is {:?}. Unable to attach...", gamepad.name(), gamepad.power_info())
                }
            }
        }

        for controller in &controllers {
            ControllerManager::prepare(&mut gilrs);
            // check if any controller changed and send attach/detach events. The network manager should ask again for all controllers on reconnection
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