# setback_website
This is an API and file server in one REST app.
## Endpoints
### File Server
- "/" GET - index page
- "/file_not_found" GET - 404 page
- "/download" GET - download page
- "/login" GET - login page
- "/account" GET - account page - requires active session
- "/css/\*.css" GET - css file
- "/js/\*.js" GET - js file
### API
- "/api/login" POST - attempts to login
- "/api/signup" POST - attempts to create an account
- "/api/logout" GET - logs out from account
- "/api/logged_in" GET - returns whether the user is logged in
- "/api/change_username" PATCH - changes the username of the account
- "/api/change_password" PATCH - changes the password of the account
- "/api/delete_account" DELETE - deletes the account and logs out
- "/api/keep_alive" GET - keeps the session alive