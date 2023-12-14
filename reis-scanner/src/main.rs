use std::{fs, io};

mod proto;

fn main() {
    let file = io::BufReader::new(fs::File::open("../../libei/proto/protocol.xml").unwrap());
    println!(
        "{:#?}",
        quick_xml::de::from_reader::<_, proto::Protocol>(file).unwrap()
    );
}
