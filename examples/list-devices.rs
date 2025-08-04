//! List devices with sender context type.

use ashpd::desktop::remote_desktop::{DeviceType, RemoteDesktop};
use futures::stream::StreamExt;
use reis::{ei, tokio::EiEventStream, PendingRequestResult};
use std::{collections::HashMap, os::unix::net::UnixStream, process};

#[derive(Default)]
struct SeatData {
    name: Option<String>,
    capabilities: HashMap<String, u64>,
    devices: Vec<ei::Device>,
    done: bool,
}

#[derive(Default)]
struct DeviceData {
    name: Option<String>,
    device_type: Option<ei::device::DeviceType>,
    interfaces: HashMap<String, reis::Object>,
    done: bool,
}

impl DeviceData {
    #[allow(dead_code)]
    fn interface<T: reis::Interface>(&self) -> Option<T> {
        self.interfaces.get(T::NAME)?.clone().downcast()
    }
}

struct State {
    context: ei::Context,
    connection: ei::Connection,
    // XXX best way to handle data associated with object?
    seats: HashMap<ei::Seat, SeatData>,
    // XXX association with seat?
    devices: HashMap<ei::Device, DeviceData>,
}

impl State {
    fn handle_event(&mut self, event: ei::Event) {
        match event {
            ei::Event::Handshake(_handshake, _request) => panic!(),
            ei::Event::Connection(_connection, request) => match request {
                ei::connection::Event::Seat { seat } => {
                    self.seats.insert(seat, SeatData::default());
                }
                ei::connection::Event::Ping { ping } => {
                    ping.done(0);
                }
                _ => {}
            },
            ei::Event::Seat(seat, request) => {
                let data = self.seats.get_mut(&seat).unwrap();
                match request {
                    ei::seat::Event::Name { name } => {
                        data.name = Some(name);
                    }
                    ei::seat::Event::Capability { mask, interface } => {
                        data.capabilities.insert(interface, mask);
                    }
                    ei::seat::Event::Done => {
                        let caps = data.capabilities.values().fold(0, |a, b| a | b);
                        seat.bind(caps);
                        data.done = true;
                        self.connection.sync(1);

                        println!("Seat");
                        println!("    Name: {:?}", data.name);
                        println!(
                            "    Capabiltities: {:?}",
                            data.capabilities.keys().collect::<Vec<_>>()
                        );
                    }
                    ei::seat::Event::Device { device } => {
                        data.devices.push(device.clone());
                        self.devices.insert(device, DeviceData::default());
                    }
                    _ => {}
                }
            }
            ei::Event::Device(device, request) => {
                let data = self.devices.get_mut(&device).unwrap();
                match request {
                    ei::device::Event::Name { name } => {
                        data.name = Some(name);
                    }
                    ei::device::Event::DeviceType { device_type } => {
                        data.device_type = Some(device_type);
                    }
                    ei::device::Event::Interface { object } => {
                        data.interfaces
                            .insert(object.interface().to_owned(), object);
                    }
                    ei::device::Event::Done => {
                        data.done = true;
                        //self.print_and_exit_if_done();
                        println!("Device");
                        println!("    Name: {:?}", data.name);
                        println!("    Type: {:?}", data.device_type);
                        println!(
                            "    Interfaces: {:?}",
                            data.interfaces.keys().collect::<Vec<_>>()
                        );
                    }
                    _ => {}
                }
            }
            ei::Event::Callback(_callback, ei::callback::Event::Done { .. }) => {
                // TODO: Callback being called after first device, but not later ones?
                // self.print_and_exit_if_done();
            }
            _ => {}
        }

        let _ = self.context.flush();
    }

    #[allow(dead_code)]
    fn print_and_exit_if_done(&self) {
        if !(self.seats.values().all(|x| x.done) && self.devices.values().all(|x| x.done)) {
            return;
        }
        process::exit(0);
    }
}

async fn open_connection() -> ei::Context {
    if let Some(context) = ei::Context::connect_to_env().unwrap() {
        context
    } else {
        eprintln!("Unable to find ei socket. Trying xdg desktop portal.");
        let remote_desktop = RemoteDesktop::new().await.unwrap();
        let session = remote_desktop.create_session().await.unwrap();
        remote_desktop
            .select_devices(
                &session,
                DeviceType::Keyboard | DeviceType::Pointer | DeviceType::Touchscreen,
                None,
                ashpd::desktop::PersistMode::DoNot,
            )
            .await
            .unwrap();
        remote_desktop.start(&session, None).await.unwrap();
        let fd = remote_desktop.connect_to_eis(&session).await.unwrap();
        let stream = UnixStream::from(fd);
        ei::Context::new(stream).unwrap()
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let context = open_connection().await;

    let mut events = EiEventStream::new(context.clone()).unwrap();
    let handshake_resp = reis::tokio::ei_handshake(
        &mut events,
        "list-devices-example",
        ei::handshake::ContextType::Sender,
    )
    .await
    .unwrap();

    let mut state = State {
        context: context.clone(),
        connection: handshake_resp.connection,
        seats: HashMap::new(),
        devices: HashMap::new(),
    };

    while let Some(result) = events.next().await {
        let event = match result.unwrap() {
            PendingRequestResult::Request(event) => event,
            PendingRequestResult::ParseError(_msg) => {
                todo!()
            }
            PendingRequestResult::InvalidObject(_object_id) => {
                // TODO
                continue;
            }
        };

        state.handle_event(event);
    }
}
