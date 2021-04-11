use std::{iter::FromIterator, slice::Iter, time::Duration};

use gilrs::{Gilrs, Button, Event, Axis};

pub fn go() {
    let mut gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads
    let mut gamepad_id = None;
    for (id, gamepad2) in gilrs.gamepads() {
        gamepad_id = Some(id);
        println!("{} is {:?}", gamepad2.name(), gamepad2.power_info());
    }
    
    loop {
        // Examine new events
        while let Some(Event { id: _, event: _, time: _ }) = gilrs.next_event() {
            //println!("{:?} New event from {}: {:?}", time, id, event);
            //active_gamepad = Some(id);
        }
    
        // You can also use cached gamepad state
        let mut buttons_state = 0;
        let mut stick_state = 0;
        if let Some(id) = gamepad_id {
            let gamepad = gilrs.gamepad(id);

            buttons_state = buttons_iterator()
                .filter(|&&button| gamepad.is_pressed(button))
                .map(|&button| map_button_state(button))
                .fold(0, |accumulated, element| accumulated | element);

            stick_state = axes_iterator()
                .map(|&axis| (axis, gamepad.axis_data(axis)))
                .filter(|axis| axis.1.is_some())
                .map(|axis| (axis.0, axis.1.unwrap()))
                .map(|axis| map_axis_data(axis.1.value(), axis.0))
                .fold(0, |accumulated, element| accumulated | element);
        }
        print!("Stick: {:#034b}. Buttons state: {:#08}.     \r", stick_state, buttons_state);

        std::thread::sleep(Duration::from_millis(1));
    }
}

fn map_button_state(button: Button) -> i32 {
    match button {
        Button::South => 1 << 0,
        Button::East => 1 << 1,
        Button::West => 1 << 2,
        Button::North => 1 << 3,

        Button::DPadLeft => 1 << 4,
        Button::DPadUp => 1 << 5,
        Button::DPadRight => 1 << 6,
        Button::DPadDown => 1 << 7,

        Button::Select => 1 << 8,
        Button::Start => 1 << 9,

        Button::C => 0,
        Button::Z => 0,

        Button::LeftTrigger => 1 << 10,
        Button::LeftTrigger2 => 0,

        Button::RightTrigger => 1 << 11,
        Button::RightTrigger2 => 0,
        
        Button::LeftThumb => 1 << 12,
        Button::RightThumb => 1 << 13,

        Button::Unknown => 1 << 14,

        Button::Mode => 1 << 15
    }
}

fn map_axis_data(value: f32, axis: Axis) -> i32 {
    let result = (((value * 128.0) + 128.0) as u8) as i32;

    match axis {
        Axis::LeftStickX => result << 24,
        Axis::LeftStickY => result << 16,
        Axis::LeftZ => 0,
        Axis::RightStickX => result << 8,
        Axis::RightStickY => result << 0,
        Axis::RightZ => 0,
        Axis::DPadX => 0,
        Axis::DPadY => 0,
        Axis::Unknown => 0
    }
}

fn buttons_iterator() -> Iter<'static, Button> {
    static BUTTONS: [Button; 19] =
        [
            Button::South, Button::East, Button::West, Button::North,
            Button::DPadLeft, Button::DPadUp, Button::DPadRight, Button::DPadDown,
            Button::Select, Button::Start, Button::C, Button::Z,
            Button::LeftTrigger, Button::LeftTrigger2, Button::RightTrigger, Button::RightTrigger2,
            Button::LeftThumb, Button::RightThumb,
            Button::Mode
        ];
    return BUTTONS.iter();
}

fn axes_iterator() -> Iter<'static, Axis> {
    static AXES: [Axis; 4] =
        [
            Axis::LeftStickX, Axis::LeftStickY,
            Axis::RightStickX, Axis::RightStickY
        ];
    return AXES.iter();
}