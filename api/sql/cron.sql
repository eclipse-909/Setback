do $$
begin
    -- delete expired data
    delete from profile_moderation_log where expires_at < now();
    delete from refresh_token          where expires_at < now();
    delete from stage_account          where expires_at < now();
    delete from stage_email_change     where expires_at < now();
    delete from stage_password_change  where expires_at < now();

    -- unsuspend profiles
    update profile set
        suspended_at    = null,
        suspended_until = null
    where suspended_until < now();

    -- remove old ip addresses
    delete from profile_ip
    where last_seen < now() + make_interval(years => 3);

    -- remove old games
    delete from game
    where ended_at < now() + make_interval(months => 1);

    -- remove resolved reports
    delete from player_report
    where resolved_at < now();
end $$;