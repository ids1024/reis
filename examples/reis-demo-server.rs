// TODO: Require context_type

use once_cell::sync::Lazy;
use reis::{
    calloop::{ConnectedContextState, EisListenerSource, EisRequestSource, EisRequestSourceEvent},
    eis::{self, device::DeviceType},
    request::{DeviceCapability, EisRequest},
};
use std::{
    collections::HashMap,
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

static SERVER_INTERFACES: Lazy<HashMap<&'static str, u32>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("ei_callback", 1);
    m.insert("ei_connection", 1);
    m.insert("ei_seat", 1);
    m.insert("ei_device", 1);
    m.insert("ei_pingpong", 1);
    m.insert("ei_keyboard", 1);
    m.insert("ei_pointer", 1);
    m.insert("ei_pointer_absolute", 1);
    m.insert("ei_button", 1);
    m.insert("ei_scroll", 1);
    m.insert("ei_touchscreen", 1);
    m
});

struct ContextState {
    seat: Option<reis::request::Seat>,
}

impl ContextState {
    fn disconnected(
        &self,
        connected_state: &ConnectedContextState,
        reason: eis::connection::DisconnectReason,
        explaination: &str,
    ) -> calloop::PostAction {
        connected_state.connection.disconnected(
            connected_state.request_converter.last_serial(),
            reason,
            explaination,
        );
        connected_state.context.flush();
        calloop::PostAction::Remove
    }

    fn protocol_error(
        &self,
        connected_state: &ConnectedContextState,
        explanation: &str,
    ) -> calloop::PostAction {
        self.disconnected(
            connected_state,
            eis::connection::DisconnectReason::Protocol,
            explanation,
        )
    }

    fn handle_request(
        &mut self,
        connected_state: &mut ConnectedContextState,
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
                    let serial = connected_state.request_converter.next_serial();
                    request.seat.eis_seat().destroyed(serial);
                    return self.disconnected(
                        connected_state,
                        eis::connection::DisconnectReason::Value,
                        "Invalid capabilities",
                    );
                }

                if connected_state.has_interface("ei_keyboard")
                    && capabilities & 2 << DeviceCapability::Keyboard as u64 != 0
                {
                    connected_state.request_converter.add_device(
                        self.seat.as_ref().unwrap(),
                        Some("keyboard"),
                        DeviceType::Virtual,
                        &[DeviceCapability::Keyboard],
                        |_| {}
                    );
                }

                // XXX button/etc should be on same object
                if connected_state.has_interface("ei_pointer")
                    && capabilities & 2 << DeviceCapability::Pointer as u64 != 0
                {
                    connected_state.request_converter.add_device(
                        self.seat.as_ref().unwrap(),
                        Some("pointer"),
                        DeviceType::Virtual,
                        &[DeviceCapability::Pointer],
                        |_| {}
                    );
                }

                if connected_state.has_interface("ei_touchscreen")
                    && capabilities & 2 << DeviceCapability::Touch as u64 != 0
                {
                    connected_state.request_converter.add_device(
                        self.seat.as_ref().unwrap(),
                        Some("touch"),
                        DeviceType::Virtual,
                        &[DeviceCapability::Touch],
                        |_| {}
                    );
                }

                if connected_state.has_interface("ei_pointer_absolute")
                    && capabilities & 2 << DeviceCapability::PointerAbsolute as u64 != 0
                {
                    connected_state.request_converter.add_device(
                        self.seat.as_ref().unwrap(),
                        Some("pointer-abs"),
                        DeviceType::Virtual,
                        &[DeviceCapability::PointerAbsolute],
                        |_| {}
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
    fn handle_new_connection(&mut self, context: eis::Context) -> io::Result<calloop::PostAction> {
        println!("New connection: {:?}", context);

        let source = EisRequestSource::new(context, &SERVER_INTERFACES, 1);
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

    fn connected(&mut self, mut connected_state: ConnectedContextState) {
        if !connected_state
            .negotiated_interfaces
            .contains_key("ei_seat")
            || !connected_state
                .negotiated_interfaces
                .contains_key("ei_device")
        {
            connected_state.connection.disconnected(
                1,
                eis::connection::DisconnectReason::Protocol,
                "Need `ei_seat` and `ei_device`",
            );
            connected_state.context.flush();
        }

        let seat = connected_state.request_converter.add_seat(
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
        connected_state: &mut ConnectedContextState,
        event: EisRequestSourceEvent,
    ) -> calloop::PostAction {
        match event {
            EisRequestSourceEvent::Connected => {
                if !connected_state
                    .negotiated_interfaces
                    .contains_key("ei_seat")
                    || !connected_state
                        .negotiated_interfaces
                        .contains_key("ei_device")
                {
                    connected_state.connection.disconnected(
                        1,
                        eis::connection::DisconnectReason::Protocol,
                        "Need `ei_seat` and `ei_device`",
                    );
                    connected_state.context.flush();
                }

                let seat = connected_state.request_converter.add_seat(
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
                let res = context_state.handle_request(connected_state, request);
                if res != calloop::PostAction::Continue {
                    return res;
                }
            }
            EisRequestSourceEvent::InvalidObject(object_id) => {
                // Only send if object ID is in range?
                connected_state
                    .connection
                    .invalid_object(connected_state.request_converter.last_serial(), object_id);
            }
        }

        connected_state.context.flush();

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
