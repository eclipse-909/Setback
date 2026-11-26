use sqlx::{Executor, Postgres};
use {
	crate::{constants::*, status::*},
	http::HeaderMap,
	lettre::{
		AsyncSmtpTransport, AsyncTransport,
		Message, Tokio1Executor,
		message::header::ContentType,
		transport::smtp::authentication::Credentials,
	},
	sqlx::types::ipnet::IpNet,
	std::{env, net::{IpAddr, SocketAddr}},
	tracing::error
};

pub struct Email {
	pub to: String,
	pub subject: String,
	pub body: String,
}

impl Email {
	pub async fn send(self) -> Result<(), Status> {
		let create_mailer = || {
			let host = env::var(ENV_NO_REPLY_EMAIL_HOST)?;
			let user = env::var(ENV_NO_REPLY_EMAIL_USER)?;
			let pass = env::var(ENV_NO_REPLY_EMAIL_PASS)?;
			
			// Sending to yourself: same address for From and To.
			let email = Message::builder()
				.from(format!("No Reply <{user}>").parse()?)
				.to(self.to.parse()?)
				.subject(self.subject)
				.header(ContentType::TEXT_PLAIN)
				.body(self.body)?;
			
			let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)?
				.credentials(Credentials::new(user, pass))
				.build();
			Ok((mailer, email))
		};
		
		let (mailer, email) = create_mailer()
			.map_err(|e: Box<dyn std::error::Error>| {
				error!("Failed to send email: {e}");
				Status::s500()
			})?;
		mailer.send(email)
			.await
			.map_err(|_| Status::s400("Invalid email address"))?;
		Ok(())
	}
}

pub fn client_ip(headers: &HeaderMap, peer: SocketAddr) -> IpNet {
	headers
		.get("x-forwarded-for")
		.and_then(|v| v.to_str().ok())
		// rightmost entry = the one appended by YOUR proxy (not client-controlled)
		.and_then(|v| v.rsplit(',').next())
		.and_then(|s| s.trim().parse::<IpAddr>().ok())
		.unwrap_or_else(|| peer.ip()) // fallback: direct connection
		.into()
}

/// Returns bool for whether the account exists
pub async fn update_account_activity<'e, 'c, E>(
	account_id: i32,
	profile_id: i32,
	ip_addr: IpNet,
	executor: E,
) -> Result<bool, sqlx::Error>
where
	'c: 'e,
	E: 'e + Executor<'c, Database = Postgres>,
	Postgres: 'e,
{
	let exists = sqlx::query_scalar!(
		r#" with ups_ip as (
				insert into profile_ip(profile_id, ip, first_seen, last_seen)
				values ($2, $3::inet, now(), now())
				on conflict(profile_id, ip) do update set last_seen = now()
				returning 1
			)
			update account
			set last_active = now()
			where account_id = $1 and deleted_at is not null
			returning 1
		"#,
		account_id,
		profile_id,
		ip_addr,
	).fetch_one(executor).await?;
	Ok(exists.is_some())
}