use {
	crate::{
		sql::*,
		SESSION_ACCOUNT_ID
	},
	resources::website_items::*,
	actix_web::{post, get, patch, delete, web, HttpResponse},
	actix_session::Session,
	serde::{Deserialize}
};

#[derive(Deserialize)]
struct UsernameQuery {username: String}
#[derive(Deserialize)]
struct PasswordQuery {password: String}
#[derive(Deserialize)]
struct EmailQuery {email: String}
#[derive(Deserialize)]
struct AuthQuery {username: String, password: String}

///Creates an account with the body of the post request and inserts it into the database.
#[post("/")]
async fn create_account(session: Session, user: web::Json<AccountData>, db: web::Data<Database>) -> HttpResponse {
	match create_account_helper(user, db) {
		Ok(account_id) => {
			if session.insert(SESSION_ACCOUNT_ID, account_id).is_err() {
				HttpResponse::InternalServerError().body("Could not establish session")
			} else {
				HttpResponse::Ok().finish()
			}
		}
		Err(str) => {HttpResponse::InternalServerError().body(str)}
	}
}

fn create_account_helper(user: web::Json<AccountData>, db: web::Data<Database>) -> Result<u32, String> {
	let user: AccountData = user.into_inner();
	
	todo!("Verify the email is valid");
	
	db.create_account(user.email, user.username, user.password)
}

///Fetches the account with the user_id from the database.
#[get("/user/{account_id}")]
async fn get_account(account_id: web::Path<u32>, db: web::Data<Database>) -> HttpResponse {
	match db.get_account(account_id.into_inner()) {
		Ok(data) => {HttpResponse::Ok().json(data)}
		Err(_) => {HttpResponse::InternalServerError().body("Could not get account")}
	}
}

///Updates the user's email
#[patch("/email")]
async fn update_email(session: Session, email: web::Query<EmailQuery>, db: web::Data<Database>) -> HttpResponse {
	
	todo!("Verify the email is valid");
	
	let Ok(Some(account_id)) = session.get(SESSION_ACCOUNT_ID) else {return HttpResponse::Unauthorized().body("Could not get session info");};
	if let Err(str) = db.update_email(account_id, email.into_inner().email) {return HttpResponse::InternalServerError().body(str);}
	session.renew();
	HttpResponse::Ok().finish()
}

///Updates the user's username
#[patch("/username")]
async fn update_username(session: Session, username: web::Query<UsernameQuery>, db: web::Data<Database>) -> HttpResponse {
	let Ok(Some(account_id)) = session.get(SESSION_ACCOUNT_ID) else {return HttpResponse::Unauthorized().body("Could not get session info");};
	if let Err(str) = db.update_username(account_id, username.into_inner().username) {return HttpResponse::InternalServerError().body(str);}
	session.renew();
	HttpResponse::Ok().finish()
}

///Updates the user's password
#[patch("/password")]
async fn update_password(session: Session, password: web::Query<PasswordQuery>, db: web::Data<Database>) -> HttpResponse {
	let Ok(Some(account_id)) = session.get(SESSION_ACCOUNT_ID) else {return HttpResponse::Unauthorized().body("Could not get session info");};
	if let Err(str) = db.update_password(account_id, password.into_inner().password) {return HttpResponse::InternalServerError().body(str);}
	session.renew();
	HttpResponse::Ok().finish()
}

///Deletes the user's account
#[delete("/")]
async fn delete_account(session: Session, db: web::Data<Database>) -> HttpResponse {
	let Ok(Some(account_id)) = session.get(SESSION_ACCOUNT_ID) else {return HttpResponse::Unauthorized().body("Could not get session info");};
	if let Err(str) = db.delete_account(account_id) {return HttpResponse::InternalServerError().body(str);}
	session.renew();
	HttpResponse::Ok().finish()
}

///Logs into an account.
#[post("/login")]
async fn login(session: Session, auth: web::Query<AuthQuery>, db: web::Data<Database>) -> HttpResponse {
	let auth: AuthQuery = auth.into_inner();
	let Ok(account_id) = db.verify_login(auth.username, auth.password) else {return HttpResponse::Unauthorized().body("Incorrect username or password");};
	session.renew();
	if session.insert(SESSION_ACCOUNT_ID, account_id).is_err() {
		HttpResponse::InternalServerError().body("Could not establish session")
	} else {
		HttpResponse::Ok().json(AccountResponse{account_id})
	}
}

///Logs out of an account.
#[get("/logout")]
async fn logout(session: Session) -> HttpResponse {
	session.purge();
	HttpResponse::Ok().finish()
}

///Keeps the session alive by renewing it
#[get("/renew")]
async fn renew_session(session: Session) -> HttpResponse {
	session.renew();
	HttpResponse::Ok().finish()
}