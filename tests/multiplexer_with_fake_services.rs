use std::time::{Duration, Instant};

use hyper::{Body, Request};
use tower::{make::Shared, Service, ServiceExt};

mod common;
use common::svc;
use multiplex_tonic_hyper::{MakeMultiplexer, Multiplexer};

#[tokio::test]
async fn multiplexer_propagate_inner_error() {
	let ready = svc::ReadyService {};
	let error = svc::ErrorService {};

	assert!(Multiplexer::new(ready, error).ready().await.is_err());
	assert!(Multiplexer::new(error, ready).ready().await.is_err());
}

#[tokio::test]
async fn multiplexer_wait_until_all_inners_are_ready() {
	let until = Instant::now() + Duration::from_millis(10); //10ms should be enough
	let delayed = svc::DelayedService::new(until);
	let ready = svc::ReadyService {};

	let grpc_delayed = tokio::spawn(async move {
		let before = Instant::now();
		let mut multiplexer = Multiplexer::new(delayed, ready);
		multiplexer.ready().await.unwrap();
		let after = Instant::now();
		println!("grpc_delayed took: {:?}", after - before); //Just to debug
		after
	});
	let web_delayed = tokio::spawn(async move {
		let before = Instant::now();
		let mut multiplexer = Multiplexer::new(ready, delayed);
		multiplexer.ready().await.unwrap();
		let after = Instant::now();
		println!("web_delayed took: {:?}", after - before); //Just to debug
		after
	});

	let grpc_after = grpc_delayed.await.unwrap();
	let web_after = web_delayed.await.unwrap();
	assert!(
		grpc_after >= until,
		"should wait for more {:?}",
		(until - grpc_after)
	);
	assert!(
		web_after >= until,
		"should wait for more {:?}",
		(until - web_after)
	);
}

#[test]
fn multiplexer_accepts_any_http_body_as_web_body() {
	let grpc = svc::ReadyService {};
	// Web has a Response with a body different than hyper::Body
	let web = svc::HttpBodyService {};
	let service = Multiplexer::new(grpc, web);

	fn impl_service<S: Service<Request<Body>>>(_service: S) {}

	//This will only compile if multiplexer implements Service
	// The first implementation was restricted to Response<hyper::Body>
	impl_service(service);
}

#[test]
fn multiplexer_accepts_any_http_body_as_grpc_body() {
	// grpc has a Response with a body different than hyper::Body
	let grpc = svc::HttpBodyService {};
	let web = svc::ReadyService {};
	let service = Multiplexer::new(grpc, web);

	fn impl_service<S: Service<Request<Body>>>(_service: S) {}

	//This will only compile if multiplexer implements Service
	// The first implementation was restricted to Response<hyper::Body>
	impl_service(service);
}

#[tokio::test]
async fn make_multiplexer_service() {
	let grpc = Shared::new(svc::HttpBodyService {});
	let web = Shared::new(svc::ReadyService {});

	let make_service = MakeMultiplexer::new(grpc, web);

	//Try to use make service
	let service = make_service.oneshot(()).await.unwrap();

	service.oneshot(Request::new(Body::empty())).await.unwrap();
}

#[tokio::test]
async fn make_multiplexer_propagate_grpc_poll_fail() {
	let make_grpc = svc::FailingMakeService {};
	let make_web = svc::ReadyMakeService {};

	let mut make_service = MakeMultiplexer::new(make_grpc, make_web);
	let res = ServiceExt::<()>::ready(&mut make_service).await;
	assert!(res.is_err());
	let err = res.err().unwrap().to_string();
	assert_eq!(
		err,
		svc::FailingMakeService::get_err_string(),
		"Should return same error"
	);
}

#[tokio::test]
async fn make_multiplexer_inner_web_poll_fail() {
	let make_grpc = svc::ReadyMakeService {};
	let make_web = svc::FailingMakeService {};

	let mut make_service = MakeMultiplexer::new(make_grpc, make_web);
	let res = ServiceExt::<()>::ready(&mut make_service).await;
	assert!(res.is_err());
	let err = res.err().unwrap().to_string();
	assert_eq!(
		err,
		svc::FailingMakeService::get_err_string(),
		"Should return same error"
	);
}

#[tokio::test]
async fn make_multiplexer_both_poll_fail() {
	let make_grpc = svc::ReadyMakeService {};
	let make_web = svc::FailingMakeService {};

	let mut make_service = MakeMultiplexer::new(make_grpc, make_web);
	let res = ServiceExt::<()>::ready(&mut make_service).await;
	assert!(res.is_err());
}

#[tokio::test]
async fn make_multiplexer_delayed_grpc() {
	let until = Instant::now() + Duration::from_millis(10); //10ms should be enough
	let make_grpc = svc::DelayedMakeService::new(until);
	let make_web = svc::ReadyMakeService {};

	let mut make_service = MakeMultiplexer::new(make_grpc, make_web);

	let before = Instant::now();
	ServiceExt::<()>::ready(&mut make_service).await.unwrap();
	let after = Instant::now();
	println!("grpc_delayed took: {:?}", after - before);
	assert!(after >= until, "should wait for more {:?}", (until - after));
}

#[tokio::test]
async fn make_multiplexer_delayed_web() {
	let until = Instant::now() + Duration::from_millis(10); //10ms should be enough
	let make_grpc = svc::ReadyMakeService {};
	let make_web = svc::DelayedMakeService::new(until);

	let mut make_service = MakeMultiplexer::new(make_grpc, make_web);

	let before = Instant::now();
	ServiceExt::<()>::ready(&mut make_service).await.unwrap();
	let after = Instant::now();
	println!("web_delayed took: {:?}", after - before);
	assert!(after >= until, "should wait for more {:?}", (until - after));
}

#[tokio::test]
async fn make_multiplexer_delayed_both() {
	let until = Instant::now() + Duration::from_millis(10); //10ms should be enough
	let make_grpc = svc::DelayedMakeService::new(until);
	let make_web = svc::DelayedMakeService::new(until);

	let mut make_service = MakeMultiplexer::new(make_grpc, make_web);

	let before = Instant::now();
	ServiceExt::<()>::ready(&mut make_service).await.unwrap();
	let after = Instant::now();
	println!("delayed took: {:?}", after - before);
	assert!(after >= until, "should wait for more {:?}", (until - after));
}
