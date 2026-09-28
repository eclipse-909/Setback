# Setback API
## Public Endpoints
Anyone can hit these endpoints.

- GET  api/healthcheck
- POST api/signup
- POST api/login

## Per-Account Endpoints
These endpoints require an auth token.

- POST   api/account/feedback
- GET    api/account/logout
- PATCH  api/account/reset-password
- DELETE api/account/delete-account

## Private Endpoints
These endpoints require an API key and auth token.
Typically, the server will use these for collecting
metrics and diagnostics.

 - TODO api/server/todo

## Private & Per-Account Endpoints
These endpoints require an API key and auth token.
Typically, the server makes these requests on behalf of a user.

- POST api/account/server/report-player
- GET  api/account/server/username