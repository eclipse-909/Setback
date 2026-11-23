use {
	crate::{constants::*, status::*},
	std::env,
	lettre::{
		Message, AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
		message::header::ContentType,
		transport::smtp::authentication::Credentials,
	},
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