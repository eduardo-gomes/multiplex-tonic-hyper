#![warn(missing_docs)]

//! Crate to route requests between a tonic gRPC service, and some other service
//!
//! The [Multiplexer] struct implements Service<Request<Body>>, and routes
//! requests based on the Content-Type header.

use std::{future::Future, task::Poll};

use hyper::{body::HttpBody, Body, Request, Response};
use pin_project::pin_project;
use tower::Service;

/// Service that routes to a gRPC service and other service
///
/// This service checks the Content-Type header, and send all requests
/// with `application/grpc` to the grpc service, and all other requests
/// to the web service.
///
/// # Examples:
///
/// Routing to the web service:
/// ```
/// # async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
/// # use std::convert::Infallible;
///	# use multiplex_tonic_hyper::Multiplexer;
/// use hyper::{header::CONTENT_TYPE, service::service_fn, Body, Request, Response};
///	use tower::{Service, ServiceExt};
///	async fn str_to_res(str: &'static str) -> Result<Response<Body>, Infallible> {
///		Ok(Response::new(Body::from(str)))
///	}
///
/// //Services that answer every request with a word
///	let grpc = service_fn(|_| str_to_res("gRPC"));
///	let web = service_fn(|_| str_to_res("web"));
///
///	let mut multiplex = Multiplexer::new(grpc, web);
/// # /// We must check if service is ready before call. See [tower::Service]
///	# multiplex.ready().await?;
///	//Request web without content-type header
///	let response = multiplex.call(Request::new(Body::empty())).await?;
///	let content = hyper::body::to_bytes(response.into_body()).await?;
/// assert_eq!(content, "web");
/// # Ok(())
/// # }
/// # tokio_test::block_on(run()).unwrap();
/// ```
///
/// Routing to the gRPC service:
/// ```
/// # async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
/// # use std::convert::Infallible;
/// # use hyper::{header::CONTENT_TYPE, service::service_fn, Body, Request, Response};
///	# use multiplex_tonic_hyper::Multiplexer;
///	# use tower::{Service, ServiceExt};
///	# async fn str_to_res(str: &'static str) -> Result<Response<Body>, Infallible> {
///	# 	Ok(Response::new(Body::from(str)))
///	# }
///	//...
///	let grpc = service_fn(|_| str_to_res("gRPC"));
///	let web = service_fn(|_| str_to_res("web"));
///
///	let mut multiplex = Multiplexer::new(grpc, web);
/// # // We must check if service is ready before call. See [tower::Service]
///	# multiplex.ready().await?;
///	//Request grpc using content-type header
///	let request = Request::builder()
///		.header(CONTENT_TYPE, "application/grpc")
///		.body(Body::empty())?;
///	let response = multiplex.call(request).await?;
///	let content = hyper::body::to_bytes(response.into_body()).await?;
/// assert_eq!(content, "gRPC");
/// # Ok(())
/// # }
/// # tokio_test::block_on(run()).unwrap();
/// ```
pub struct Multiplexer<Grpc, Web> {
	grpc: Grpc,
	web: Web,
}
impl<Grpc, Web> Multiplexer<Grpc, Web>
where
	Grpc: Service<Request<Body>>,
	Web: Service<Request<Body>>,
{
	///This function consumes two Services, and returns a Multiplexer
	pub fn new(grpc: Grpc, web: Web) -> Self {
		Multiplexer { grpc, web }
	}
}
type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;
fn to_boxed<T: Into<BoxedError>>(e: T) -> BoxedError {
	e.into()
}
impl<Grpc, Web, GrpcBody, WebBody> Service<Request<Body>> for Multiplexer<Grpc, Web>
where
	//Each type is a Service<> with its own Body type
	Grpc: Service<Request<Body>, Response = Response<GrpcBody>>,
	Web: Service<Request<Body>, Response = Response<WebBody>>,
	GrpcBody: HttpBody,
	WebBody: HttpBody,
	//Inner errors can be converted to our error type
	Grpc::Error: Into<BoxedError>,
	Web::Error: Into<BoxedError>,
{
	type Response = Response<EncapsulatedBody<GrpcBody, WebBody>>;
	///Generic error that can be moved between threads
	type Error = BoxedError;
	type Future = EncapsulatedFuture<Grpc::Future, Web::Future>;

	///Call inner services poll_ready, and propagate errors.
	/// Only is ready if both are ready.
	fn poll_ready(
		&mut self,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		//There is no problem in calling poll_ready if is Ready, and the docs don't have any limitation on pending
		let grpc = self.grpc.poll_ready(cx).map_err(to_boxed)?;
		let web = self.web.poll_ready(cx).map_err(to_boxed)?;
		match (grpc, web) {
			(Poll::Ready(_), Poll::Ready(_)) => Poll::Ready(Ok(())),
			_ => Poll::Pending,
		}
	}

	fn call(&mut self, req: Request<Body>) -> Self::Future {
		let is_grpc = req
			.headers()
			.get("content-type")
			.map(|x| x.as_bytes().starts_with(b"application/grpc"))
			.unwrap_or_default();
		if is_grpc {
			EncapsulatedFuture::Grpc(self.grpc.call(req))
		} else {
			EncapsulatedFuture::Web(self.web.call(req))
		}
	}
}

/// Type to encapsulate both inner services Futures
///
///Because [poll(cx)][Future::poll] uses a pinned mutable reference,
/// this enum needs to project the pin.
/// That way is possible to call poll on the inner future
#[pin_project(project = EncapsulatedProj)]
pub enum EncapsulatedFuture<GrpcFuture, WebFuture> {
	///Encapsulates a future from Grpc service
	Grpc(#[pin] GrpcFuture),
	///Encapsulates a future from Web service
	Web(#[pin] WebFuture),
}
/// This implementation should map the response and the error from the inner futures
///
/// The response has its body mapped to another enum, the enum should implement `HttpBody`
///
impl<GrpcFuture, WebFuture, GrpcResponseBody, WebResponseBody, GrpcError, WebError> Future
	for EncapsulatedFuture<GrpcFuture, WebFuture>
where
	GrpcFuture: Future<Output = Result<Response<GrpcResponseBody>, GrpcError>>,
	WebFuture: Future<Output = Result<Response<WebResponseBody>, WebError>>,
	GrpcError: Into<BoxedError>,
	WebError: Into<BoxedError>,
{
	/// We should output `Result<Response<impl HttpBody>, Multiplexer::Error>`
	type Output = Result<Response<EncapsulatedBody<GrpcResponseBody, WebResponseBody>>, BoxedError>;

	fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
		match self.project() {
			EncapsulatedProj::Grpc(future) => future
				.poll(cx)
				.map_ok(EncapsulatedBody::map_grpc)
				.map_err(to_boxed),
			EncapsulatedProj::Web(future) => future
				.poll(cx)
				.map_ok(EncapsulatedBody::map_web)
				.map_err(to_boxed),
		}
	}
}

/// Type to encapsulate both inner services HttpBody types
///
/// Because [poll_data(cx)][HttpBody::poll_data] and [poll_trailers(cx)][HttpBody::poll_trailers] uses pinned reference, this enum needs to project the pin.
///
/// This enum is used as the body type in [EncapsulatedFuture].
#[pin_project(project = BodyProj)]
pub enum EncapsulatedBody<GrpcBody, WebBody> {
	///Encapsulates the body from Grpc service
	Grpc(#[pin] GrpcBody),
	///Encapsulates the body from Web service
	Web(#[pin] WebBody),
}
impl<GrpcBody, WebBody> EncapsulatedBody<GrpcBody, WebBody> {
	fn map_grpc(response: Response<GrpcBody>) -> Response<Self> {
		response.map(EncapsulatedBody::Grpc)
	}
	fn map_web(response: Response<WebBody>) -> Response<Self> {
		response.map(EncapsulatedBody::Web)
	}
}
fn into_data<T: Into<hyper::body::Bytes>>(data: T) -> hyper::body::Bytes {
	data.into()
}
impl<GrpcBody, WebBody, GrpcError, WebError> HttpBody for EncapsulatedBody<GrpcBody, WebBody>
where
	GrpcBody: HttpBody<Error = GrpcError>,
	WebBody: HttpBody<Error = WebError>,
	GrpcBody::Error: Into<BoxedError>,
	WebBody::Error: Into<BoxedError>,
	GrpcBody::Data: Into<hyper::body::Bytes>,
	WebBody::Data: Into<hyper::body::Bytes>,
{
	type Data = hyper::body::Bytes;

	type Error = BoxedError;

	fn poll_data(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Option<Result<Self::Data, Self::Error>>> {
		match self.project() {
			BodyProj::Grpc(body) => body.poll_data(cx).map_ok(into_data).map_err(to_boxed),
			BodyProj::Web(body) => body.poll_data(cx).map_ok(into_data).map_err(to_boxed),
		}
	}

	fn poll_trailers(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Result<Option<hyper::HeaderMap>, Self::Error>> {
		match self.project() {
			BodyProj::Grpc(body) => body.poll_trailers(cx).map_err(to_boxed),
			BodyProj::Web(body) => body.poll_trailers(cx).map_err(to_boxed),
		}
	}
}

#[cfg(test)]
mod tests {
	use std::{convert::Infallible, future::ready};

	use crate::{EncapsulatedBody, Multiplexer};
	use hyper::{
		body::HttpBody, header::CONTENT_TYPE, service::service_fn, Body, HeaderMap, Request,
		Response,
	};
	use tower::{Service, ServiceExt}; // ServiceExt provides ready()

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

	// #[ignore = "While EncapsulatedBody is not implemented"]
	#[tokio::test]
	async fn multiplexer_request_to_web() {
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
		{
			//Request web
			let request = Request::new(Body::empty());
			let response = multiplex.call(request).await.unwrap();
			let content = hyper::body::to_bytes(response.into_body()).await.unwrap();

			assert_ne!(content.len(), 0);
			assert_eq!(content, "web service");
		}
		multiplex.ready().await.unwrap();
		{
			//Request grpc
			let request = Request::builder()
				.header(CONTENT_TYPE, "application/grpc")
				.body(Body::empty())
				.unwrap();
			let response = multiplex.call(request).await.unwrap();
			let content = hyper::body::to_bytes(response.into_body()).await.unwrap();

			assert_ne!(content.len(), 0);
			assert_eq!(content, "gRPC service");
		}
	}

	#[tokio::test]
	async fn encapsulated_body_poll_data_grpc() {
		let string = "body grpc";
		let body = EncapsulatedBody::<Body, Body>::Grpc(Body::from(string));

		let data = hyper::body::to_bytes(body).await.unwrap();
		assert_eq!(data, string);
	}

	#[tokio::test]
	async fn encapsulated_body_poll_data_web() {
		let string = "body web";
		let body = EncapsulatedBody::<Body, Body>::Grpc(Body::from(string));

		let data = hyper::body::to_bytes(body).await.unwrap();
		assert_eq!(data, string);
	}

	#[tokio::test]
	async fn encapsulated_body_poll_trailers_grpc() {
		let (mut sender, body) = Body::channel();
		let mut header_map = HeaderMap::new();
		header_map.insert("From", "grpc sender".parse().unwrap());
		let header_map = header_map;
		sender.send_trailers(header_map.clone()).await.unwrap();

		let mut body = EncapsulatedBody::<Body, Body>::Grpc(body);

		let headers = body
			.trailers()
			.await
			.unwrap()
			.expect("Should return trailers!");
		assert_eq!(headers, header_map);
	}

	#[tokio::test]
	async fn encapsulated_body_poll_trailers_web() {
		let (mut sender, body) = Body::channel();
		let mut header_map = HeaderMap::new();
		header_map.insert("From", "web sender".parse().unwrap());
		let header_map = header_map;
		sender.send_trailers(header_map.clone()).await.unwrap();

		let mut body = EncapsulatedBody::<Body, Body>::Web(body);

		let headers = body
			.trailers()
			.await
			.unwrap()
			.expect("Should return trailers!");
		assert_eq!(headers, header_map);
	}
}
