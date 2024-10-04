use std::fmt::Debug;

use tower_http::classify::{GrpcErrorsAsFailures, SharedClassifier};
use tower_http::trace::{
    DefaultMakeSpan, DefaultOnBodyChunk, DefaultOnEos, OnFailure, OnRequest,
    OnResponse, TraceLayer,
};
use tracing::level_filters::LevelFilter;

const HEALTH_PATH: &str = "/grpc.health.v1.Health/Check";
const REFLECTION_PATH: &str =
    "/grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo";

#[derive(Debug, Clone, Default)]
pub struct LogOnRequest {}

impl<B> OnRequest<B> for LogOnRequest {
    fn on_request(
        &mut self,
        request: &tonic::codegen::http::Request<B>,
        _span: &tracing::Span,
    ) {
        if request.uri().path() == HEALTH_PATH
            || request.uri().path() == REFLECTION_PATH
        {
            return;
        }

        if LevelFilter::current() == LevelFilter::DEBUG {
            tracing::debug!(
                target: "grpc-request",
                "{:?} {} {} {:?}",
                request.version(),
                request.method(),
                request.uri(),
                request.headers()
            );
        } else {
            tracing::info!(
                target: "grpc-request",
                "{:?} {} {}",
                request.version(),
                request.method(),
                request.uri()
            )
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LogOnResponse {}

impl<B> OnResponse<B> for LogOnResponse {
    fn on_response(
        self,
        response: &tonic::codegen::http::Response<B>,
        _latency: std::time::Duration,
        _span: &tracing::Span,
    ) {
        if response.status().is_success() {
            return;
        }

        tracing::debug!(
            target: "grpc-response",
            "{:?} {} {:?}",
            response.version(),
            response.status(),
            response.headers(),
        );
    }
}

#[derive(Debug, Clone, Default)]
pub struct LogOnFailure {}

impl<B> OnFailure<B> for LogOnFailure
where
    B: Debug,
{
    fn on_failure(
        &mut self,
        failure_classification: B,
        _latency: std::time::Duration,
        _span: &tracing::Span,
    ) {
        tracing::log::error!(
            target: "grpc-failure",
            "{:?}",
            failure_classification,
        );
    }
}

pub fn init_trace_layer() -> TraceLayer<
    SharedClassifier<GrpcErrorsAsFailures>,
    DefaultMakeSpan,
    LogOnRequest,
    LogOnResponse,
    DefaultOnBodyChunk,
    DefaultOnEos,
    LogOnFailure,
> {
    TraceLayer::new_for_grpc()
        .on_request(LogOnRequest::default())
        .on_response(LogOnResponse::default())
        .on_failure(LogOnFailure::default())
}
