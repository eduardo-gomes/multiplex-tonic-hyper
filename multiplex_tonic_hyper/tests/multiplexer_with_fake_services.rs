use std::time::{Duration, Instant};

use tower::ServiceExt;

mod common;
use common::svc;
use multiplex_tonic_hyper::Multiplexer;

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