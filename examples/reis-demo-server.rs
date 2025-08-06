//! Demo server.
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
        connection: &Connection,
        reason: eis::connection::DisconnectReason,
        explaination: &str,
    ) -> calloop::PostAction {
        connection.disconnected(reason, explaination);
        let _ = connection.flush();
        calloop::PostAction::Remove
    }

    fn protocol_error(connection: &Connection, explanation: &str) -> calloop::PostAction {
        Self::disconnected(
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

                let seat = self.seat.as_ref().unwrap();

                if connection.has_interface("ei_keyboard")
                    && capabilities.contains(DeviceCapability::Keyboard)
                {
                    seat.add_device(
                        Some("keyboard"),
                        DeviceType::Virtual,
                        DeviceCapability::Keyboard.into(),
                        |_| {},
                    );
                }

                // XXX button/etc should be on same object
                if connection.has_interface("ei_pointer")
                    && capabilities.contains(DeviceCapability::Pointer)
                {
                    seat.add_device(
                        Some("pointer"),
                        DeviceType::Virtual,
                        DeviceCapability::Pointer.into(),
                        |_| {},
                    );
                }

                if connection.has_interface("ei_touchscreen")
                    && capabilities.contains(DeviceCapability::Touch)
                {
                    seat.add_device(
                        Some("touch"),
                        DeviceType::Virtual,
                        DeviceCapability::Touch.into(),
                        |_| {},
                    );
                }

                if connection.has_interface("ei_pointer_absolute")
                    && capabilities.contains(DeviceCapability::PointerAbsolute)
                {
                    seat.add_device(
                        Some("pointer-abs"),
                        DeviceType::Virtual,
                        DeviceCapability::PointerAbsolute.into(),
                        |_| {},
                    );
                }

                // TODO create devices; compare against current bitflag
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
    #![allow(clippy::unnecessary_wraps)]
    fn handle_new_connection(&mut self, context: eis::Context) -> io::Result<calloop::PostAction> {
        println!("New connection: {context:?}");

        let source = EisRequestSource::new(context, 1);
        let mut context_state = ContextState { seat: None };
        self.handle
            .insert_source(source, move |event, connected_state, _state| match event {
                Ok(event) => Ok(Self::handle_request_source_event(
                    &mut context_state,
                    connected_state,
                    event,
                )),
                Err(err) => {
                    if let reis::Error::Request(reis::request::RequestError::InvalidCapabilities) =
                        err
                    {
                        Ok(ContextState::disconnected(
                            connected_state,
                            eis::connection::DisconnectReason::Value,
                            &err.to_string(),
                        ))
                    } else {
                        Ok(ContextState::protocol_error(
                            connected_state,
                            &err.to_string(),
                        ))
                    }
                }
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
                if !connection.has_interface("ei_seat") || !connection.has_interface("ei_device") {
                    connection.disconnected(
                        eis::connection::DisconnectReason::Protocol,
                        "Need `ei_seat` and `ei_device`",
                    );
                    let _ = connection.flush();
                }

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
            EisRequestSourceEvent::InvalidObject(object_id) => {
                // Only send if object ID is in range?
                connection
                    .connection()
                    .invalid_object(connection.last_serial(), object_id);
            }
        }

        let _ = connection.flush();

        calloop::PostAction::Continue
    }
}

fn main() {
    let mut event_loop = calloop::EventLoop::try_new().unwrap();
    let handle = event_loop.handle();

    let path = reis::default_socket_path().unwrap();
    let _ = std::fs::remove_file(&path); // XXX in use?
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
