use futures::future::Join;
use hyper::{Body, Request};
use pin_project::pin_project;
use std::future::Future;
use std::task::Poll;
use tower::Service;

use crate::to_boxed;
use crate::BoxedError;
use crate::Multiplexer;

/// A MakeService for [Multiplexer]
///
/// This type is used when more than one Multiplexer instance is needed
pub struct MakeMultiplexer<MakeGrpc, MakeWeb> {
	make_grpc: MakeGrpc,
	make_web: MakeWeb,
}

impl<MakeGrpc, MakeWeb> MakeMultiplexer<MakeGrpc, MakeWeb> {
	/// Move two make services into a new MakeService for Multiplexer
	pub fn new(make_grpc: MakeGrpc, make_web: MakeWeb) -> Self {
		MakeMultiplexer {
			make_grpc,
			make_web,
		}
	}
}
impl<Grpc, Web, GrpcError, WebError, MakeGrpc, MakeWeb, Target> Service<Target>
	for MakeMultiplexer<MakeGrpc, MakeWeb>
where
	MakeGrpc: Service<Target, Response = Grpc, Error = GrpcError>,
	MakeWeb: Service<Target, Response = Web, Error = WebError>,
	Grpc: Service<Request<Body>>,
	Web: Service<Request<Body>>,
	GrpcError: Into<BoxedError>,
	WebError: Into<BoxedError>,
	Target: Clone,
{
	type Response = Multiplexer<Grpc, Web>;

	type Error = BoxedError;

	type Future = MakeMultiplexerFuture<MakeGrpc::Future, MakeWeb::Future>;

	fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
		match (
			self.make_grpc.poll_ready(cx).map_err(to_boxed)?,
			self.make_web.poll_ready(cx).map_err(to_boxed)?,
		) {
			(Poll::Ready(_), Poll::Ready(_)) => Poll::Ready(Ok(())),
			(Poll::Ready(_), Poll::Pending) => Poll::Pending,
			(Poll::Pending, Poll::Ready(_)) => Poll::Pending,
			(Poll::Pending, Poll::Pending) => Poll::Pending,
		}
	}

	fn call(&mut self, req: Target) -> Self::Future {
		let make_grpc_future = self.make_grpc.call(req.clone());
		let make_web_future = self.make_web.call(req);
		MakeMultiplexerFuture::new(make_grpc_future, make_web_future)
	}
}

#[pin_project]
pub struct MakeMultiplexerFuture<MakeGrpcFuture, MakeWebFuture>
where
	MakeGrpcFuture: Future,
	MakeWebFuture: Future,
{
	#[pin]
	inner: Join<MakeGrpcFuture, MakeWebFuture>,
}
impl<MakeGrpcFuture, MakeWebFuture> MakeMultiplexerFuture<MakeGrpcFuture, MakeWebFuture>
where
	MakeGrpcFuture: Future,
	MakeWebFuture: Future,
{
	fn new(make_grpc_future: MakeGrpcFuture, make_web_future: MakeWebFuture) -> Self {
		let joined_future = futures::future::join(make_grpc_future, make_web_future);
		MakeMultiplexerFuture {
			inner: joined_future,
		}
	}
}

impl<MakeGrpcFuture, MakeWebFuture, MakeGrpcError, MakeWebError, Grpc, Web> Future
	for MakeMultiplexerFuture<MakeGrpcFuture, MakeWebFuture>
where
	MakeGrpcFuture: Future<Output = Result<Grpc, MakeGrpcError>>,
	MakeWebFuture: Future<Output = Result<Web, MakeWebError>>,
	MakeGrpcError: Into<BoxedError>,
	MakeWebError: Into<BoxedError>,
	Grpc: Service<Request<Body>>,
	Web: Service<Request<Body>>,
{
	type Output = Result<Multiplexer<Grpc, Web>, BoxedError>;

	fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
		let poll = self.project().inner.poll(cx);
		if let Poll::Ready(output) = poll {
			match output {
				(Ok(grpc), Ok(web)) => Poll::Ready(Ok(Multiplexer::new(grpc, web))),
				// (Ok(_), Err(web_error)) => Poll::Ready(Err(web_error.into())),
				// (Err(grpc_error), _) => Poll::Ready(Err(grpc_error.into())),
				// (Err(_), Err(_)) => todo!(), //Should return both errors some way
				_ => todo!(),
			}
		} else {
			Poll::Pending
		}
	}
}

#[cfg(test)]
mod tests {
	use std::convert::Infallible;

	use hyper::{service::service_fn, Body, Request, Response};
	use tower::Service;

	use super::MakeMultiplexer;

	#[test]
	fn make_multiplexer_receives_two_make_service_impl_make_service() {
		let make_grpc = tower::make::Shared::new(service_fn(|_req: Request<Body>| async {
			Ok::<_, Infallible>(Response::new(Body::from("service")))
		}));
		let make_web = tower::make::Shared::new(service_fn(|_req: Request<Body>| async {
			Ok::<_, Infallible>(Response::new(Body::from("service")))
		}));

		//Only ensure that constructor accepts these services
		let _make_multiplexer = MakeMultiplexer::new(make_grpc, make_web);
	}
	#[tokio::test]
	async fn make_multiplexer_is_a_make_service() {
		let make_grpc = tower::make::Shared::new(service_fn(|_req: Request<Body>| async {
			Ok::<_, String>(Response::new(Body::from("service")))
		}));
		let make_web = tower::make::Shared::new(service_fn(|_req: Request<Body>| async {
			Ok::<_, String>(Response::new(Body::from("service")))
		}));

		let mut make_multiplexer = MakeMultiplexer::new(make_grpc, make_web);
		use tower::make::MakeService;
		//Ensure make_multiplexer is a MakeService
		let _service = make_multiplexer.make_service(()).await.unwrap();
	}
	#[tokio::test]
	async fn use_make_multiplexer_as_service() {
		let make_grpc = tower::make::Shared::new(service_fn(|_req: Request<Body>| async {
			Ok::<_, String>(Response::new(Body::from("service")))
		}));
		let make_web = tower::make::Shared::new(service_fn(|_req: Request<Body>| async {
			Ok::<_, String>(Response::new(Body::from("service")))
		}));
		let mut make_multiplexer = MakeMultiplexer::new(make_grpc, make_web);
		use tower::ServiceExt;

		ServiceExt::<()>::ready(&mut make_multiplexer)
			.await
			.unwrap();
		make_multiplexer.call(()).await.unwrap();
	}
}
