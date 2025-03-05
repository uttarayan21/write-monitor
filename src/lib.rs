#![cfg_attr(not(feature = "std"), no_std)]
//! `WriteMonitor` will wrap over a writer and monitor how many bytes are written to it.
//! This is useful for showing progress of writes
//! # Example
//! ```
//! use write_monitor::WriteMonitor;
//! use std::io::Write;
//! let mut buf = std::fs::File::create("somefile").unwrap();
//! let mut wm = WriteMonitor::new(buf);
//! let big_data = std::fs::read("Cargo.toml").unwrap();
//! let big_data_len = big_data.len();
//! let monitor = wm.monitor();
//! std::thread::spawn(move || {
//!     wm.write_all(&big_data).unwrap();
//! });
//! let mut last_written = 0;
//! while monitor.bytes_written() < big_data_len as u64 {
//!    let written = monitor.bytes_written();
//!    if written != last_written {
//!    println!("{} bytes written", written);
//!    last_written = written;
//!    }
//!  std::thread::sleep(std::time::Duration::from_millis(100));
//! }
//! ```

extern crate alloc;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};

#[cfg(any(feature = "futures", feature = "tokio"))]
use core::{pin::Pin, task::Poll};

#[cfg_attr(any(feature = "futures", feature = "tokio"), pin_project::pin_project)]
#[derive(Debug, Clone)]
pub struct WriteMonitor<W> {
    #[cfg_attr(any(feature = "futures", feature = "tokio"), pin)]
    inner: W,
    bytes_written: Arc<AtomicU64>,
}

impl<W> WriteMonitor<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            bytes_written: Arc::new(AtomicU64::new(0)),
        }
    }
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written.load(Ordering::Acquire)
    }

    pub fn monitor(&self) -> Monitor<'_> {
        Monitor {
            bytes_written: self.bytes_written.clone(),
            __marker: core::marker::PhantomData,
        }
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

#[derive(Debug, Clone)]
pub struct Monitor<'m> {
    bytes_written: Arc<AtomicU64>,
    __marker: core::marker::PhantomData<&'m WriteMonitor<()>>,
}


impl Monitor {
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written.load(Ordering::Acquire)
    }

    pub fn into_inner(self) -> Arc<AtomicU64> {
        self.bytes_written
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
#[cfg(feature = "tokio")]
impl<W: tokio::io::AsyncWrite + core::marker::Unpin> tokio::io::AsyncWrite for WriteMonitor<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
        buf: &[u8],
    ) -> core::task::Poll<std::io::Result<usize>> {
        let ah = self.project();
        let r = ah.inner.poll_write(cx, buf);
        if let Poll::Ready(Ok(n)) = r {
            ah.bytes_written.fetch_add(n as u64, Ordering::AcqRel);
        }
        r
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<std::io::Result<()>> {
        let ah = self.project();
        ah.inner.poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<std::io::Result<()>> {
        let ah = self.project();
        ah.inner.poll_shutdown(cx)
    }
}

#[cfg(feature = "futures")]
impl<W: futures::io::AsyncWrite + core::marker::Unpin> futures::io::AsyncWrite for WriteMonitor<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
        buf: &[u8],
    ) -> core::task::Poll<futures::io::Result<usize>> {
        let ah = self.project();
        let r = ah.inner.poll_write(cx, buf);
        if let Poll::Ready(Ok(n)) = r {
            ah.bytes_written.fetch_add(n as u64, Ordering::AcqRel);
        }
        r
    }
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<futures::io::Result<()>> {
        let ah = self.project();
        ah.inner.poll_flush(cx)
    }
    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<futures::io::Result<()>> {
        let ah = self.project();
        ah.inner.poll_close(cx)
    }
}

#[cfg(feature = "std")]
impl<W: std::io::Write> std::io::Write for WriteMonitor<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let r = std::io::Write::write(&mut self.inner, buf);
        if let Ok(n) = r {
            self.bytes_written.fetch_add(n as u64, Ordering::AcqRel);
        }
        r
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
