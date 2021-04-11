use std::{rc::Rc, sync::Arc, time::Duration};

use gilrs::Gilrs;

use crate::controller_manager::ControllerManager;

pub fn go() {
    let mut gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads
    let mut gamepad_id = None;
    for (id, gamepad) in gilrs.gamepads() {
        gamepad_id = Some(id);
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    if let Some(id) = gamepad_id {
        let loop_sleep_duration = Duration::from_millis(1);
        let controller_manager = ControllerManager::new();
        loop {
            ControllerManager::prepare(&mut gilrs);
            let gamepad = gilrs.gamepad(id);

            let data = controller_manager.poll(&gamepad);
    
            std::thread::sleep(loop_sleep_duration);
        }
    }
}