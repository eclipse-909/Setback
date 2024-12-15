use {
	crate::{client, SESSION_ACCOUNT_ID},
	std::future::{ready, Ready},
	actix_files::NamedFile,
	actix_web::{
		Error, HttpResponse,
		body::EitherBody,
		dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform}
	},
	actix_session::SessionExt,
	futures_util::future::LocalBoxFuture
};

pub struct SessionValidator;

impl<S, B> Transform<S, ServiceRequest> for SessionValidator
where S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static, S::Future: 'static, B: 'static {
	type Response = ServiceResponse<EitherBody<B>>;
	type Error = Error;
	type Transform = SessionValidatorMiddleware<S>;
	type InitError = ();
	type Future = Ready<Result<Self::Transform, Self::InitError>>;
	fn new_transform(&self, service: S) -> Self::Future {ready(Ok(SessionValidatorMiddleware {service}))}
}

pub struct SessionValidatorMiddleware<S> {service: S}

impl<S, B> Service<ServiceRequest> for SessionValidatorMiddleware<S>
where S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static, S::Future: 'static, B: 'static {
	type Response = ServiceResponse<EitherBody<B>>;
	type Error = Error;
	type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
	forward_ready!(service);
	fn call(&self, req: ServiceRequest) -> Self::Future {
		if req.get_session().get::<u32>(SESSION_ACCOUNT_ID).unwrap_or(None).is_some() {
			let fut: <S as Service<ServiceRequest>>::Future = self.service.call(req);
			Box::pin(async move {Ok(fut.await?.map_into_left_body())})
		} else {
			let (request, _) = req.into_parts();
			Box::pin(async {
				let response: HttpResponse = NamedFile::open_async(client!("/html/file_not_found.html")).await?.into_response(&request);
				Ok(ServiceResponse::new(request, response).map_into_right_body())
			})
		}
	}
}