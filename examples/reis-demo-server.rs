// TODO: Require context_type

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

struct ContextState {
    seat: Option<reis::request::Seat>,
}

impl ContextState {
    fn disconnected(
        &self,
        connection: &Connection,
        reason: eis::connection::DisconnectReason,
        explaination: &str,
    ) -> calloop::PostAction {
        connection.disconnected(reason, explaination);
        connection.flush();
        calloop::PostAction::Remove
    }

    fn protocol_error(&self, connection: &Connection, explanation: &str) -> calloop::PostAction {
        self.disconnected(
            connection,
            eis::connection::DisconnectReason::Protocol,
            explanation,
        )
    }

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

                // TODO Handle in converter
                if capabilities & 0x7e != capabilities {
                    return self.disconnected(
                        connection,
                        eis::connection::DisconnectReason::Value,
                        "Invalid capabilities",
                    );
                }

                let seat = self.seat.as_ref().unwrap();

                if connection.has_interface("ei_keyboard")
                    && capabilities & 2 << DeviceCapability::Keyboard as u64 != 0
                {
                    seat.add_device(
                        Some("keyboard"),
                        DeviceType::Virtual,
                        &[DeviceCapability::Keyboard],
                        |_| {},
                    );
                }

                // XXX button/etc should be on same object
                if connection.has_interface("ei_pointer")
                    && capabilities & 2 << DeviceCapability::Pointer as u64 != 0
                {
                    seat.add_device(
                        Some("pointer"),
                        DeviceType::Virtual,
                        &[DeviceCapability::Pointer],
                        |_| {},
                    );
                }

                if connection.has_interface("ei_touchscreen")
                    && capabilities & 2 << DeviceCapability::Touch as u64 != 0
                {
                    seat.add_device(
                        Some("touch"),
                        DeviceType::Virtual,
                        &[DeviceCapability::Touch],
                        |_| {},
                    );
                }

                if connection.has_interface("ei_pointer_absolute")
                    && capabilities & 2 << DeviceCapability::PointerAbsolute as u64 != 0
                {
                    seat.add_device(
                        Some("pointer-abs"),
                        DeviceType::Virtual,
                        &[DeviceCapability::PointerAbsolute],
                        |_| {},
                    );
                }

                // TODO create devices; compare against current bitflag
            }
            EisRequest::TouchCancel(request) => {
                // TODO protocol error if version wrong
            }
            _ => {}
        }

        calloop::PostAction::Continue
    }
}

struct State {
    handle: calloop::LoopHandle<'static, Self>,
}

impl State {
    fn handle_new_connection(&mut self, context: eis::Context) -> io::Result<calloop::PostAction> {
        println!("New connection: {:?}", context);

        let source = EisRequestSource::new(context, 1);
        let mut context_state = ContextState { seat: None };
        self.handle
            .insert_source(source, move |event, connected_state, state| match event {
                Ok(event) => Ok(state.handle_request_source_event(
                    &mut context_state,
                    connected_state,
                    event,
                )),
                Err(err) => Ok(context_state.protocol_error(connected_state, &err.to_string())),
            })
            .unwrap();

        Ok(calloop::PostAction::Continue)
    }

    fn connected(&mut self, connection: &Connection) {
        if !connection.has_interface("ei_seat") || !connection.has_interface("ei_device") {
            connection.disconnected(
                eis::connection::DisconnectReason::Protocol,
                "Need `ei_seat` and `ei_device`",
            );
            connection.flush();
        }

        let seat = connection.add_seat(
            Some("default"),
            &[
                DeviceCapability::Pointer,
                DeviceCapability::PointerAbsolute,
                DeviceCapability::Keyboard,
                DeviceCapability::Touch,
                DeviceCapability::Scroll,
                DeviceCapability::Button,
            ],
        );
    }

    fn handle_request_source_event(
        &mut self,
        context_state: &mut ContextState,
        connection: &Connection,
        event: EisRequestSourceEvent,
    ) -> calloop::PostAction {
        match event {
            EisRequestSourceEvent::Connected => {
                if !connection.has_interface("ei_seat") || !connection.has_interface("ei_device") {
                    connection.disconnected(
                        eis::connection::DisconnectReason::Protocol,
                        "Need `ei_seat` and `ei_device`",
                    );
                    connection.flush();
                }

                let seat = connection.add_seat(
                    Some("default"),
                    &[
                        DeviceCapability::Pointer,
                        DeviceCapability::PointerAbsolute,
                        DeviceCapability::Keyboard,
                        DeviceCapability::Touch,
                        DeviceCapability::Scroll,
                        DeviceCapability::Button,
                    ],
                );

                context_state.seat = Some(seat);
            }
            EisRequestSourceEvent::Request(request) => {
                let res = context_state.handle_request(connection, request);
                if res != calloop::PostAction::Continue {
                    return res;
                }
            }
            EisRequestSourceEvent::InvalidObject(object_id) => {
                // Only send if object ID is in range?
                connection
                    .connection()
                    .invalid_object(connection.last_serial(), object_id);
            }
        }

        connection.flush();

        calloop::PostAction::Continue
    }
}

fn main() {
    let mut event_loop = calloop::EventLoop::try_new().unwrap();
    let handle = event_loop.handle();

    let path = reis::default_socket_path().unwrap();
    std::fs::remove_file(&path); // XXX in use?
    let listener = eis::Listener::bind(&path).unwrap();
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
