#[macro_use]
extern crate log;

use conn::ConnStream;
use crossterm::{execute, terminal};
use mio::{net::TcpStream, Events, Interest, Poll, Token};
use parking_lot::Mutex;
use sc_proxy::stream::java::JavaStream;
use std::{env, error::Error, io, sync::Arc, thread};

mod cli;
mod command;
mod conn;
mod handle;
mod status;

fn main() {
  let (_cols, rows) = terminal::size().unwrap();
  cli::setup().unwrap();
  sc_common::init_with_stdout("cli", cli::skip_appender(15, rows - 30));
  match run(rows) {
    Ok(_) => (),
    Err(e) => {
      terminal::disable_raw_mode().unwrap();
      execute!(io::stdout(), terminal::LeaveAlternateScreen).unwrap();
      error!("error: {}", e);
      std::process::exit(1);
    }
  };
}

fn run(rows: u16) -> Result<(), Box<dyn Error>> {
  let mut args = env::args();
  args.next(); // current process
  let ip = args.next().unwrap_or("127.0.0.1:25565".into());

  // let ver = ProtocolVersion::V1_8;

  info!("connecting to {}", ip);
  let mut stream = TcpStream::connect(ip.parse()?)?;
  info!("connection established");

  let mut poll = Poll::new()?;
  let mut events = Events::with_capacity(1024);
  // let (needs_flush_tx, needs_flush_rx) = crossbeam_channel::bounded(1024);

  poll.registry().register(&mut stream, Token(0), Interest::READABLE | Interest::WRITABLE)?;

  let mut conn = ConnStream::new(JavaStream::new(stream));
  conn.start_handshake();
  let conn = Arc::new(Mutex::new(conn));

  let status = Arc::new(Mutex::new(status::Status::new()));
  status::Status::enable_drawing(status.clone());

  // let mut next_token = 0;

  let c = conn.clone();
  thread::spawn(move || {
    let mut lr = cli::LineReader::new("> ", rows - 15, 15);
    loop {
      match lr.read_line() {
        Ok(line) => {
          if line.is_empty() {
            continue;
          }
          let mut sections = line.split(' ');
          let command = sections.next().unwrap();
          let args: Vec<_> = sections.collect();
          let mut conn = c.lock();
          match command::handle(command, &args, &mut conn, &mut lr) {
            Ok(_) => {}
            Err(e) => {
              error!("error handling command: {}", e);
            }
          }
        }
        Err(_) => break,
      }
    }
  });

  loop {
    // Wait for events
    poll.poll(&mut events, None)?;

    for event in &events {
      let tok = event.token();

      let mut closed = false;
      let mut conn = conn.lock();
      if event.is_readable() {
        // let conn = clients.get_mut(&token).expect("client doesn't exist!");
        loop {
          match conn.poll() {
            Ok(_) => match conn.read() {
              Ok(p) => {
                if conn.closed() {
                  closed = true;
                  break;
                } else if let Some(p) = p {
                  handle::handle_packet(&mut conn, &status, p);
                }
              }
              Err(e) => {
                error!("error while parsing packet from client {:?}: {}", tok, e);
                closed = true;
                break;
              }
            },
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) => {
              error!("error while listening to client {:?}: {}", tok, e);
              closed = true;
              break;
            }
          }
        }
      }
      // The order here is important. If we are handshaking, then reading a packet
      // will probably prompt a direct response. In this arrangement, we can send more
      // packets before going back to poll().
      if event.is_writable() && !closed {
        // let conn = clients.get_mut(&token).expect("client doesn't exist!");
        while conn.needs_flush() {
          match conn.flush() {
            Ok(_) => {}
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) => {
              error!("error while flushing packets to the client {:?}: {}", tok, e);
              break;
            }
          }
        }
      }
    }
  }

  // handle::handshake(&mut reader, &mut writer, ver).await?;
  // info!("login complete");
  //
  // let reader = ConnReader { stream: reader, ver };
  // let writer = Arc::new(Mutex::new(ConnWriter { stream: writer, ver }));
  // let status = Arc::new(Mutex::new(status::Status::new()));
  // status::Status::enable_drawing(status.clone());
  //
  // let w = writer.clone();
  // let s = status.clone();
  // tokio::spawn(async move {
  //   let mut handler = handle::Handler { reader, writer: w, status: s };
  //   match handler.run().await {
  //     Ok(_) => warn!("handler exited"),
  //     Err(e) => error!("handler error: {}", e),
  //   }
  // });

  // info!("closing");
  //
  // Ok(())
}
