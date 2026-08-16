//! The poll loop.
//!
//! Owns nothing but timing, logging and reconnection: what to show is decided by
//! [`PresenceService`].

use jellyfin_rpc::{JfError, PresenceService, Tick};
use log::{debug, error, info, warn};
use retry::retry_with_index;
use std::thread::sleep;
use std::time::Duration;

pub struct Runner {
    service: PresenceService,
    interval: Duration,
    last_summary: Option<String>,
    /// Set while every server is unreachable, so the warning is logged once.
    servers_down: bool,
}

impl Runner {
    pub fn new(service: PresenceService, interval_secs: u64) -> Self {
        Self {
            service,
            interval: Duration::from_secs(interval_secs.max(1)),
            last_summary: None,
            servers_down: false,
        }
    }

    /// Connects to Discord, retrying with an exponential backoff.
    pub fn connect(&mut self) {
        info!("Connecting to Discord");
        self.retry("connect", |service| service.connect());
        info!("Connected!");
    }

    pub fn run(&mut self) -> ! {
        loop {
            sleep(self.interval);

            match self.service.tick() {
                Ok(tick) => self.report(tick),
                Err(err) => self.handle_error(err),
            }
        }
    }

    fn report(&mut self, tick: Tick) {
        match tick {
            Tick::Playing {
                summary,
                source,
                device,
            } => {
                if self.last_summary.as_deref() != Some(summary.as_str()) {
                    info!("[{}] {} ({})", source, summary, device);
                    self.last_summary = Some(summary);
                }
            }
            Tick::Idle | Tick::Hidden => {
                if self.last_summary.is_some() {
                    info!("Cleared activity");
                    self.last_summary = None;
                }
            }
        }
    }

    fn handle_error(&mut self, err: Box<dyn std::error::Error>) {
        // A blacklisted item is an expected outcome, not a failure.
        if matches!(err.downcast_ref::<JfError>(), Some(JfError::ContentBlacklist)) {
            debug!("{}", err);
            return;
        }

        // Jellyfin being unreachable says nothing about the Discord socket, so
        // keep polling instead of tearing the connection down.
        if matches!(
            err.downcast_ref::<JfError>(),
            Some(JfError::AllSourcesUnreachable)
        ) {
            if !self.servers_down {
                warn!("{} — retrying every {:?}", err, self.interval);
                self.servers_down = true;
            }
            return;
        }

        if self.servers_down {
            info!("Jellyfin reachable again");
            self.servers_down = false;
        }

        error!("{}", err);
        debug!("{:?}", err);

        self.last_summary = None;
        self.retry("reconnect", |service| service.reconnect());
        info!("Reconnected!");
    }

    fn retry<F>(&mut self, what: &str, mut op: F)
    where
        F: FnMut(&mut PresenceService) -> Result<(), Box<dyn std::error::Error>>,
    {
        let service = &mut self.service;

        let _ = retry_with_index(retry::delay::Exponential::from_millis(1000), |attempt| {
            info!("Attempt {}: trying to {}", attempt, what);
            match op(service) {
                Ok(()) => retry::OperationResult::<(), ()>::Ok(()),
                Err(err) => {
                    error!("{}", err);
                    retry::OperationResult::Retry(())
                }
            }
        });
    }
}
