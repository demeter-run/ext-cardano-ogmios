//! Buffering an HTTP request body far enough to classify it, then replaying
//! it to the upstream byte for byte.

use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use http_body_util::BodyExt;
use hyper::body::{Body, Frame, SizeHint};
use hyper::HeaderMap;

/// Reads `body` until it ends or the buffered bytes exceed `cap`, and returns
/// the buffered prefix together with a body that replays the prefix followed
/// by whatever remains of `body`, unchanged.
///
/// The prefix holds every byte when the body ended within the cap, and more
/// than `cap` bytes otherwise; it can exceed `cap` by up to one frame. An
/// error means the client aborted before the body was complete.
pub async fn buffer_body<B>(mut body: B, cap: usize) -> Result<(Bytes, ReplayBody<B>), B::Error>
where
    B: Body<Data = Bytes> + Unpin,
{
    let mut buffer = BytesMut::new();
    let mut trailers = None;
    let mut ended = true;

    while let Some(frame) = body.frame().await {
        match frame?.into_data() {
            Ok(data) => buffer.extend_from_slice(&data),
            Err(frame) => {
                // Trailers are the last frame of a body.
                trailers = frame.into_trailers().ok();
                break;
            }
        }
        if buffer.len() > cap {
            ended = false;
            break;
        }
    }

    let prefix = buffer.freeze();
    let replay = ReplayBody {
        prefix: (!prefix.is_empty()).then(|| prefix.clone()),
        rest: (!ended).then_some(body),
        trailers,
    };
    Ok((prefix, replay))
}

/// A request body that yields a buffered prefix, then the rest of the original
/// body, then any trailers already read.
#[derive(Debug)]
pub struct ReplayBody<B> {
    prefix: Option<Bytes>,
    rest: Option<B>,
    trailers: Option<HeaderMap>,
}

impl<B> Body for ReplayBody<B>
where
    B: Body<Data = Bytes> + Unpin,
{
    type Data = Bytes;
    type Error = B::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, B::Error>>> {
        if let Some(prefix) = self.prefix.take() {
            return Poll::Ready(Some(Ok(Frame::data(prefix))));
        }
        if let Some(rest) = self.rest.as_mut() {
            match Pin::new(rest).poll_frame(cx) {
                Poll::Ready(None) => self.rest = None,
                other => return other,
            }
        }
        Poll::Ready(
            self.trailers
                .take()
                .map(|trailers| Ok(Frame::trailers(trailers))),
        )
    }

    fn is_end_stream(&self) -> bool {
        self.prefix.is_none()
            && self.trailers.is_none()
            && match &self.rest {
                Some(rest) => rest.is_end_stream(),
                None => true,
            }
    }

    fn size_hint(&self) -> SizeHint {
        let prefix = self.prefix.as_ref().map_or(0, |prefix| prefix.len() as u64);
        let rest = self
            .rest
            .as_ref()
            .map_or_else(|| SizeHint::with_exact(0), |rest| rest.size_hint());

        let mut hint = SizeHint::new();
        hint.set_lower(rest.lower() + prefix);
        if let Some(upper) = rest.upper() {
            hint.set_upper(upper + prefix);
        }
        hint
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use futures_util::stream;
    use http_body_util::StreamBody;

    use super::*;

    const CAP: usize = 64 * 1024;

    type Chunk = Result<Frame<Bytes>, &'static str>;

    fn stub(chunks: Vec<Chunk>) -> StreamBody<stream::Iter<std::vec::IntoIter<Chunk>>> {
        StreamBody::new(stream::iter(chunks))
    }

    /// Splits `data` into frames of `frame` bytes.
    fn frames(data: &[u8], frame: usize) -> Vec<Chunk> {
        data.chunks(frame)
            .map(|chunk| Ok(Frame::data(Bytes::copy_from_slice(chunk))))
            .collect()
    }

    fn payload(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    async fn replayed<B>(body: B) -> Vec<u8>
    where
        B: Body<Data = Bytes>,
        B::Error: std::fmt::Debug,
    {
        body.collect().await.unwrap().to_bytes().to_vec()
    }

    #[tokio::test]
    async fn a_body_within_the_cap_is_buffered_whole_and_replayed() {
        for len in [0, 1, 4096, CAP - 1, CAP] {
            let data = payload(len);
            let (prefix, replay) = buffer_body(stub(frames(&data, 1000)), CAP).await.unwrap();
            assert_eq!(&prefix[..], &data[..], "len {len}");
            assert_eq!(replay.size_hint().exact(), Some(len as u64));
            assert_eq!(replayed(replay).await, data, "len {len}");
        }
    }

    #[tokio::test]
    async fn a_body_over_the_cap_replays_the_prefix_then_the_rest() {
        for (len, frame) in [(CAP + 1, 1000), (200 * 1024, 1000), (200 * 1024, 70_000)] {
            let data = payload(len);
            let (prefix, replay) = buffer_body(stub(frames(&data, frame)), CAP).await.unwrap();
            assert!(prefix.len() > CAP);
            assert!(prefix.len() <= CAP + frame);
            assert_eq!(&prefix[..], &data[..prefix.len()]);
            assert_eq!(replayed(replay).await, data, "len {len} frame {frame}");
        }
    }

    #[tokio::test]
    async fn trailers_are_replayed_after_the_data() {
        let mut trailers = HeaderMap::new();
        trailers.insert("x-check", "1".parse().unwrap());
        let chunks = vec![
            Ok(Frame::data(Bytes::from_static(b"{\"method\""))),
            Ok(Frame::data(Bytes::from_static(b":\"nextBlock\"}"))),
            Ok(Frame::trailers(trailers.clone())),
        ];

        let (prefix, replay) = buffer_body(stub(chunks), CAP).await.unwrap();
        assert_eq!(&prefix[..], b"{\"method\":\"nextBlock\"}");
        let collected = replay.collect().await.unwrap();
        assert_eq!(collected.trailers(), Some(&trailers));
        assert_eq!(&collected.to_bytes()[..], b"{\"method\":\"nextBlock\"}");
    }

    #[tokio::test]
    async fn a_client_abort_is_an_error() {
        let mut chunks = frames(&payload(10_000), 1000);
        chunks.push(Err("connection reset"));
        assert_eq!(
            buffer_body(stub(chunks), CAP).await.unwrap_err(),
            "connection reset"
        );
    }

    #[tokio::test]
    async fn an_abort_after_the_cap_reaches_the_replay() {
        let mut chunks = frames(&payload(CAP + 2000), 1000);
        chunks.push(Err("connection reset"));
        let (_, replay) = buffer_body(stub(chunks), CAP).await.unwrap();
        assert_eq!(replay.collect().await.unwrap_err(), "connection reset");
    }

    #[tokio::test]
    async fn an_empty_body_is_at_its_end() {
        let empty =
            http_body_util::Empty::<Bytes>::new().map_err(|never: Infallible| match never {});
        let (prefix, replay) = buffer_body(empty, CAP).await.unwrap();
        assert!(prefix.is_empty());
        assert!(replay.is_end_stream());
    }
}
