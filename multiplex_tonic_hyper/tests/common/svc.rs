use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

use hyper::{Body, Request, Response};
use tower::Service;

#[derive(Clone, Copy)]
pub(crate) struct ErrorService {}

impl Service<Request<Body>> for ErrorService {
	type Response = Response<Body>;
	type Error = String;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Err("This service always error".into()))
	}

	fn call(&mut self, _req: Request<Body>) -> Self::Future {
		let res = Ok(Response::new(Body::empty()));
		Box::pin(async { res })
	}
}
#[derive(Clone, Copy)]
pub(crate) struct ReadyService {}

impl Service<Request<Body>> for ReadyService {
	type Response = Response<Body>;
	type Error = hyper::Error;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, _req: Request<Body>) -> Self::Future {
		let res = Ok(Response::new(Body::empty()));
		Box::pin(async { res })
	}
}
