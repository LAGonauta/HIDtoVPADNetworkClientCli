use std::{sync::Arc, thread, time::Duration};
use std::num::NonZeroU32;
use flume::{Receiver, Sender};
use gilrs::{GamepadId, Gilrs, ff::{BaseEffect, BaseEffectType, Effect, EffectBuilder}};
use crate::{commands::WriteCommand, controller_manager::ControllerManager, models::{ApplicationState, AttachData, DetachData, Controller, Rumble, TcpMessage, UdpMessage}};
use governor::{Quota, RateLimiter, clock::{self, Clock}};
use atomic::{Atomic, Ordering};

fn gamepad_id_to_handle(gamepad_id: GamepadId) -> i32 {
    let raw_id: usize = gamepad_id.into();
    let max = (i32::MAX - 1) as usize;
    ((raw_id % max) as i32) + 1
}

fn attach(gamepad_id: GamepadId, tcp_sender: &Sender<TcpMessage>) -> Option<Controller> {
    let handle = gamepad_id_to_handle(gamepad_id);
    let (s, r) = flume::bounded(0);
    match tcp_sender.send(TcpMessage::Attach(AttachData { handle, response: s })) {
        Ok(_) => {
            match r.recv_timeout(Duration::from_secs(10)) {
                Ok(val) => {
                    match val {
                        Some(val) => {
                            if val.device_slot < 0 || val.pad_slot < 0 {
                                println!("Unable to attach controller, invalid slots.")
                            } else {
                                return Some(Controller { id: gamepad_id, handle: handle, device_slot: val.device_slot, pad_slot: val.pad_slot, effect: None });
                            }
                        },
                        None => println!("Unable to attach controller, no response received.")
                    }
                }
                Err(e) => println!("Unable to attach controller, error on receive: {}", e)
            };
        },
        Err(e) => println!("Unable to attach controller, error on send: {}", e)
    };

    None
}

fn dettach_gamepad(gamepad_id: GamepadId, tcp_sender: &Sender<TcpMessage>) {
    let handle = gamepad_id_to_handle(gamepad_id);
    match tcp_sender.send(TcpMessage::Detach(DetachData { handle })) {
        Ok(_) => {},
        Err(e) => println!("Unable to dettach controller: {}", e)
    };
}

fn create_effect(id: GamepadId, gilrs: &mut Gilrs) -> Option<Effect> {
    if !gilrs.gamepad(id).is_ff_supported() {
        return None;
    }

    match EffectBuilder::new()
        // .add_effect(BaseEffect {
        //     kind: BaseEffectType::Strong { magnitude: 10_000 },
        //     ..Default::default()
        // })
        .add_effect(BaseEffect {
            kind: BaseEffectType::Weak { magnitude: 20_000 },
            ..Default::default()
        })
        .add_gamepad(&gilrs.gamepad(id))
        .finish(gilrs) {
        Ok(effect) => {
            Some(effect)
        },
        Err(e) => {
            println!("Unable to add rumble to {}. Error: {}", gilrs.gamepad(id).name(), e);
            None
        }
    }
}

fn attach_gamepad(gamepad_id: GamepadId, tcp_sender: &Sender<TcpMessage>, gilrs: &mut Gilrs) -> Option<Controller> {
    let gamepad = gilrs.gamepad(gamepad_id);
    match attach(gamepad_id, &tcp_sender) {
        Some(controller) => {
            println!("{} is {:?}. Attached!", gamepad.name(), gamepad.power_info());
            Some(controller)
        },
        None => {
            println!("{} is {:?}. Unable to attach...", gamepad.name(), gamepad.power_info());
            None
        }
    }
}

pub fn go(
    polling_rate: u32,
    tcp_sender: Sender<TcpMessage>,
    udp_sender: Sender<UdpMessage>,
    reconection_notifier: Receiver<()>,
    rumble_receiver: Receiver<Rumble>,
    application_state: Arc<Atomic<ApplicationState>>
) {

    while application_state.load(Ordering::Relaxed).is_disconnected() {
        thread::sleep(Duration::from_secs(1));
    }

    if application_state.load(Ordering::Relaxed).is_exiting() {
        return;
    }

    let mut gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads and attach them
    let mut controllers = Vec::new();

    for gamepad_id in gilrs.gamepads().map(|(gamepad_id, _)| gamepad_id).collect::<Vec<GamepadId>>() {
        if let Some(mut controller) = attach_gamepad(gamepad_id, &tcp_sender, &mut gilrs) {
            controller.effect = create_effect(gamepad_id, &mut gilrs);
            controllers.push(controller);
        }
    }

    let controller_manager = ControllerManager::new();
    let clock = clock::DefaultClock::default();
    let limiter = RateLimiter::direct_with_clock(
        // for some reason the synchronous solution requires to divide the polling rate. Might be interesting to use async-await.
        Quota::per_second(NonZeroU32::new(polling_rate / 2).unwrap()).allow_burst(NonZeroU32::new(1u32).unwrap()),
        &clock
    );

    // let mut i = 0;
    // let mut last = Instant::now();
    // let interval = Duration::from_secs(1);
    let send_timeout = Duration::from_secs(1);
    loop {
        // {
        //     i = i + 1;
        //     let now = Instant::now();
        //     if now > last + interval {
        //         println!("Count: {}", i);
        //         i = 0;
        //         last = now;
        //     }
        // }

        match limiter.check() {
            Ok(_) => {},
            Err(e) => {
                thread::sleep(e.wait_time_from(clock.now()));
            }
        }

        match application_state.load(Ordering::Relaxed) {
            ApplicationState::Disconnected => {
                thread::sleep(Duration::from_secs(1));
                continue;
            },
            ApplicationState::Exiting => return,
            ApplicationState::Connected => {}
        }

        if let Ok(rumble) = rumble_receiver.try_recv() {
            match rumble {
                Rumble::Start(handle) => {
                    if let Some(controller) = controllers.iter().find(|c| c.handle == handle) {
                        if let Some(effect) = &controller.effect {
                            let _ = effect.play(); // play only if it is playing? also check for how long the effect runs
                        }
                    }
                }
                Rumble::Stop(handle) => {
                    if let Some(controller) = controllers.iter().find(|c| c.handle == handle) {
                        if let Some(effect) = &controller.effect {
                            let _ = effect.stop();
                        }
                    }
                }
            };
        }

        if reconection_notifier.try_recv().is_ok() {
            for controller in &mut controllers {
                match attach(controller.id, &tcp_sender) {
                    Some(new_data) => {
                        controller.pad_slot = new_data.pad_slot;
                        controller.device_slot = new_data.device_slot;
                        //println!("{} is {:?}. Attached!", gamepad.name(), gamepad.power_info());
                    },
                    None => {}//println!("{} is {:?}. Unable to attach...", gamepad.name(), gamepad.power_info())
                }
            }
        }

        while let Some(event) = gilrs.next_event() {
            match event.event {
                gilrs::EventType::Connected => {
                    if !controllers.iter().any(|controller| controller.id == event.id) {
                        println!("Attaching {}", gilrs.gamepad(event.id).name());
                        if let Some(mut controller) = attach_gamepad(event.id, &tcp_sender, &mut gilrs) {
                            controller.effect = create_effect(event.id, &mut gilrs);
                            controllers.push(controller);
                        }
                    }
                },
                gilrs::EventType::Disconnected => {
                    if controllers.iter().any(|controller| controller.id == event.id) {
                        println!("Dettaching {}", gilrs.gamepad(event.id).name());
                        dettach_gamepad(event.id, &tcp_sender);
                        controllers.retain(|c| c.id != event.id);
                    }
                },
                _ => {}
            }
        }
        
        let mut commands = Vec::new();
        for controller in &controllers {
            let gamepad = gilrs.gamepad(controller.id);
            commands.push((controller, controller_manager.poll(&gamepad)));
        }

        if commands.len() > 0 {
            let write_command =
                WriteCommand::new(&commands, 1);
            match udp_sender.send_timeout(UdpMessage::UdpData(Box::new(write_command)), send_timeout) { // add timeout?
                Err(e) => println!("Unable to send data to thread: {}", e),
                Ok(_) => {}
            }
        }
    }
}
