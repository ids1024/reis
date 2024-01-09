use ashpd::desktop::input_capture::{Barrier, Capabilities, InputCapture};
use once_cell::sync::Lazy;
use pollster::FutureExt as _;
use reis::{
    ei,
    event::{DeviceCapability, EiConvertEventIterator},
};
use std::{
    collections::HashMap,
    os::unix::{io::FromRawFd, net::UnixStream},
};
use xkbcommon::xkb;

static INTERFACES: Lazy<HashMap<&'static str, u32>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("ei_connection", 1);
    m.insert("ei_callback", 1);
    m.insert("ei_pingpong", 1);
    m.insert("ei_seat", 1);
    m.insert("ei_device", 2);
    m.insert("ei_pointer", 1);
    m.insert("ei_pointer_absolute", 1);
    m.insert("ei_scroll", 1);
    m.insert("ei_button", 1);
    m.insert("ei_keyboard", 1);
    m.insert("ei_touchscreen", 1);
    m
});

async fn open_connection() -> ei::Context {
    if let Some(context) = ei::Context::connect_to_env().unwrap() {
        context
    } else {
        eprintln!("Unable to find ei socket. Trying xdg desktop portal.");
        let input_capture = InputCapture::new().await.unwrap();
        let session = input_capture
            .create_session(
                &ashpd::WindowIdentifier::default(),
                (Capabilities::Keyboard | Capabilities::Pointer | Capabilities::Touchscreen).into(),
            )
            .await
            .unwrap()
            .0;
        let raw_fd = input_capture.connect_to_eis(&session).await.unwrap();
        let stream = unsafe { UnixStream::from_raw_fd(raw_fd) };
        let zones = input_capture
            .zones(&session)
            .await
            .unwrap()
            .response()
            .unwrap();

        let barriers = zones
            .regions()
            .iter()
            .enumerate()
            .map(|(n, region)| {
                let x = region.x_offset();
                let y = region.y_offset();
                let w = region.width() as i32;
                let h = region.height() as i32;
                Barrier::new(n as u32 + 1, (x, y, x + w - 1, y))
            })
            .collect::<Vec<_>>();
        let resp = input_capture
            .set_pointer_barriers(&session, &barriers, zones.zone_set())
            .await
            .unwrap()
            .response()
            .unwrap();
        assert_eq!(&resp.failed_barriers(), &[]);
        eprintln!("Set capture barrier to top edge of screen.");
        eprintln!("(When input is captured, Esc will exit.)");
        input_capture.enable(&session).await.unwrap();
        ei::Context::new(stream).unwrap()
    }
}

fn main() {
    let context = open_connection().block_on();
    reis::handshake::ei_handshake_blocking(
        &context,
        "receive-example",
        ei::handshake::ContextType::Receiver,
        &INTERFACES,
    )
    .unwrap();

    let mut events = EiConvertEventIterator::new(context.clone());
    while let Some(event) = events.next() {
        let event = event.unwrap();
        println!("{event:?}");
        match &event {
            reis::event::EiEvent::SeatAdded(evt) => {
                // println!("    capabilities: {:?}", evt.seat);
                evt.seat.bind_capabilities(&[
                    DeviceCapability::Pointer,
                    DeviceCapability::PointerAbsolute,
                    DeviceCapability::Keyboard,
                    DeviceCapability::Touch,
                    DeviceCapability::Scroll,
                    DeviceCapability::Button,
                ]);
                context.flush();
            }
            reis::event::EiEvent::DeviceAdded(evt) => {
                println!("  seat: {:?}", evt.device.seat().name());
                println!("  type: {:?}", evt.device.device_type());
                if let Some(dimensions) = evt.device.dimensions() {
                    println!("  dimensions: {:?}", dimensions);
                }
                println!("  regions: {:?}", evt.device.regions());
                if let Some(keymap) = evt.device.keymap() {
                    println!("  keymap: {:?}", keymap);
                }
                // Interfaces?
            }
            reis::event::EiEvent::KeyboardKey(evt) => {
                // Escape key
                if evt.key == 1 {
                    std::process::exit(0);
                }
            }
            _ => {}
        }
    }
}
