mod api;
mod sql;

use {
	crate::{
		sql::Database,
		api::*
	},
	actix_web::{
		App, HttpServer, HttpResponse, Error,
		web::{self, Data},
		cookie::Key,
		dev::{forward_ready, fn_service, Service, ServiceRequest, ServiceResponse, Transform},
		body::EitherBody
	},
	actix_files::{Files, NamedFile},
	actix_session::{
		SessionExt, SessionMiddleware,
		storage::CookieSessionStore
	},
	std::future::{ready, Ready},
	futures_util::{FutureExt, future::LocalBoxFuture}
};

const SESSION_ACCOUNT_ID: &str = "account_id";

///"./website/src/client" + path
fn client(path: &str) -> String {format!("./website/client{}", path)}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	HttpServer::new(|| {App::new()
		.wrap(
			SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
				.cookie_secure(false)
				.build()
		)
		.app_data(Data::new(Database::connect()))
		.service(Files::new("/css", client("/css")))
		.service(Files::new("/js", client("/js")))
		.service(files("html").index_file("index.html"))
		.service(web::scope("/account")
			.wrap(SessionValidator)
			.service(files("account_html").index_file("account.html"))
		)
		.service(web::scope("/api")
			.service(login)
			.service(get_account)
			.service(web::scope("/account")
				.wrap(SessionValidator)
				.service(create_account)
				.service(update_email)
				.service(update_username)
				.service(update_password)
				.service(delete_account)
				.service(logout)
				.service(renew_session)
			)
		)
	}).bind(("127.0.0.1", 8080))?.run().await//TODO change to "0.0.0.0" for production
}

fn files(directory: &str) -> Files {
	Files::new("/", client(format!("/{}", directory).as_str()))
		.default_handler(fn_service(|req: ServiceRequest| async {
			let (req, _) = req.into_parts();
			let res: HttpResponse = NamedFile::open_async(client("/html/file_not_found.html")).await?.into_response(&req);
			Ok(ServiceResponse::new(req, res))
		}))
}

struct SessionValidator;
impl<S, B> Transform<S, ServiceRequest> for SessionValidator where S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static, S::Future: 'static, B: 'static {
	type Response = ServiceResponse<EitherBody<B>>;
	type Error = Error;
	type Transform = SessionValidatorMiddleware<S>;
	type InitError = ();
	type Future = Ready<Result<Self::Transform, Self::InitError>>;
	fn new_transform(&self, service: S) -> Self::Future {ready(Ok(SessionValidatorMiddleware {service}))}
}

struct SessionValidatorMiddleware<S> {service: S}
impl<S, B> Service<ServiceRequest> for SessionValidatorMiddleware<S> where S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static, S::Future: 'static, B: 'static {
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
			async move {
				let response: HttpResponse = NamedFile::open_async(client("/html/file_not_found.html")).await?.into_response(&request);
				Ok(ServiceResponse::new(request, response).map_into_right_body())
			}.boxed_local()
		}
	}
}