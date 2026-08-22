# Setback API
## Public Endpoints
Anyone can hit these endpoints

- POST api/signup
- POST api/login
- GET  api/healthcheck

## Per-Account Endpoints
These endpoints require an auth token

- GET  api/account/logout
- POST api/account/feedback

## Private & Per-Account Endpoints
These endpoints require an API key and auth token

- POST api/account/report
- GET  api/account/username