//! Serial ports for `std.devices` (spec 14.18, decision 0068), through
//! `serial2`, for the interpreter and native code alike. Open ports are
//! kept here by number, as an operating system keeps open files.

use std::io::ErrorKind;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Why a port couldn't be opened, read or written: `DeviceError` in
/// `std.devices`.
#[derive(Debug, PartialEq)]
pub enum DevError {
    NotFound(String),
    Denied(String),
    Failed(String, String),
}

struct Open {
    path: String,
    port: Arc<serial2::SerialPort>,
}

static PORTS: Mutex<(i64, Vec<(i64, Open)>)> = Mutex::new((1, Vec::new()));

fn error(path: &str, e: std::io::Error) -> DevError {
    match e.kind() {
        ErrorKind::NotFound => DevError::NotFound(path.to_string()),
        ErrorKind::PermissionDenied => DevError::Denied(path.to_string()),
        _ => DevError::Failed(path.to_string(), e.to_string()),
    }
}

fn port(id: i64) -> Result<(String, Arc<serial2::SerialPort>), DevError> {
    let ports = PORTS.lock().unwrap_or_else(|e| e.into_inner());
    let found = ports.1.iter().find(|(i, _)| *i == id);
    found
        .map(|(_, o)| (o.path.clone(), Arc::clone(&o.port)))
        .ok_or_else(|| DevError::Failed(String::new(), "the port is closed".into()))
}

/// The serial ports there are, by path or name.
pub fn ports() -> Vec<String> {
    let mut names: Vec<String> = serial2::SerialPort::available_ports()
        .unwrap_or_default()
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Opens the port at `path` at `baud`, 8 data bits, no parity, 1 stop
/// bit, and gives its number.
pub fn open(path: &str, baud: i64) -> Result<i64, DevError> {
    let baud = u32::try_from(baud)
        .map_err(|_| DevError::Failed(path.into(), format!("{baud} isn't a baud rate")))?;
    let mut port = serial2::SerialPort::open(path, baud).map_err(|e| error(path, e))?;
    port.set_read_timeout(Duration::from_secs(1))
        .map_err(|e| error(path, e))?;
    let mut ports = PORTS.lock().unwrap_or_else(|e| e.into_inner());
    let id = ports.0;
    ports.0 += 1;
    ports.1.push((
        id,
        Open {
            path: path.to_string(),
            port: Arc::new(port),
        },
    ));
    Ok(id)
}

/// Up to `max` bytes from port `id`: what has arrived, waiting at most a
/// second for the first; none when nothing came.
pub fn read(id: i64, max: i64) -> Result<Vec<u8>, DevError> {
    let (path, port) = port(id)?;
    let mut buf = vec![0u8; usize::try_from(max).unwrap_or(0).min(1 << 20)];
    if buf.is_empty() {
        return Ok(buf);
    }
    match port.read(&mut buf) {
        Ok(n) => {
            buf.truncate(n);
            Ok(buf)
        }
        Err(e) if matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) => Ok(Vec::new()),
        Err(e) => Err(error(&path, e)),
    }
}

/// Sends `data` on port `id`.
pub fn write(id: i64, data: &[u8]) -> Result<(), DevError> {
    let (path, port) = port(id)?;
    port.write_all(data).map_err(|e| error(&path, e))
}

/// Closes port `id`; closing it again does nothing.
pub fn close(id: i64) {
    let mut ports = PORTS.lock().unwrap_or_else(|e| e.into_inner());
    ports.1.retain(|(i, _)| *i != id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_port_that_isnt_there_is_not_found() {
        let missing = if cfg!(windows) {
            "COM250"
        } else {
            "/dev/ttyKAVACH0"
        };
        assert!(matches!(
            open(missing, 9_600),
            Err(DevError::NotFound(_) | DevError::Failed(..))
        ));
        assert!(read(424_242, 8).is_err());
        close(424_242);
    }
}
