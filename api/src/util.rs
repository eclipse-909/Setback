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
	to: String,
	subject: String,
	body: String,
}

pub async fn send_email(email: Email) -> Result<(), Box<dyn std::error::Error>> {
	let host = env::var(ENV_NO_REPLY_EMAIL_HOST)?;
	let user = env::var(ENV_NO_REPLY_EMAIL_USER)?;
	let pass = env::var(ENV_NO_REPLY_EMAIL_PASS)?;
	
	// Sending to yourself: same address for From and To.
	let email = Message::builder()
		.from(format!("No Reply <{user}>").parse()?)
		.to(email.to.parse()?)
		.subject(email.subject)
		.header(ContentType::TEXT_PLAIN)
		.body(email.body)?;
	
	let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)?
		.credentials(Credentials::new(user, pass))
		.build();
	
	mailer.send(email).await?;
	Ok(())
}