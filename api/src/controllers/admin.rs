use {
	crate::{middleware::*, models::*, status::*, util::*},
	axum::{Extension, Json, extract::State},
	chrono::Utc,
	setback::*,
	sqlx::PgPool,
	utoipa_axum::{router::OpenApiRouter as Router, routes},
};

type Result<T> = core::result::Result<T, Status>;

#[utoipa::path(
	post,
	path = "/suspend",
	summary = "Update suspention status of an account",
	description = "An admin can suspend, reinstate, or warn an account",
	request_body = SuspendRequest,
	responses(
		(status = 200),
		(status = 500),
	),
	tag = "admin"
)]
async fn api_suspend(
	State(db): State<PgPool>,
	Extension(claims): Extension<Claims>,
	Json(suspend): Json<SuspendRequest>,
) -> Result<()> {
	struct Account {
		email: String,
		username: String,
	}
	match suspend.action {
		ModAction::Suspend => {
			let suspend_until = Utc::now() + suspend.duration;
			let Account {email, username} = sqlx::query_as!(
				Account,
				r#" with target as (
						select a.profile_id, a.username, e.email_normalized
						from account a
						inner join profile_email e using (profile_id)
						where a.account_id = $1
					),
					upd_sspd as (
						update profile p set
							suspended_at = now(),
							suspended_until = $2
						from target t
						where t.profile_id = p.profile_id
						returning 1
					),
					ins_mod_log as (
						insert into profile_moderation_log(profile_id, action, reason, acted_by)
						select profile_id, $3::mod_action, $4, $5 from target
					)
					select
					    email_normalized as "email!",
					    username as "username!"
					from target
				"#,
				suspend.account_id as i32,
				suspend_until,
				suspend.action as ModAction,
				suspend.reason,
				claims.account_id,
			).fetch_one(&db).await?;
			
			let email = Email {
				to: email,
				subject: "Setback - Account suspended".to_owned(),
				body: format!(
					"Hello, {username},\n\n\
					Your account has been suspended until {suspend_until}.\n\n\
					Reason given:\n{}",
					suspend.reason
				),
			};
			email.send().await?;
		}
		ModAction::Reinstate => {
			let Account {email, username} = sqlx::query_as!(
				Account,
				r#" with target as (
						select a.profile_id, a.username, e.email_normalized
						from account a
						inner join profile_email e using (profile_id)
						where a.account_id = $1
					),
					upd_sspd as (
						update profile p set
							suspended_at = null,
							suspended_until = null
						from target t
						where t.profile_id = p.profile_id
						returning 1
					),
					ins_mod_log as (
						insert into profile_moderation_log(profile_id, action, reason, acted_by)
						select profile_id, $2::mod_action, $3, $4 from target
					)
					select
					    email_normalized as "email!",
					    username as "username!"
					from target
				"#,
				suspend.account_id as i32,
				suspend.action as ModAction,
				suspend.reason,
				claims.account_id,
			).fetch_one(&db).await?;
			
			let email = Email {
				to: email,
				subject: "Setback - Account reinstated".to_owned(),
				body: format!(
					"Hello, {username},\n\n\
					Your account has been reinstated.\n\n\
					Reason given:\n{}",
					suspend.reason
				),
			};
			email.send().await?;
		}
		ModAction::Warn => {
			let Account {email, username} = sqlx::query_as!(
				Account,
				r#" with target as (
						select a.profile_id, a.username, e.email_normalized
						from account a
						inner join profile_email e using (profile_id)
						where a.account_id = $1
					),
					ins_mod_log as (
						insert into profile_moderation_log(profile_id, action, reason, acted_by)
						select profile_id, $2::mod_action, $3, $4 from target
					)
					select
					    email_normalized as "email!",
					    username as "username!"
					from target
				"#,
				suspend.account_id as i32,
				suspend.action as ModAction,
				suspend.reason,
				claims.account_id,
			).fetch_one(&db).await?;
			
			let email = Email {
				to: email,
				subject: "Setback - Account warning".to_owned(),
				body: format!(
					"Hello, {username},\n\n\
					You are receiving a warning.\n\n\
					Reason given:\n{}",
					suspend.reason
				),
			};
			email.send().await?;
		}
	}
	Ok(())
}

// TODO we could make an admin dashboard and other things

pub fn api_admin_routes(state: AppState) -> Router<AppState> {
	Router::new()
		.routes(routes!(api_suspend))
		.route_layer(axum::middleware::from_fn_with_state(state.clone(), jwt::require_admin))
}