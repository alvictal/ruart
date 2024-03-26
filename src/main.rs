use core::time;
use std::fs::OpenOptions;
use std::io::{stdin, stdout, Read, Write};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::io::RawFd;
use std::process;
use std::env;
use std::thread;
use termios::*;
use termios::os::linux::B115200;
use termios::os::linux::B57600;


fn setup_fd(fd: RawFd, baudrate: u32) -> io::Result<()> {
    let mut termios = Termios::from_fd(fd)?;
  
    termios.c_iflag = IGNPAR | IGNBRK;
    termios.c_oflag = 0;
    termios.c_cflag = CS8 | CREAD | CLOCAL;
    termios.c_lflag = 0;
  
    cfsetspeed(&mut termios, baudrate)?;
    tcsetattr(fd, TCSANOW, &termios)?;
    tcflush(fd, TCIOFLUSH)?;
  
    Ok(())
  }


fn main() -> io::Result<()>  {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <device_path> <baudrate>", args[0]);
        process::exit(1);
    }

    let device_path = &args[1];
    let baudrate_str = &args[2];

    // Define pre-configured baud rates
    let supported_baudrates = [B9600, B115200, B57600, B38400];

    // Parse baudrate from string
    let baudrate: speed_t = match baudrate_str.parse::<u32>() {
        Ok(value) => {
            match supported_baudrates.iter().find(|&b| *b as u32 == value) {
                Some(_) => value as speed_t,
                None => {
                    eprintln!("Unsupported baudrate: {}", baudrate_str);
                    process::exit(1);
                }
            }
        }
        Err(_) => {
            eprintln!("Invalid baudrate: {}", baudrate_str);
            process::exit(1);
        }
    };

    // Open the UART device
    let mut uart_fd = OpenOptions::new()
        .read(true)
        .write(true)
        .open(device_path)?;

    setup_fd(uart_fd.as_raw_fd(), baudrate)?;

    let mut uart_writer = uart_fd.try_clone()?;

    // Spawn a thread to read from UART
    let uart_thread= thread::spawn(move|| -> io::Result<()> {
        loop {
            let mut buffer = [0u8; 512]; // Adjust buffer size as needed
            let bytes_read = uart_fd.read(&mut buffer)?;

            if bytes_read > 0 {
                stdout().write(&buffer[..bytes_read])?;
            }

            thread::sleep(time::Duration::from_millis(10));
        }
    });

    loop {
        // Get user input
        let mut input = String::new();
        stdin().read_line(&mut input)?;

        // Remove trailing newline
        input.pop();
        match input {
            _ if input == "q!" => break,
            _ => {
                uart_writer.write_all(input.as_bytes())?;
                println!("Sent command: {}", input);
            }
        }
    }


    uart_thread.join().unwrap()?; // Wait for the reading thread to finish

    Ok(())
}