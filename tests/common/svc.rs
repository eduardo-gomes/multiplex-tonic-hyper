use std::{
	future::{ready, Future},
	pin::Pin,
	task::{Context, Poll},
	thread,
	time::Instant,
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

/// Service implementation with custom body type
#[derive(Clone)]
pub(crate) struct HttpBodyService {}
impl Service<Request<Body>> for HttpBodyService {
	type Response = Response<http_body::Empty<&'static [u8]>>;
	type Error = hyper::Error;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, _req: Request<Body>) -> Self::Future {
		Box::pin(async { Ok(Response::new(http_body::Empty::new())) })
	}
}

#[derive(Clone, Copy)]
pub(crate) struct DelayedService {
	ready_after: Instant,
}
impl DelayedService {
	pub(crate) fn new(ready_after: Instant) -> Self {
		DelayedService { ready_after }
	}
}

impl Service<Request<Body>> for DelayedService {
	type Response = Response<Body>;
	type Error = hyper::Error;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
		if Instant::now() >= self.ready_after {
			Poll::Ready(Ok(()))
		} else {
			let ready_after = self.ready_after;
			let waker = cx.waker().clone();
			thread::spawn(move || {
				let now = Instant::now();
				if now < ready_after {
					thread::sleep(ready_after - now);
				}
				waker.wake();
			});
			Poll::Pending
		}
	}

	fn call(&mut self, _req: Request<Body>) -> Self::Future {
		let res = Ok(Response::new(Body::empty()));
		Box::pin(async { res })
	}
}

#[derive(Clone, Copy)]
pub(crate) struct FailingMakeService {}
impl FailingMakeService {
	pub fn get_err_string() -> String {
		"This service fails!".into()
	}
}

impl<T> Service<T> for FailingMakeService {
	type Response = ReadyService;
	type Error = String;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Err(Self::get_err_string()))
	}

	fn call(&mut self, _req: T) -> Self::Future {
		Box::pin(ready(Ok(ReadyService {})))
	}
}

#[derive(Clone, Copy)]
pub(crate) struct ReadyMakeService {}

impl<T> Service<T> for ReadyMakeService {
	type Response = ReadyService;
	type Error = String;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, _req: T) -> Self::Future {
		Box::pin(ready(Ok(ReadyService {})))
	}
}

#[derive(Clone, Copy)]
pub(crate) struct FailingFutureMakeService {}
impl FailingFutureMakeService {
	pub fn get_err_string() -> String {
		"This service future fails!".into()
	}
}

impl<T> Service<T> for FailingFutureMakeService {
	type Response = ReadyService;
	type Error = String;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, _req: T) -> Self::Future {
		Box::pin(ready(Err(Self::get_err_string())))
	}
}

#[derive(Clone, Copy)]
pub(crate) struct DelayedMakeService {
	ready_after: Instant,
}
impl DelayedMakeService {
	pub(crate) fn new(ready_after: Instant) -> Self {
		DelayedMakeService { ready_after }
	}
}

impl<T> Service<T> for DelayedMakeService {
	type Response = ReadyService;
	type Error = String;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
		if Instant::now() >= self.ready_after {
			Poll::Ready(Ok(()))
		} else {
			let ready_after = self.ready_after;
			let waker = cx.waker().clone();
			thread::spawn(move || {
				let now = Instant::now();
				if now < ready_after {
					thread::sleep(ready_after - now);
				}
				waker.wake();
			});
			Poll::Pending
		}
	}

	fn call(&mut self, _req: T) -> Self::Future {
		Box::pin(ready(Ok(ReadyService {})))
	}
}
