use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

use chrono::{Datelike, Local};
use chrono::Timelike;
use clap::{Parser, Subcommand};
use serialport::{available_ports, SerialPortType, UsbPortInfo};

#[derive(Debug, Subcommand, Clone)]
enum Command {
    Read {
        /// The device path to a serial port
        #[arg(short, long)]
        port: String,

        /// Output file.
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// The baud rate to connect at
        #[arg(short, long)]
        baud: u32,
    },

    List,
}

#[derive(Parser)]
#[command(
    author,
    about,
    long_about = "Reads data from a serial port and writes it to a file"
)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

fn main() {
    let cli = Cli::parse();

    let (port_path, output, baud) = match cli.command {
        Command::Read { port, output, baud } => (port, output, baud),
        Command::List => {
            list_ports();
            return;
        }
    };

    let port = serialport::new(&port_path, baud)
        .timeout(Duration::from_millis(10))
        .open();

    match port {
        Ok(mut port) => {
            let mut serial_buf: Vec<u8> = vec![0; 1000];
            println!("Receiving data on {} at {} baud:", &port_path, baud);
            let mut accumulated_data = Vec::new();

            loop {
                match port.read(serial_buf.as_mut_slice()) {
                    Ok(t) => {
                        accumulated_data.extend_from_slice(&serial_buf[..t]);

                        // Split the accumulated data by newlines
                        while let Some(pos) = accumulated_data.iter().position(|&x| x == b'\n') {
                            let line = accumulated_data.drain(..=pos).collect::<Vec<u8>>();
                            let timestamp = generate_timestamp().into_bytes();

                            let mut data = Vec::with_capacity(timestamp.len() + line.len());
                            data.extend_from_slice(&timestamp);
                            data.extend_from_slice(&line);

                            io::stdout().write_all(&data).unwrap();
                            io::stdout().flush().unwrap();
                            if let Some(ref file) = &output {
                                let mut file = match OpenOptions::new()
                                    .write(true)
                                    .append(true)
                                    .create(true)
                                    .open(file)
                                {
                                    Ok(file) => file,
                                    Err(e) => {
                                        eprintln!(
                                            "Failed to open \"{}\". Error: {}",
                                            output.as_ref().unwrap().to_str().unwrap(),
                                            e
                                        );
                                        ::std::process::exit(1);
                                    }
                                };
                                file.write_all(&data).unwrap();
                                file.flush().unwrap();
                            }
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                    Err(e) => eprintln!("{e:?}"),
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open \"{}\". Error: {}", &port_path, e);
            ::std::process::exit(1);
        }
    }
}

fn list_ports() {
    if let Ok(ports) = available_ports() {
        match ports.len() {
            0 => println!("No ports found."),
            1 => println!("Found 1 port:"),
            n => println!("Found {} ports:", n),
        }

        for p in ports {
            println!("  {}", p.port_name);
            match p.port_type {
                SerialPortType::UsbPort(info) => {
                    println!("    Type: USB");
                    print_usb_info(&info);
                }
                SerialPortType::BluetoothPort => {
                    println!("    Type: Bluetooth");
                }
                SerialPortType::PciPort => {
                    println!("    Type: PCI");
                }
                SerialPortType::Unknown => {
                    println!("    Type: Unknown");
                }
            }
        }
    } else {
        eprintln!("Error listing serial ports");
    }
}

fn print_usb_info(info: &UsbPortInfo) {
    println!("    VID:{:04x} PID:{:04x}", info.vid, info.pid);
    print_optional_info("Serial Number", &info.serial_number);
    print_optional_info("Manufacturer", &info.manufacturer);
    print_optional_info("Product", &info.product);

    #[cfg(feature = "usbportinfo-interface")]
    if let Some(interface) = &info.interface {
        println!("         Interface: {:02x}", interface);
    }
}

fn print_optional_info(label: &str, opt: &Option<String>) {
    println!(
        "    {:<15}: {}",
        label,
        opt.as_ref().map_or("", String::as_str)
    );
}

fn generate_timestamp() -> String {
    let now = Local::now();

    format!(
        "{RESET}[{GREEN}{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}{RESET}] ",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        now.timestamp_subsec_millis(),
        RESET = "\x1b[0m",
        GREEN = "\x1b[32m",
    )
}
