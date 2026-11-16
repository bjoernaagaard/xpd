//! The messages on stdin and stdout. lsp-server's own stdio transport hands the server one
//! message at a time and allocates the length a header states before it reads the body. This
//! one reads ahead, so the server sees every message that waits, and it refuses a length no
//! editor sends.

use std::io::{self, BufRead, Read};
use std::thread::{self, JoinHandle};

use lsp_server::{Connection, Message};

/// The longest body the server reads. A message holds at most one document, and a document
/// of this size takes longer to lint than anyone waits.
const MAX_CONTENT_LENGTH: u64 = 1 << 30;

/// The two threads that move messages between the client and the server's `Connection`.
#[derive(Debug)]
pub(crate) struct Stdio {
    reader: JoinHandle<io::Result<()>>,
    writer: JoinHandle<io::Result<()>>,
}

/// Starts the threads. The reader puts each message on the connection as soon as the client
/// has sent it, without waiting for the server to take the one before.
pub(crate) fn connect() -> io::Result<(Connection, Stdio)> {
    let (server, client) = Connection::memory();
    let Connection { sender, receiver } = client;
    let reader = thread::Builder::new()
        .name("xpd lsp reader".to_owned())
        .spawn(move || {
            let mut stdin = io::stdin().lock();
            while let Some(message) = read_message(&mut stdin)? {
                let is_exit = matches!(&message, Message::Notification(n) if n.method == "exit");
                // A failed send means the server stopped reading, and then nothing is left
                // to do.
                if sender.send(message).is_err() || is_exit {
                    break;
                }
            }
            Ok(())
        })?;
    let writer = thread::Builder::new()
        .name("xpd lsp writer".to_owned())
        .spawn(move || {
            let mut stdout = io::stdout().lock();
            receiver
                .into_iter()
                .try_for_each(|message| message.write(&mut stdout))
        })?;
    Ok((server, Stdio { reader, writer }))
}

impl Stdio {
    /// Waits for both threads, after the server dropped its `Connection`. Call it only when
    /// the client sent `exit` or closed the stream, because the reader waits for one of them.
    pub(crate) fn join(self) -> io::Result<()> {
        let read = self.reader.join();
        let written = self.writer.join();
        read.map_err(|_| io::Error::other("the thread that reads stdin panicked"))??;
        written.map_err(|_| io::Error::other("the thread that writes stdout panicked"))?
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// Reads one message: header lines that end in `\r\n`, an empty line, and a JSON body of
/// `Content-Length` bytes. `None` is the end of the stream.
fn read_message(input: &mut impl BufRead) -> io::Result<Option<Message>> {
    let mut length = None;
    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let Some(header) = line.strip_suffix("\r\n") else {
            return Err(invalid(format!("malformed header: {line:?}")));
        };
        if header.is_empty() {
            break;
        }
        let (name, value) = header
            .split_once(':')
            .ok_or_else(|| invalid(format!("malformed header: {header:?}")))?;
        if name.eq_ignore_ascii_case("Content-Length") {
            let value = value.trim();
            let stated: u64 = value
                .parse()
                .map_err(|e| invalid(format!("invalid Content-Length {value:?}: {e}")))?;
            length = Some(stated);
        }
    }
    let length = length.ok_or_else(|| invalid("no Content-Length"))?;
    if length > MAX_CONTENT_LENGTH {
        return Err(invalid(format!(
            "Content-Length {length} is over the limit of {MAX_CONTENT_LENGTH} bytes"
        )));
    }
    // The buffer grows with what arrives, so a length that the body does not fill takes no
    // memory.
    let mut body = Vec::new();
    input.take(length).read_to_end(&mut body)?;
    if u64::try_from(body.len()).ok() != Some(length) {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            format!(
                "the stream ended after {} of {length} bytes of a message",
                body.len()
            ),
        ));
    }
    let message = serde_json::from_slice(&body).map_err(|e| {
        invalid(format!(
            "malformed message: {e}: {:?}",
            String::from_utf8_lossy(&body)
        ))
    })?;
    Ok(Some(message))
}
