use std::fs;
use std::sync::{Arc, Mutex, Condvar};
use std::io::{BufRead, BufReader};
use std::os::unix::net::{UnixListener, UnixStream};
use std::thread;
use std::sync::mpsc::{Sender, Receiver, channel};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use sqlparser::ast::Statement;
use ctrlc;

const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
const SQL_DELIMITER: u8 = b';';

fn handle_shutdown_signal(condvar: Arc<(Mutex<bool>, Condvar)>) {
    println!("Setting up ctrl+c handler");
    ctrlc::set_handler(move || {
        println!("Caught Ctrl+C");
        let (lock, cvar) = &*condvar;
        let mut shutdown = lock.lock().unwrap();
        *shutdown = true;
        cvar.notify_all();
    })
    .expect("Error setting Ctrl+C handler");
}


fn parse_msg(message: String) -> Result<Vec<Statement>, ()> {
    let dialect = GenericDialect{};
    return Parser::parse_sql(&dialect, message.as_str()).map_err(|err| {
        eprintln!("Could not parse SQL command: {:?}", err);
        ()
    });
}

fn handle_client(stream: UnixStream, rawcmds: Sender<String>) -> Result<(), ()> {
    let mut reader = BufReader::new(&stream);
    let mut buffer = Vec::new();

    match reader.read_until(SQL_DELIMITER, &mut buffer) {
        Ok(_) => {
            let raw_command = String::from_utf8_lossy(&buffer);
            return rawcmds.send(raw_command.to_string()).map_err(|err| {
                eprintln!("Error sending raw command: {}", err);
                ()
            });
        }
        Err(err) => {
            eprintln!("Error reading from the client: {}", err);
        }
    }

    Ok(())
}

fn command_thread(rawcmds: Receiver<String>, commands: Sender<Statement>) -> Result<(), ()> {
    loop {
        let msg = rawcmds.recv().expect("Command parser shutdown");
        match parse_msg(msg) {
            Ok(parsed) => {
                for statement in parsed {
                    commands.send(statement).expect("Could not send command");
                }
            }
            Err(_) => {
                println!("Could not parse message");
            }
        }
    }

}

fn main() -> Result<(), ()> {
    // output version information so users know what they're using
    println!("sqld version {}", VERSION.unwrap_or("unknown"));
    let (rawcmd_sender, rawcmd_receiver) = channel();
    let (command_sender, _command_receiver) = channel();

    let sock_path: &str = "/tmp/rust-sqld.sock";

    let condvar = Arc::new((Mutex::new(false), Condvar::new()));

    handle_shutdown_signal(condvar.clone());

    // 1. spawn command thread and start listening for raw strings to parse into commands
    thread::spawn(|| command_thread(rawcmd_receiver, command_sender));
    // 2. spawn write thread and start listening for commands
    // 3. create read thread pool and start them each listening for commands
    // 4. set up socket to listen for protocol messages locally
    let listener = UnixListener::bind(sock_path).expect("Failed to bind unix socket");

    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let rawcmds = rawcmd_sender.clone();
                    thread::spawn(|| {
                        match handle_client(stream, rawcmds) {
                            Ok(_) => {},
                            Err(_) => {
                                eprintln!("Client thread destroyed");
                            }
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {e}");
                }
            }
        }
    });

    // wait for shutdown_flag to be true
    let (lock, cvar) = &*condvar;
    let mut shutdown = lock.lock().unwrap();
    while !*shutdown {
        shutdown = cvar.wait(shutdown).unwrap();
    }

    println!("Shutting down gracefully");
    
    // 5. spawn TCPListener to listen for protocol messages

    fs::remove_file(sock_path).expect("Could not remove socket file.");

    Ok(())
}

