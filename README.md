# Multiplex tonic hyper

-   Note: This is the first crate i made. Just publishing to try [crates.io](crates.io)

This implements a service that routes requests based on `Content-Type`. If starts with `application/grpc` it sends to the inner gRPC service. If it doesn't, then it sends to the other service.
