Reis :rice: provides a Rust version of EI :egg: and EIS :icecream: for emulated input on Wayland.

See the upstream project [libei](https://gitlab.freedesktop.org/libinput/libei) for more information.

This library should be usable for both clients and servers, but the API is subject to change, and it still lacks some checks that `libei` does.

Setting the env var `REIS_DEBUG` will make the library print ei messages it sends and receives.
