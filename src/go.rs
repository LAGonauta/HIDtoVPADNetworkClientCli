use std::{borrow::Borrow, sync::{Arc, atomic::{AtomicBool, Ordering}}, thread, time::{Duration, Instant}};
use std::num::NonZeroU32;
use flume::{Receiver, Sender};
use gilrs::{GamepadId, Gilrs, GilrsBuilder, ff::{BaseEffect, BaseEffectType, Effect, EffectBuilder}};
use crate::{commands::{AttachData, Rumble, WriteCommand}, controller_manager::ControllerManager, handle_factory::HandleFactory, network::Message};
use governor::{Jitter, Quota, RateLimiter, clock::{self, Clock, QuantaInstant}};

pub fn go(
    polling_rate: u32,
    sender: Sender<Message>,
    reconection_notifier: Receiver<()>,
    rumble_receiver: Receiver<Rumble>,
    should_shutdown: Arc<AtomicBool>
) {
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

    for controller in &mut controllers {
        if gilrs.gamepad(controller.id).is_ff_supported() {
            match EffectBuilder::new()
            // .add_effect(BaseEffect {
            //     kind: BaseEffectType::Strong { magnitude: 10_000 },
            //     ..Default::default()
            // })
            .add_effect(BaseEffect {
                kind: BaseEffectType::Weak { magnitude: 20_000 },
                ..Default::default()
            })
            .add_gamepad(&gilrs.gamepad(controller.id))
            .finish(&mut gilrs) {
                Ok(effect) => {
                    controller.effect = Some(effect);
                },
                Err(e) => println!("Unable to add rumble to {}. Error: {}", gilrs.gamepad(controller.id).name(), e)
            };
        }

    }

    let controller_manager = ControllerManager::new();
    let clock = clock::DefaultClock::default();
    let limiter = RateLimiter::direct_with_clock(
        Quota::per_second(NonZeroU32::new(polling_rate).unwrap()),
        &clock
    );
    let jitter = Jitter::up_to(Duration::from_millis((1000 / polling_rate).into()));
    loop {
        match limiter.check() {
            Ok(_) => {},
            Err(e) => {
                thread::sleep(jitter + e.wait_time_from(clock.now()));
            }
        }

        if should_shutdown.load(Ordering::Relaxed) {
            return;
        }

        if let Ok(rumble) = rumble_receiver.try_recv() {
            match rumble {
                Rumble::Start(handle) => {
                    if let Some(controller) = controllers.iter().find(|c| c.handle == handle) {
                        if let Some(effect) = &controller.effect {
                            let _ = effect.play();
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

        let mut commands = Vec::new();
        for controller in &controllers {
            // check if any controller changed and send attach/detach events.
            ControllerManager::prepare(&mut gilrs);

            let gamepad = gilrs.gamepad(controller.id);
            commands.push((controller, controller_manager.poll(&gamepad)));
        }

        if commands.len() > 0 {
            let write_command =
                WriteCommand::new(&commands, 1);
            match sender.send(Message::UdpData(Box::new(write_command))) {
                Err(e) => println!("Unable to send data to thread: {}", e),
                Ok(_) => {}
            }
        }
    }
}

pub struct Controller {
    pub id: GamepadId,
    pub handle: i32,
    pub device_slot: i16,
    pub pad_slot: i8,
    pub effect: Option<Effect>
}