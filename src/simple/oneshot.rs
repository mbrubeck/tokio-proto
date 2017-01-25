//! One-shot RPC protocols.
//!
//! See the crate-level docs for an overview.

// Re-export the pipelined client traits, because only server behavior needs to differ.
pub use super::pipeline::ClientProto;
pub use super::pipeline::ClientService;

pub use self::server::ServerProto;

mod server {
    use std::io;

    use pipeline;

    use futures::{self, stream, Stream, Sink, Future, IntoFuture};

    /// A one-shot server protocol.
    ///
    /// The `T` parameter is used for the I/O object used to communicate, which is
    /// supplied in `bind_transport`.
    ///
    /// For simple protocols, the `Self` type is often a unit struct. In more
    /// advanced cases, `Self` may contain configuration information that is used
    /// for setting up the transport in `bind_transport`.
    pub trait ServerProto<T: 'static>: 'static {
        /// Request messages.
        type Request: 'static;

        /// Response messages.
        type Response: 'static;

        /// The message transport, which works with I/O objects of type `T`.
        ///
        /// An easy way to build a transport is to use `tokio_core::io::Framed`
        /// together with a `Codec`; in that case, the transport type is
        /// `Framed<T, YourCodec>`. See the crate docs for an example.
        type Transport: 'static +
            Stream<Item = Self::Request, Error = io::Error> +
            Sink<SinkItem = Self::Response, SinkError = io::Error>;

        /// A future for initializing a transport from an I/O object.
        ///
        /// In simple cases, `Result<Self::Transport, Self::Error>` often suffices.
        type BindTransport: IntoFuture<Item = Self::Transport, Error = io::Error>;

        /// Build a transport from the given I/O object, using `self` for any
        /// configuration.
        ///
        /// An easy way to build a transport is to use `tokio_core::io::Framed`
        /// together with a `Codec`; in that case, `bind_transport` is just
        /// `io.framed(YourCodec)`. See the crate docs for an example.
        fn bind_transport(&self, io: T) -> Self::BindTransport;
    }

    // Use `Stream::take` to create a "pipelined" protocol whose stream ends after a single
    // response, closing the connection.
    impl<T: 'static, P: ServerProto<T>> pipeline::ServerProto<T> for P {
        type Request = P::Request;
        type Response = P::Response;

        type Transport = stream::Take<P::Transport>;
        type BindTransport = futures::Map<<P::BindTransport as IntoFuture>::Future,
                                          fn(P::Transport) -> Self::Transport>;

        fn bind_transport(&self, io: T) -> Self::BindTransport {
            ServerProto::bind_transport(self, io).into_future().map(take_one)
        }
    }

    // Helper fn so we can write the type of pipeline::ServerProto::BindTransport above.
    fn take_one<S: Stream>(stream: S) -> stream::Take<S> {
        stream.take(1)
    }
}
