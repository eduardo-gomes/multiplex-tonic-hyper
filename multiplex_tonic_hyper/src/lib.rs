use std::{
	future::{ready, Ready},
	task::Poll,
};

use hyper::{Body, Request};
use tower::Service;

/// Service that routes to a gRPC service and other service
///
/// This service checks the Content-Type header, and send all requests
/// with `application/grpc` to the grpc service, and all the others requests
/// to the web service.
pub struct Multiplexer<Grpc, Web> {
	grpc: Grpc,
	web: Web,
}
impl<Grpc, Web> Multiplexer<Grpc, Web>
where
	Grpc: Service<Request<Body>>,
	Web: Service<Request<Body>>,
{
	pub fn new(grpc: Grpc, web: Web) -> Self {
		Multiplexer { grpc, web }
	}
}

impl<Grpc, Web> Service<Request<Body>> for Multiplexer<Grpc, Web>
where
	Grpc: Service<Request<Body>>,
	Web: Service<Request<Body>>,
{
	type Response = ();
	///Any Error
	type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
	type Future = Ready<Result<Self::Response, Self::Error>>;

	fn poll_ready(
		&mut self,
		_cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: Request<Body>) -> Self::Future {
		ready(Ok(()))
	}
}

#[cfg(test)]
mod tests {
	use std::{convert::Infallible, future::ready};

	use crate::Multiplexer;
	use hyper::{service::service_fn, Body, Request, Response};
	use tower::ServiceExt; // ServiceExt provides ready()

	//This test only checks if this compiles
	#[test]
	fn new_multiplex_receives_two_services() {
		let generate_service = |string: &'static str| {
			service_fn(|_req: Request<Body>| {
				ready(Ok::<Response<Body>, Infallible>(Response::new(Body::from(
					string.to_owned(),
				))))
			})
		};
		let service_1 = generate_service("Service 1");
		let service_2 = generate_service("Service 2");

		let _multiplex = Multiplexer::new(service_1, service_2);
	}

	#[tokio::test]
	async fn new_multiplex_is_ready() {
		let generate_service = |string: &'static str| {
			service_fn(|_req: Request<Body>| {
				ready(Ok::<Response<Body>, Infallible>(Response::new(Body::from(
					string.to_owned(),
				))))
			})
		};
		let grpc = generate_service("gRPC service");
		let web = generate_service("web service");

		let mut multiplex = Multiplexer::new(grpc, web);

		multiplex.ready().await.unwrap();
	}
}
