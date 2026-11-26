use {
	crate::{middleware::*, models::*, status::*},
	axum::{Json, extract::State, Extension},
	chrono::Utc,
	http::StatusCode,
	setback::*,
	sqlx::PgPool,
	utoipa_axum::{router::OpenApiRouter as Router, routes},
};

type Result<T> = core::result::Result<T, Status>;

#[utoipa::path(
	post,
	path = "/suspend",
	summary = "Suspends an account",
	description = "An admin can suspend an account",
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
	match suspend.action {
		ModAction::Suspend => {
			let suspend_until = Utc::now() + suspend.duration;
			sqlx::query!(
				r#" with prof_id as (
						update profile set
							suspended_at = now(),
							suspended_until = $1
						from account a
						where a.account_id = $2 and profile.profile_id = a.profile_id
						returning a.profile_id
					)
					insert into profile_moderation_log(profile_id, action, reason, acted_by)
					select profile_id, $3::mod_action, $4, $5 from prof_id
				"#,
				suspend_until,
				suspend.account_id as i32,
				suspend.action as ModAction,
				suspend.reason,
				claims.account_id,
			).execute(&db).await?;
		}
		ModAction::Reinstate => {
			sqlx::query!(
				r#" with prof_id as (
						update profile set
							suspended_at = null,
							suspended_until = null
						from account a
						where a.account_id = $1 and profile.profile_id = a.profile_id
						returning a.profile_id
					)
					insert into profile_moderation_log(profile_id, action, reason, acted_by)
					select profile_id, $2::mod_action, $3, $4 from prof_id
				"#,
				suspend.account_id as i32,
				suspend.action as ModAction,
				suspend.reason,
				claims.account_id,
			).execute(&db).await?;
		}
		ModAction::Warn => {
			sqlx::query!(
				r#" insert into profile_moderation_log(profile_id, action, reason, acted_by)
					select profile_id, $2::mod_action, $3, $4 from account a
					where a.account_id = $1
				"#,
				suspend.account_id as i32,
				suspend.action as ModAction,
				suspend.reason,
				claims.account_id,
			).execute(&db).await?;
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