use bytes::Bytes;
use hyper::body::{Body, Frame, SizeHint};
use rain_core::artifacts::ArtifactStore;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use uuid::Uuid;
type Chunk = Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send>>;
pub(crate) struct ArtifactBody {
    store: ArtifactStore,
    id: Uuid,
    position: u64,
    end: u64,
    pending: Option<Chunk>,
    failed: bool,
}
impl ArtifactBody {
    pub fn new(store: ArtifactStore, id: Uuid, start: u64, length: u64) -> Self {
        Self {
            store,
            id,
            position: start,
            end: start + length,
            pending: None,
            failed: false,
        }
    }
}
impl Body for ArtifactBody {
    type Data = Bytes;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        if self.failed || self.position >= self.end {
            return Poll::Ready(None);
        }
        if self.pending.is_none() {
            let store = self.store.clone();
            let id = self.id;
            let position = self.position;
            let length = (self.end - position).min(1024 * 1024);
            self.pending = Some(Box::pin(
                async move { store.read(id, position, length).await },
            ));
        }
        match self.pending.as_mut().unwrap().as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(data)) => {
                self.pending = None;
                if data.is_empty() {
                    self.failed = true;
                    return Poll::Ready(Some(Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Shared file ended before its recorded size",
                    )
                    .into())));
                }
                self.position += data.len() as u64;
                Poll::Ready(Some(Ok(Frame::data(Bytes::from(data)))))
            }
            Poll::Ready(Err(error)) => {
                self.failed = true;
                Poll::Ready(Some(Err(std::io::Error::other(error).into())))
            }
        }
    }
    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.end - self.position)
    }
    fn is_end_stream(&self) -> bool {
        self.failed || self.position >= self.end
    }
}

pub(crate) fn byte_range(header: &str, size: u64) -> Result<(u64, u64), String> {
    let value = header
        .strip_prefix("bytes=")
        .ok_or("Only byte ranges are supported")?;
    if size == 0 || value.contains(',') {
        return Err("Range is unavailable".into());
    }
    let (start, end) = value.split_once('-').ok_or("Invalid byte range")?;
    if start.is_empty() {
        let count = end
            .parse::<u64>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or("Invalid suffix range")?;
        return Ok((size.saturating_sub(count), size - 1));
    }
    let start = start.parse::<u64>().map_err(|_| "Invalid range start")?;
    let end = if end.is_empty() {
        size - 1
    } else {
        end.parse::<u64>()
            .map_err(|_| "Invalid range end")?
            .min(size - 1)
    };
    if start >= size || end < start {
        return Err("Range is outside the file".into());
    }
    Ok((start, end))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges() {
        assert_eq!(byte_range("bytes=5-9", 10).unwrap(), (5, 9));
        assert_eq!(byte_range("bytes=-3", 10).unwrap(), (7, 9));
        assert_eq!(byte_range("bytes=2-", 10).unwrap(), (2, 9));
        for bad in ["bytes=10-", "bytes=4-1", "bytes=0-1,3-4", "bytes=-0"] {
            assert!(byte_range(bad, 10).is_err());
        }
    }
}
