use {
	crate::{
		sql::{DB, Database},
		jwt::{create_jwts, create_access_token, Claims, DECODING_KEY},
		SESSION_ACCOUNT_ID
	},
	resources::website_items::*,
	actix_web::{post, get, patch, delete, web, HttpResponse},
	actix_session::Session,
	serde::{Deserialize},
	jsonwebtoken::{decode, Validation}
};

#[derive(Deserialize)]
struct UsernameQuery {username: String}
#[derive(Deserialize)]
struct PasswordQuery {password: String}
#[derive(Deserialize)]
struct AuthQuery {username: String, password: String}

///Logs into an account.
#[post("/login")]
async fn session_login(session: Session, auth: web::Query<AuthQuery>, db: web::Data<Database>) -> HttpResponse {
	let auth: AuthQuery = auth.into_inner();
	let Ok(account_id) = db.verify_login(auth.username, auth.password).await else {return HttpResponse::Unauthorized().body("Incorrect username or password");};
	session.renew();
	if session.insert(SESSION_ACCOUNT_ID, account_id).is_ok() {
		HttpResponse::Ok().finish()
	} else {
		HttpResponse::InternalServerError().body("Could not establish session")
	}
}

///Logs into an account.
#[post("/login")]
async fn jwt_login(auth: web::Json<AuthQuery>, db: web::Data<Database>) -> HttpResponse {
	if db.verify_login(auth.username.clone(), auth.password.clone()).await.is_ok() {
		match create_jwts(auth.username.clone()) {
			Ok((access_token, refresh_token)) => {
				HttpResponse::Ok().json(serde_json::json!({
				    "access_token": access_token,
				    "refresh_token": refresh_token
				}))
			}
			Err(_) => {
				HttpResponse::InternalServerError().body("Token creation failed")
			}
		}
	} else {
		HttpResponse::Unauthorized().body("Invalid credentials")
	}
}

///Creates an account with the body of the post request and inserts it into the database.
#[post("/")]
async fn create_account(session: Session, user: web::Json<AccountData>, db: web::Data<Database>) -> HttpResponse {
	let user: AccountData = user.into_inner();
	match db.create_account(user.username, user.password).await {
		Ok(account_id) => {
			if session.insert(SESSION_ACCOUNT_ID, account_id).is_err() {
				HttpResponse::InternalServerError().body("Could not establish session")
			} else {
				HttpResponse::Ok().finish()
			}
		}
		Err(str) => {HttpResponse::InternalServerError().body(str.to_string())}
	}
}

///Updates the user's username
#[patch("/username")]
async fn update_username(session: Session, username: web::Query<UsernameQuery>, db: web::Data<Database>) -> HttpResponse {
	let Ok(Some(account_id)) = session.get(SESSION_ACCOUNT_ID) else {return HttpResponse::Unauthorized().body("Could not get session info");};
	if let Err(str) = db.update_username(account_id, username.into_inner().username).await {return HttpResponse::InternalServerError().body(str.to_string());}
	session.renew();
	HttpResponse::Ok().finish()
}

///Updates the user's password
#[patch("/password")]
async fn update_password(session: Session, password: web::Query<PasswordQuery>, db: web::Data<Database>) -> HttpResponse {
	let Ok(Some(account_id)) = session.get(SESSION_ACCOUNT_ID) else {return HttpResponse::Unauthorized().body("Could not get session info");};
	if let Err(str) = db.update_password(account_id, password.into_inner().password).await {return HttpResponse::InternalServerError().body(str.to_string());}
	session.renew();
	HttpResponse::Ok().finish()
}

///Deletes the user's account
#[delete("/")]
async fn delete_account(session: Session, db: web::Data<Database>) -> HttpResponse {
	let Ok(Some(account_id)) = session.get(SESSION_ACCOUNT_ID) else {return HttpResponse::Unauthorized().body("Could not get session info");};
	if let Err(str) = db.delete_account(account_id).await {return HttpResponse::InternalServerError().body(str.to_string());}
	session.renew();
	HttpResponse::Ok().finish()
}

///Logs out of an account.
#[get("/logout")]
async fn session_logout(session: Session) -> HttpResponse {
	session.purge();
	HttpResponse::Ok().finish()
}

///Keeps the session alive by renewing it
#[get("/renew")]
async fn renew_session(session: Session) -> HttpResponse {
	session.renew();
	HttpResponse::Ok().finish()
}

///Refereshes the JWT
#[post("/renew")]
async fn renew_jwt(refresh_token: web::Json<String>) -> HttpResponse {
	let token_data = decode::<Claims>(
		&refresh_token.into_inner(),
		&*DECODING_KEY,
		&Validation::default()
	);
	match token_data {
		Ok(data) => {
			if let Ok(new_access_token) = create_access_token(data.claims.sub) {
				HttpResponse::Ok().json(serde_json::json!({"access_token": new_access_token}))
			} else {
				HttpResponse::InternalServerError().body("Token creation failed")
			}
		}
		Err(_) => {HttpResponse::Unauthorized().body("Invalid refresh token")}
	}
}