use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdout, Command};
use tokio::sync::{mpsc, oneshot};

use super::models::{Direction, McpEvent};
use super::protocol::{self, Incoming, RpcError};
use super::session::{Emitter, MAX_MESSAGE_BYTES, traffic};
use crate::error::AppError;

const MAX_LOG_LINE: usize = 4 * 1024;
const QUEUE: usize = 64;
const EXIT_GRACE: Duration = Duration::from_secs(2);
/// Keeps a console window from opening for the server on Windows.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

type Waiting = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, RpcError>>>>>;

/// A server started as a program, speaking JSON-RPC one message per line on stdin/stdout.
pub struct StdioSession {
    writer: mpsc::Sender<String>,
    waiting: Waiting,
    next_id: AtomicU64,
    emit: Emitter,
    /// Dropping it (or sending) stops the program.
    stop: Mutex<Option<oneshot::Sender<()>>>,
}

impl StdioSession {
    /// Starts `program` directly, without a shell, so its arguments are never interpreted.
    pub fn spawn(
        program: &Path,
        args: &[String],
        env: &[(String, String)],
        emit: Emitter,
    ) -> Result<Self, AppError> {
        let mut command = Command::new(program);
        command
            .args(args)
            .envs(env.iter().map(|(name, value)| (name, value)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        let mut child = command.spawn().map_err(|error| {
            AppError::Connect(format!("Could not start “{}”: {error}", program.display()))
        })?;
        let (Some(stdin), Some(stdout), Some(stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            return Err(AppError::Connect(
                "Could not talk to the program".to_owned(),
            ));
        };

        let (writer, mut lines) = mpsc::channel::<String>(QUEUE);
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(line) = lines.recv().await {
                let written = async {
                    stdin.write_all(line.as_bytes()).await?;
                    stdin.write_all(b"\n").await?;
                    stdin.flush().await
                };
                if written.await.is_err() {
                    break;
                }
            }
        });

        let log = Arc::clone(&emit);
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            while let Ok(Some(line)) = read_line(&mut reader, MAX_LOG_LINE).await {
                if !line.trim().is_empty() && !log(McpEvent::Log { text: line }) {
                    break;
                }
            }
        });

        let waiting: Waiting = Arc::default();
        let (stop, stopped) = oneshot::channel();
        tokio::spawn(run(
            child,
            stdout,
            writer.clone(),
            Arc::clone(&waiting),
            Arc::clone(&emit),
            stopped,
        ));
        Ok(Self {
            writer,
            waiting,
            next_id: AtomicU64::new(1),
            emit,
            stop: Mutex::new(Some(stop)),
        })
    }

    pub async fn request(
        &self,
        method: &str,
        params: Option<&Value>,
        timeout: Duration,
    ) -> Result<Result<Value, RpcError>, AppError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (reply, answer) = oneshot::channel();
        lock(&self.waiting).insert(id, reply);
        let message = protocol::request(id, method, params).to_string();
        traffic(&self.emit, Direction::Sent, &message);
        if self.writer.send(message).await.is_err() {
            lock(&self.waiting).remove(&id);
            return Err(stopped());
        }
        match tokio::time::timeout(timeout, answer).await {
            Ok(Ok(outcome)) => Ok(outcome),
            Ok(Err(_)) => Err(stopped()),
            Err(_) => {
                lock(&self.waiting).remove(&id);
                Err(AppError::Timeout)
            }
        }
    }

    pub async fn notify(&self, method: &str) -> Result<(), AppError> {
        let message = protocol::notification(method).to_string();
        traffic(&self.emit, Direction::Sent, &message);
        self.writer.send(message).await.map_err(|_| stopped())
    }

    pub fn close(&self) {
        if let Some(stop) = lock(&self.stop).take() {
            // The program may already have ended; then there is nothing left to stop.
            let _ = stop.send(());
        }
    }
}

fn stopped() -> AppError {
    AppError::Request("The server is no longer running".to_owned())
}

/// The map stays consistent even if a holder panicked, so a poisoned lock is still usable.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

async fn run(
    mut child: Child,
    stdout: ChildStdout,
    writer: mpsc::Sender<String>,
    waiting: Waiting,
    emit: Emitter,
    mut stopped: oneshot::Receiver<()>,
) {
    let mut reader = BufReader::new(stdout);
    let reason = loop {
        tokio::select! {
            _ = &mut stopped => {
                // Already gone is fine; the program is stopped either way.
                let _ = child.kill().await;
                break "Disconnected".to_owned();
            }
            line = read_line(&mut reader, MAX_MESSAGE_BYTES) => match line {
                Ok(Some(line)) => handle_line(&line, &writer, &waiting, &emit).await,
                Ok(None) => break exit_reason(&mut child).await,
                Err(error) => {
                    let _ = child.kill().await;
                    break error.to_string();
                }
            },
        }
    };
    // Dropping the senders tells every request still waiting that the server is gone.
    lock(&waiting).clear();
    emit(McpEvent::Closed { reason });
}

async fn handle_line(line: &str, writer: &mpsc::Sender<String>, waiting: &Waiting, emit: &Emitter) {
    if line.trim().is_empty() {
        return;
    }
    let messages = protocol::parse_messages(line);
    if messages.is_empty() {
        // Some servers print a banner on stdout; it is not part of the protocol.
        emit(McpEvent::Log {
            text: line.to_owned(),
        });
        return;
    }
    traffic(emit, Direction::Received, line);
    for message in messages {
        match message {
            Incoming::Response { id, outcome } => {
                if let Some(reply) = lock(waiting).remove(&id) {
                    // The request may have timed out meanwhile; nobody waits for it then.
                    let _ = reply.send(outcome);
                }
            }
            Incoming::Request { id, method } => {
                let answer = protocol::reply_to_server(&id, &method).to_string();
                traffic(emit, Direction::Sent, &answer);
                // A closed writer means the program is ending; the loop notices that next.
                let _ = writer.send(answer).await;
            }
            Incoming::Notification { method, params } => {
                if method == "notifications/message"
                    && let Some(text) = log_text(params.as_ref())
                {
                    emit(McpEvent::Log { text });
                }
            }
        }
    }
}

/// `level: data` from a logging notification.
pub fn log_text(params: Option<&Value>) -> Option<String> {
    let params = params?;
    let level = params
        .get("level")
        .and_then(Value::as_str)
        .unwrap_or("info");
    let data = match params.get("data")? {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    Some(format!("{level}: {data}"))
}

async fn exit_reason(child: &mut Child) -> String {
    match tokio::time::timeout(EXIT_GRACE, child.wait()).await {
        Ok(Ok(status)) => match status.code() {
            Some(0) => "The server exited".to_owned(),
            Some(code) => format!("The server exited with code {code}"),
            None => "The server was stopped".to_owned(),
        },
        _ => {
            // It closed its output but kept running; it cannot be used anymore.
            let _ = child.kill().await;
            "The server stopped answering".to_owned()
        }
    }
}

/// One line without its line ending; `None` at the end of the output.
async fn read_line<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<Option<String>, AppError> {
    let mut line = Vec::new();
    let limit = u64::try_from(max_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let read = (&mut *reader)
        .take(limit)
        .read_until(b'\n', &mut line)
        .await
        .map_err(|_| AppError::Request("Could not read from the server".to_owned()))?;
    if read == 0 {
        return Ok(None);
    }
    if line.len() > max_bytes {
        return Err(AppError::Request(
            "A message from the server is larger than 16 MB".to_owned(),
        ));
    }
    let text = String::from_utf8_lossy(&line);
    Ok(Some(text.trim_end_matches(['\r', '\n']).to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reads_lines_and_refuses_overlong_ones() {
        let mut reader = BufReader::new(&b"first\r\nsecond\n0123456789\n"[..]);
        assert_eq!(
            read_line(&mut reader, 8).await.ok().flatten().as_deref(),
            Some("first")
        );
        assert_eq!(
            read_line(&mut reader, 8).await.ok().flatten().as_deref(),
            Some("second")
        );
        assert!(read_line(&mut reader, 8).await.is_err());
    }

    #[test]
    fn formats_logging_notifications() {
        let params = serde_json::json!({ "level": "warning", "data": "disk almost full" });
        assert_eq!(
            log_text(Some(&params)).as_deref(),
            Some("warning: disk almost full")
        );
        let structured = serde_json::json!({ "data": { "rows": 3 } });
        assert_eq!(
            log_text(Some(&structured)).as_deref(),
            Some(r#"info: {"rows":3}"#)
        );
    }
}
