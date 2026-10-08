# Setback API
## Public Endpoints
Anyone can hit these endpoints.

- GET  /api/healthcheck
- POST /api/begin-signup
- POST /api/finish-signup
- POST /api/login

## Per-Account Endpoints
These endpoints require a bearer auth token.

- GET    /api/account/refresh-token
- GET    /api/account/username
- POST   /api/account/feedback
- GET    /api/account/logout
- GET    /api/account/begin-reset-password
- PATCH  /api/account/finish-reset-password
- DELETE /api/account/delete-account

## Private Endpoints
These endpoints require an API key.
Typically, the server will use these for collecting diagnostics.
The server may also call endpoints for a specific player.

- POST  /api/server/{account_id}/report-player
- PATCH /api/server/{account_id}/stats

## Developer Endpoints
These require a bearer auth token and the user must be an admin.

- TODO /api/admin/todo

## Environment Email
You must provide values for these variables in order for emails to work.
```env
# example for gmail
SETBACK_NO_REPLY_EMAIL_HOST=smtp.gmail.com
SETBACK_NO_REPLY_EMAIL_USER=example@gmail.com
SETBACK_NO_REPLY_EMAIL_PASS=abcdefghijklmnop
```