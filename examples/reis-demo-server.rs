//! Demo server.

use enumflags2::BitFlags;
use reis::{
    calloop::{EisListenerSource, EisRequestSource, EisRequestSourceEvent},
    eis::{self, device::DeviceType},
    request::{Connection, DeviceCapability, EisRequest},
};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[derive(Default)]
struct ContextState {
    seat: Option<reis::request::Seat>,
    device_keyboard: Option<reis::request::Device>,
    device_pointer: Option<reis::request::Device>,
    device_pointer_absolute: Option<reis::request::Device>,
    device_touch: Option<reis::request::Device>,
    sequence: u32,
}

impl ContextState {
    fn handle_request(
        &mut self,
        connection: &Connection,
        request: EisRequest,
    ) -> calloop::PostAction {
        match request {
            EisRequest::Disconnect => {
                return calloop::PostAction::Remove;
            }
            EisRequest::Bind(request) => {
                let capabilities = request.capabilities;

                if self.device_keyboard.is_none()
                    && capabilities.contains(DeviceCapability::Keyboard)
                {
                    self.device_keyboard = Some(add_device(
                        "keyboard",
                        DeviceCapability::Keyboard.into(),
                        |_| {},
                        &request.seat,
                        connection,
                        &mut self.sequence,
                    ));
                }

                if self.device_pointer.is_none() && capabilities.contains(DeviceCapability::Pointer)
                {
                    self.device_pointer = Some(add_device(
                        "pointer",
                        DeviceCapability::Pointer
                            | DeviceCapability::Button
                            | DeviceCapability::Scroll,
                        |_| {},
                        &request.seat,
                        connection,
                        &mut self.sequence,
                    ));
                }

                if self.device_touch.is_none() && capabilities.contains(DeviceCapability::Touch) {
                    self.device_touch = Some(add_device(
                        "touch",
                        DeviceCapability::Touch.into(),
                        |_| {},
                        &request.seat,
                        connection,
                        &mut self.sequence,
                    ));
                }

                if self.device_pointer_absolute.is_none()
                    && capabilities.contains(DeviceCapability::PointerAbsolute)
                {
                    self.device_pointer_absolute = Some(add_device(
                        "pointer-abs",
                        DeviceCapability::PointerAbsolute
                            | DeviceCapability::Button
                            | DeviceCapability::Scroll,
                        |_| {},
                        &request.seat,
                        connection,
                        &mut self.sequence,
                    ));
                }
            }
            _ => {}
        }

        calloop::PostAction::Continue
    }
}

fn add_device(
    name: &str,
    capabilities: BitFlags<DeviceCapability>,
    before_done_cb: impl for<'a> FnOnce(&'a reis::request::Device),
    seat: &reis::request::Seat,
    connection: &Connection,
    sequence: &mut u32,
) -> reis::request::Device {
    let device = seat.add_device(
        Some(name),
        DeviceType::Virtual,
        capabilities,
        before_done_cb,
    );
    device.resumed();
    if connection.context_type() == eis::handshake::ContextType::Receiver {
        *sequence += 1;
        device.start_emulating(*sequence);
    }
    device
}

struct State {
    handle: calloop::LoopHandle<'static, Self>,
}

impl State {
    #![allow(clippy::unnecessary_wraps)]
    fn handle_new_connection(&mut self, context: eis::Context) -> io::Result<calloop::PostAction> {
        println!("New connection: {context:?}");

        let source = EisRequestSource::new(context, 1);
        let mut context_state = ContextState::default();
        self.handle
            .insert_source(source, move |event, connected_state, _state| {
                Ok(match event {
                    Ok(event) => Self::handle_request_source_event(
                        &mut context_state,
                        connected_state,
                        event,
                    ),
                    Err(err) => {
                        eprintln!("Error communicating with client: {err}");
                        calloop::PostAction::Remove
                    }
                })
            })
            .unwrap();

        Ok(calloop::PostAction::Continue)
    }

    fn handle_request_source_event(
        context_state: &mut ContextState,
        connection: &Connection,
        event: EisRequestSourceEvent,
    ) -> calloop::PostAction {
        match event {
            EisRequestSourceEvent::Connected => {
                let seat = connection.add_seat(
                    Some("default"),
                    DeviceCapability::Pointer
                        | DeviceCapability::PointerAbsolute
                        | DeviceCapability::Keyboard
                        | DeviceCapability::Touch
                        | DeviceCapability::Scroll
                        | DeviceCapability::Button,
                );

                context_state.seat = Some(seat);
            }
            EisRequestSourceEvent::Request(request) => {
                let res = context_state.handle_request(connection, request);
                if res != calloop::PostAction::Continue {
                    return res;
                }
            }
        }

        let _ = connection.flush();

        calloop::PostAction::Continue
    }
}

fn main() {
    let mut event_loop = calloop::EventLoop::try_new().unwrap();
    let handle = event_loop.handle();

    let listener = eis::Listener::bind_auto().unwrap();
    let listener_source = EisListenerSource::new(listener);
    handle
        .insert_source(listener_source, |context, (), state: &mut State| {
            state.handle_new_connection(context)
        })
        .unwrap();

    let terminate = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, terminate.clone()).unwrap();

    let mut state = State { handle };
    while !terminate.load(Ordering::Relaxed) {
        event_loop
            .dispatch(Duration::from_millis(100), &mut state)
            .unwrap();
    }
}
