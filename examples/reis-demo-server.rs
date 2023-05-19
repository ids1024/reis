use reis::eis;
use std::thread;

fn main() {
    let path = reis::default_socket_path().unwrap();
    std::fs::remove_file(&path); // XXX in use?
    let listener = eis::Listener::bind(&path).unwrap();
    for connection in listener.incoming() {
        thread::spawn(move || {
            println!("New connection: {:?}", connection);
            loop {
                let mut buf = [0; 16];
                let mut fds = Vec::new();
                let count = connection.recv(&mut buf, &mut fds).unwrap();
                if count == 0 {
                    break;
                }
                assert_eq!(count, 16); // XXX bad
                let header = reis::Header::parse(&buf).unwrap();

                println!("{:?}", header);

                let mut buf = vec![0; header.length as usize - 16];
                let mut fds = Vec::new();
                let count = connection.recv(&mut buf, &mut fds).unwrap();
                assert_eq!(count, buf.len());
            }
        });
    }
}
