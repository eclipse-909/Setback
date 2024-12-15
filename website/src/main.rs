mod session;
mod jwt;
mod api;
mod sql;

use {
	crate::{
		session::SessionValidator,
		jwt::JwtValidator,
		api::*,
		sql::{DB, Database}
	},
	actix_web::{
		App, HttpServer, HttpResponse,
		web::{self, Data},
		cookie::Key,
		dev::{fn_service, ServiceRequest, ServiceResponse},
		middleware::Logger
	},
	actix_files::{Files, NamedFile},
	actix_session::{
		SessionMiddleware,
		storage::CookieSessionStore,
		config::{CookieContentSecurity, PersistentSession, SessionLifecycle}
	},
	lazy_static::lazy_static,
	dotenv::from_filename,
	std::env
};

const SESSION_ACCOUNT_ID: &str = "account_id";

lazy_static! {
	static ref PRIVATE_KEY: Key = Key::generate();
}

#[macro_export]
macro_rules! client {
    ($path:expr) => {
        concat!("./website/client/", $path)
    };
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	// Set up database connection
	// MySql for release - Sqlite for debug
	//TODO uncomment when everthing is ready
	// if cfg!(debug_assertions) {
	// 	from_filename("./website/.env.release.debug").ok();
	// } else {
	// 	from_filename("./website/.env.release.release").ok();
	// }
	// let database: Database = Database::connect(env::var("DATABASE_URL").expect("DATABASE_URL is not set").as_str());
	//TODO database.create_if_not_exists();
	
	HttpServer::new(|| {
		App::new()
			//TODO uncomment when everthing is ready
			// .app_data(Data::new(database))
			.wrap(Logger::default())
			
			//Public pages
			.service(Files::new("/css", client!("css")))
			.service(Files::new("/js", client!("js")))
			.service(files("html").index_file("index.html"))
			//TODO uncomment when everthing is ready
			//Session-based protected pages and API
			// .service(
			// 	web::scope("/session")
			// 		.wrap(SessionMiddleware::builder(CookieSessionStore::default(), PRIVATE_KEY.to_owned())
			// 			.cookie_content_security(CookieContentSecurity::Private)
			// 			.session_lifecycle(SessionLifecycle::PersistentSession(PersistentSession::default()))
			// 			.cookie_secure(false)//TODO change to true when TSL and HTTPS is set up
			// 			.build()
			// 		)
			// 		.service(
			// 			web::scope("/account")
			// 				.wrap(SessionValidator)
			// 				.service(files("account_html").index_file("account.html"))
			// 		)
			// 		.service(
			// 			web::scope("/api")
			// 				.service(session_login)
			// 				.service(create_account)
			// 				.service(web::scope("/session")
			// 					.wrap(SessionValidator)
			// 					.service(update_username)
			// 					.service(update_password)
			// 					.service(delete_account)
			// 					.service(session_logout)
			// 					.service(renew_session)
			// 				)
			// 		)
			// )
			//
			// //JWT-based protected API
			// .service(
			// 	web::scope("/api")
			// 		.wrap(JwtValidator)
			// 		.service(jwt_login)
			// 		.service(renew_jwt)
			// )
	}).bind((if cfg!(debug_assertions) {"127.0.0.1"} else {"0.0.0.0"}, 8080))?.run().await
}

fn files(directory: &str) -> Files {
	Files::new("/", format!("./website/client/{}", directory).as_str())
		.default_handler(fn_service(|req: ServiceRequest| async {
			let (req, _) = req.into_parts();
			let res: HttpResponse = NamedFile::open_async(client!("html/file_not_found.html")).await?.into_response(&req);
			Ok(ServiceResponse::new(req, res))
		}))
}