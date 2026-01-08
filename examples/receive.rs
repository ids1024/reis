//! Capturing input asynchronously.
#![allow(clippy::incompatible_msrv)]

use ashpd::desktop::input_capture::{Barrier, Capabilities, InputCapture};
use futures::stream::StreamExt;
use reis::{ei, event::DeviceCapability};
use std::{num::NonZero, os::unix::net::UnixStream};

async fn open_connection() -> ei::Context {
    if let Some(context) = ei::Context::connect_to_env().unwrap() {
        context
    } else {
        eprintln!("Unable to find ei socket. Trying xdg desktop portal.");
        let input_capture = InputCapture::new().await.unwrap();
        let session = input_capture
            .create_session(
                None,
                Capabilities::Keyboard | Capabilities::Pointer | Capabilities::Touchscreen,
            )
            .await
            .unwrap()
            .0;
        let fd = input_capture.connect_to_eis(&session).await.unwrap();
        let stream = UnixStream::from(fd);
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
                let _h = region.height() as i32;
                Barrier::new(NonZero::new(n as u32 + 1).unwrap(), (x, y, x + w - 1, y))
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

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let context = open_connection().await;
    let (_connection, mut events) = context
        .handshake_tokio("receive-example", ei::handshake::ContextType::Receiver)
        .await
        .unwrap();
    while let Some(event) = events.next().await {
        let event = event.unwrap();
        println!("{event:?}");
        match &event {
            reis::event::EiEvent::SeatAdded(evt) => {
                // println!("    capabilities: {:?}", evt.seat);
                evt.seat.bind_capabilities(
                    DeviceCapability::Pointer
                        | DeviceCapability::PointerAbsolute
                        | DeviceCapability::Keyboard
                        | DeviceCapability::Touch
                        | DeviceCapability::Scroll
                        | DeviceCapability::Button,
                );
                let _ = context.flush();
            }
            reis::event::EiEvent::DeviceAdded(evt) => {
                println!("  seat: {:?}", evt.device.seat().name());
                println!("  type: {:?}", evt.device.device_type());
                if let Some(dimensions) = evt.device.dimensions() {
                    println!("  dimensions: {dimensions:?}");
                }
                println!("  regions: {:?}", evt.device.regions());
                if let Some(keymap) = evt.device.keymap() {
                    println!("  keymap: {keymap:?}");
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
