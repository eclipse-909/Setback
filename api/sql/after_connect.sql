do $$
begin
    -- if the server crashes it won't set ended_at for active games
    -- This makes sure all games have ended_at set
    if to_regclass('game') is not null then
        update game set ended_at = current_timestamp where ended_at is null;
    end if;
end $$;