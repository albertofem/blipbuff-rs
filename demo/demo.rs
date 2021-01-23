mod basic;
mod chip;

extern crate blipbuff;
extern crate clap;
extern crate hound;
extern crate text_io;
extern crate unwrap;

use clap::{App, Arg};

fn main() {
    let matches = App::new("blipbuff-rs demos")
        .version("1.0.0")
        .author("Alberto Fernández <albertofem@gmail.com>")
        .about("blipbuff-rs demos")
        .arg(
            Arg::with_name("demo-name")
                .help("Name of the demo to run")
                .required(true)
                .index(1),
        )
        .get_matches();

    let demo_name = matches.value_of("demo-name").unwrap();

    match demo_name {
        "basic" => basic::run(),
        "chip" => chip::run(),
        _ => println!("Invalid demo name. Exiting..."),
    }

    return;
}
