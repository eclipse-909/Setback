use {
	crate::constants::*,
	std::env,
	lettre::{
		Message, AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
		message::header::ContentType,
		transport::smtp::authentication::Credentials,
	}
};

pub struct Email {
	pub to: String,
	pub subject: String,
	pub body: String,
}

impl Email {
	pub async fn send(self) -> Result<(), Box<dyn std::error::Error>> {
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
		
		mailer.send(email).await?;
		Ok(())
	}
}