use {
	serde::{Deserialize, Serialize},
	chrono::{Utc, Duration},
	jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation},
	actix_web::{
		dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
		Error, HttpMessage, HttpResponse,
		cookie::Key,
		body::EitherBody
	},
	futures_util::future::{ok, Ready, LocalBoxFuture},
	lazy_static::lazy_static
};

lazy_static! {
	static ref _key: Key = Key::generate();
	static ref ENCODING_KEY: EncodingKey = EncodingKey::from_secret(_key.master());
	pub static ref DECODING_KEY: DecodingKey = DecodingKey::from_secret(_key.master());
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
	///the user ID or unique identifier
	pub sub: String,
	///expiration time as a timestamp
	exp: usize
}

//Creates only the access token
pub fn create_access_token(user_id: String) -> jsonwebtoken::errors::Result<String> {
	encode(
		&Header::default(),
		&Claims {
			sub: user_id,
			exp: (Utc::now() + Duration::hours(2)).timestamp() as usize
		},
		&ENCODING_KEY
	)
}

//Creates the access token and refresh token, and returns them in a tuple in that order.
pub fn create_jwts(user_id: String) -> jsonwebtoken::errors::Result<(String, String)> {
	Ok((
		create_access_token(user_id.clone())?,
		encode(
			&Header::default(),
			&Claims {
				sub: user_id,
				exp: (Utc::now() + Duration::days(7)).timestamp() as usize
			},
			&ENCODING_KEY
		)?
	))
}

pub struct JwtValidator;

impl<S, B> Transform<S, ServiceRequest> for JwtValidator
where S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static, S::Future: 'static, B: 'static {
	type Response = ServiceResponse<EitherBody<B>>;
	type Error = Error;
	type Transform = JwtMiddleware<S>;
	type InitError = ();
	type Future = Ready<Result<Self::Transform, Self::InitError>>;
	fn new_transform(&self, service: S) -> Self::Future {ok(JwtMiddleware{service})}
}

pub struct JwtMiddleware<S> {service: S}

impl<S, B> Service<ServiceRequest> for JwtMiddleware<S>
where S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static, S::Future: 'static, B: 'static {
	type Response = ServiceResponse<EitherBody<B>>;
	type Error = Error;
	type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
	forward_ready!(service);
	fn call(&self, req: ServiceRequest) -> Self::Future {
		if let Some(token) = req.headers().get("Authorization").and_then(|h| h.to_str().ok()).and_then(|h| h.strip_prefix("Bearer ")) {
			if let Ok(decoded_token) = decode::<Claims>(token, &DECODING_KEY, &Validation::default()) {
				req.extensions_mut().insert(decoded_token.claims);
				let fut: <S as Service<ServiceRequest>>::Future = self.service.call(req);
				return Box::pin(async {Ok(fut.await?.map_into_left_body())});
			}
		}
		Box::pin(async {Ok(req.into_response(HttpResponse::Unauthorized()).map_into_right_body())})
	}
}