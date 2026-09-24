//! Host-side relay for the Bitburner MCP server.
//!
//! An MCP client (Claude Code, an editor, ...) spawns this binary and speaks
//! MCP over stdio. The real MCP server runs inside the Bitburner page, which
//! cannot listen for connections, so it dials out to this process over a
//! websocket instead. This binary is a dumb pipe between the two: it never
//! parses or interprets MCP.
//!
//! Wire rules:
//! - stdin/stdout carry newline-delimited JSON-RPC, as the MCP stdio
//!   transport requires. Nothing but JSON-RPC is written to stdout; logs go
//!   to stderr.
//! - The websocket carries one JSON-RPC message per text frame.
//! - While no game is connected, lines from the client wait in a bounded
//!   backlog rather than being dropped or answered with an error. Once the
//!   backlog is full, stdin is simply not read until the game connects.

use anyhow::{Context, Result};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Stdout};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

/// Address the in-game client dials. Override with `BITBURNER_MCP_BIND`.
/// The Remote File API convention is 12525; this is the next port up.
const DEFAULT_BIND: &str = "127.0.0.1:12526";

/// Client lines that may queue while the game is disconnected.
const BACKLOG: usize = 64;

type Sink = SplitSink<WebSocketStream<TcpStream>, Message>;
type Source = SplitStream<WebSocketStream<TcpStream>>;

/// Why a relay session ended.
enum Ended {
    /// The game closed the websocket; wait for it to reconnect.
    GameDisconnected,
    /// stdin hit EOF; the MCP client is gone and so are we.
    ClientClosed,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let bind = std::env::var("BITBURNER_MCP_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_owned());
    let listener = TcpListener::bind(&bind)
        .await
        .with_context(|| format!("binding {bind}"))?;
    tracing::info!(%bind, "listening for the in-game client");

    let mut from_client = spawn_stdin_reader();
    let mut stdout = tokio::io::stdout();
    // One line held back while we wait for the game, so we can still notice
    // the client going away without pulling an unbounded amount off stdin.
    let mut stash: Option<String> = None;

    loop {
        let (stream, peer) = tokio::select! {
            accepted = listener.accept() => accepted.context("accepting connection")?,
            line = from_client.recv(), if stash.is_none() => match line {
                Some(line) => {
                    stash = Some(line);
                    continue;
                }
                None => return Ok(()),
            },
        };

        let ws = match tokio_tungstenite::accept_async(stream).await {
            Ok(ws) => ws,
            Err(err) => {
                tracing::warn!(%peer, %err, "websocket handshake failed");
                continue;
            }
        };
        tracing::info!(%peer, "game connected");
        let (mut sink, mut source) = ws.split();

        if let Some(line) = stash.take() {
            sink.send(Message::Text(line.into()))
                .await
                .context("sending backlog to game")?;
        }

        match relay(&mut sink, &mut source, &mut from_client, &mut stdout).await? {
            Ended::GameDisconnected => tracing::info!(%peer, "game disconnected"),
            Ended::ClientClosed => return Ok(()),
        }
    }
}

/// Pump lines from the client into the websocket and frames from the game
/// onto stdout until one side goes away.
async fn relay(
    sink: &mut Sink,
    source: &mut Source,
    from_client: &mut mpsc::Receiver<String>,
    stdout: &mut Stdout,
) -> Result<Ended> {
    loop {
        tokio::select! {
            line = from_client.recv() => match line {
                Some(line) => sink
                    .send(Message::Text(line.into()))
                    .await
                    .context("sending to game")?,
                None => return Ok(Ended::ClientClosed),
            },
            frame = source.next() => match frame {
                Some(Ok(Message::Text(text))) => {
                    stdout.write_all(text.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                }
                Some(Ok(Message::Close(_))) | None => return Ok(Ended::GameDisconnected),
                Some(Ok(other)) => tracing::debug!(?other, "ignoring non-text frame"),
                Some(Err(err)) => {
                    tracing::warn!(%err, "websocket error");
                    return Ok(Ended::GameDisconnected);
                }
            },
        }
    }
}

/// Read stdin line by line on its own task. Dropping the sender on EOF is
/// how the rest of the program learns the client has gone.
fn spawn_stdin_reader() -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel(BACKLOG);
    tokio::spawn(async move {
        let mut lines = BufReader::new(tokio::io::stdin()).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) if line.trim().is_empty() => continue,
                Ok(Some(line)) => {
                    if tx.send(line).await.is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    tracing::warn!(%err, "reading stdin");
                    break;
                }
            }
        }
    });
    rx
}
