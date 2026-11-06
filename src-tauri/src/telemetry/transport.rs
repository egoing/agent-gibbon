//! A single bounded queue, checked again at the HTTP boundary. Consent epochs
//! prevent an old client or buffered session/log batch replaying after re-opt-in.
use super::*;
use sentry::{Envelope, Transport, TransportFactory};
use std::{
    collections::VecDeque,
    sync::{atomic::AtomicU64, Condvar, Mutex, OnceLock},
};

#[derive(Default)]
struct Gate {
    allowed: AtomicBool,
    epoch: AtomicU64,
    sending: Mutex<()>,
}
impl Gate {
    fn current(&self) -> Option<u64> {
        self.allowed
            .load(Ordering::SeqCst)
            .then(|| self.epoch.load(Ordering::SeqCst))
    }
    fn change(&self, allow: bool) {
        // Closing admission first stops new requests while an already-started
        // upload finishes. The HTTP timeout bounds this barrier.
        self.allowed.store(false, Ordering::SeqCst);
        self.epoch.fetch_add(1, Ordering::SeqCst);
        let _barrier = self.sending.lock().unwrap_or_else(|e| e.into_inner());
        self.allowed.store(allow, Ordering::SeqCst);
    }
    fn deliver(&self, epoch: u64, send: impl FnOnce()) -> bool {
        let _barrier = self.sending.lock().unwrap_or_else(|e| e.into_inner());
        if self.current() != Some(epoch) {
            return false;
        }
        send();
        true
    }
}
fn gate() -> &'static Arc<Gate> {
    static G: OnceLock<Arc<Gate>> = OnceLock::new();
    G.get_or_init(|| Arc::new(Gate::default()))
}
pub(super) fn epoch() -> Option<u64> {
    gate().current()
}
pub fn permit(allowed: bool) {
    gate().change(allowed);
}

struct Pending {
    entries: VecDeque<(u64, Envelope)>,
    submitted: u64,
    completed: u64,
    stopping: bool,
}
struct Queue {
    state: Mutex<Pending>,
    ready: Condvar,
}
struct HttpTransport {
    queue: Arc<Queue>,
    epoch: u64,
    gate: Arc<Gate>,
}
impl HttpTransport {
    fn new(options: &sentry::ClientOptions) -> Self {
        Self::with_gate(options, gate().clone())
    }
    fn with_gate(options: &sentry::ClientOptions, gate: Arc<Gate>) -> Self {
        let epoch = gate.current().unwrap_or(0);
        let queue = Arc::new(Queue {
            state: Mutex::new(Pending {
                entries: VecDeque::new(),
                submitted: 0,
                completed: 0,
                stopping: false,
            }),
            ready: Condvar::new(),
        });
        let worker = queue.clone();
        let permission = gate.clone();
        let endpoint = options
            .dsn
            .as_ref()
            .map(|d| d.envelope_api_url().to_string());
        let auth = options
            .dsn
            .as_ref()
            .map(|d| d.to_auth(Some("agent-gibbon")).to_string());
        std::thread::spawn(move || {
            // Construct/drop the blocking HTTP client on this dedicated thread.
            let builder = reqwest::blocking::Client::builder();
            #[cfg(test)]
            let builder = builder.no_proxy();
            let client = builder
                .timeout(Duration::from_secs(3))
                .connect_timeout(Duration::from_secs(1))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .ok();
            let mut retry_at = Instant::now();
            loop {
                let packet = {
                    let mut s = worker.state.lock().unwrap();
                    while s.entries.is_empty() && !s.stopping {
                        s = worker.ready.wait(s).unwrap();
                    }
                    if s.stopping {
                        s.entries.clear();
                        s.completed = s.submitted;
                        worker.ready.notify_all();
                        break;
                    }
                    s.entries.pop_front().unwrap()
                };
                if Instant::now() >= retry_at {
                    if let (Some(client), Some(endpoint), Some(auth)) = (&client, &endpoint, &auth)
                    {
                        let mut bytes = Vec::new();
                        if packet.1.to_writer(&mut bytes).is_ok() && bytes.len() <= 512 * 1024 {
                            permission.deliver(packet.0, || {
                                if let Ok(response) = client
                                    .post(endpoint)
                                    .header("X-Sentry-Auth", auth)
                                    .header("Content-Type", "application/x-sentry-envelope")
                                    .body(bytes)
                                    .send()
                                {
                                    let seconds = response
                                        .headers()
                                        .get("x-sentry-rate-limits")
                                        .and_then(|v| v.to_str().ok())
                                        .and_then(|s| s.split(':').next())
                                        .and_then(|s| s.parse::<f64>().ok())
                                        .or_else(|| {
                                            response
                                                .headers()
                                                .get("retry-after")
                                                .and_then(|v| v.to_str().ok())
                                                .and_then(|s| s.parse::<f64>().ok())
                                        })
                                        .or_else(|| {
                                            (response.status().as_u16() == 429).then_some(60.)
                                        });
                                    if let Some(s) = seconds.filter(|s| s.is_finite() && *s >= 0.) {
                                        retry_at = Instant::now()
                                            + Duration::from_secs_f64(s.clamp(1., 3600.));
                                    }
                                }
                            });
                        }
                    }
                }
                let mut s = worker.state.lock().unwrap();
                s.completed += 1;
                worker.ready.notify_all();
            }
        });
        Self { queue, epoch, gate }
    }
}
impl Transport for HttpTransport {
    fn send_envelope(&self, input: Envelope) {
        if self.gate.current() != Some(self.epoch) {
            return;
        }
        let envelope = super::privacy::envelope(input);
        if envelope.items().next().is_none() {
            return;
        }
        let mut s = self.queue.state.lock().unwrap();
        if s.stopping || s.entries.len() >= 64 {
            return;
        }
        s.submitted += 1;
        s.entries.push_back((self.epoch, envelope));
        self.queue.ready.notify_all();
    }
    fn flush(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut s = self.queue.state.lock().unwrap();
        let target = s.submitted;
        while s.completed < target {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            let Ok((next, timed)) = self.queue.ready.wait_timeout(s, left) else {
                return false;
            };
            s = next;
            if timed.timed_out() && s.completed < target {
                return false;
            }
        }
        true
    }
    fn shutdown(&self, timeout: Duration) -> bool {
        let flushed = self.flush(timeout);
        let mut s = self.queue.state.lock().unwrap();
        s.stopping = true;
        self.queue.ready.notify_all();
        flushed
    }
}
impl Drop for HttpTransport {
    fn drop(&mut self) {
        if let Ok(mut s) = self.queue.state.lock() {
            s.stopping = true;
            self.queue.ready.notify_all();
        }
    }
}
pub struct Factory;
impl TransportFactory for Factory {
    fn create_transport(&self, options: &sentry::ClientOptions) -> Arc<dyn Transport> {
        Arc::new(HttpTransport::new(options))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    #[test]
    fn http_queue_cannot_send_without_consent_or_replay_an_old_client() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let received = count.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !stopped.load(Ordering::Relaxed) && Instant::now() < deadline {
                if let Ok((mut socket, _)) = listener.accept() {
                    received.fetch_add(1, Ordering::Relaxed);
                    socket
                        .set_read_timeout(Some(Duration::from_secs(1)))
                        .unwrap();
                    let mut bytes = [0u8; 8192];
                    let _ = socket.read(&mut bytes);
                    let _=socket.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                } else {
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        });
        let options = sentry::ClientOptions {
            dsn: Some(format!("http://public@{address}/1").parse().unwrap()),
            ..Default::default()
        };
        let permission = Arc::new(Gate::default());
        let event = || {
            let mut e = Envelope::new();
            e.add_item(p::Event {
                message: Some("Agent Gibbon diagnostic probe".into()),
                ..Default::default()
            });
            e
        };
        let denied = HttpTransport::with_gate(&options, permission.clone());
        denied.send_envelope(event());
        assert!(denied.flush(Duration::from_secs(2)));
        assert_eq!(count.load(Ordering::Relaxed), 0);
        permission.change(true);
        let first = HttpTransport::with_gate(&options, permission.clone());
        first.send_envelope(event());
        assert!(first.flush(Duration::from_secs(2)));
        assert_eq!(count.load(Ordering::Relaxed), 1);
        permission.change(false);
        first.send_envelope(event());
        assert!(first.flush(Duration::from_secs(2)));
        assert_eq!(count.load(Ordering::Relaxed), 1);
        permission.change(true);
        first.send_envelope(event());
        assert!(first.flush(Duration::from_secs(2)));
        assert_eq!(count.load(Ordering::Relaxed), 1);
        let current = HttpTransport::with_gate(&options, permission);
        current.send_envelope(event());
        assert!(current.flush(Duration::from_secs(2)));
        assert_eq!(count.load(Ordering::Relaxed), 2);
        denied.shutdown(Duration::from_secs(1));
        first.shutdown(Duration::from_secs(1));
        current.shutdown(Duration::from_secs(1));
        stop.store(true, Ordering::Relaxed);
        server.join().unwrap();
    }
    #[test]
    fn no_delivery_without_consent_and_old_queues_never_replay() {
        let g = Gate::default();
        let count = AtomicUsize::new(0);
        assert!(!g.deliver(0, || {
            count.fetch_add(1, Ordering::Relaxed);
        }));
        g.change(true);
        let first = g.current().unwrap();
        assert!(g.deliver(first, || {
            count.fetch_add(1, Ordering::Relaxed);
        }));
        g.change(false);
        assert!(!g.deliver(first, || {
            count.fetch_add(1, Ordering::Relaxed);
        }));
        g.change(true);
        assert!(!g.deliver(first, || {
            count.fetch_add(1, Ordering::Relaxed);
        }));
        assert!(g.deliver(g.current().unwrap(), || {
            count.fetch_add(1, Ordering::Relaxed);
        }));
        assert_eq!(count.load(Ordering::Relaxed), 2);
    }
    #[test]
    fn revocation_waits_for_started_request_and_denies_following_requests() {
        let g = Arc::new(Gate::default());
        g.change(true);
        let epoch = g.current().unwrap();
        let (started, wait) = mpsc::channel();
        let (release, held) = mpsc::channel();
        let running = g.clone();
        let request = std::thread::spawn(move || {
            running.deliver(epoch, || {
                started.send(()).unwrap();
                held.recv_timeout(Duration::from_secs(2)).unwrap();
            })
        });
        wait.recv_timeout(Duration::from_secs(2)).unwrap();
        let revoking = g.clone();
        let (done, result) = mpsc::channel();
        let revoke = std::thread::spawn(move || {
            revoking.change(false);
            done.send(()).unwrap();
        });
        release.send(()).unwrap();
        assert!(request.join().unwrap());
        result.recv_timeout(Duration::from_secs(2)).unwrap();
        revoke.join().unwrap();
        assert!(!g.deliver(epoch, || panic!("sent after withdrawal")));
    }
}
