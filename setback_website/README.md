# setback_website
This is an API and file server in one REST app.
## Endpoints
Format:
- "/endpoint"
```
Request info

Response info
```
### File Server
- "/"
```
GET

Index page
```
- "/file_not_found"
```
GET

404 page
```
- "/downloads"
```
GET

Downloads page
```
- "/login"
```
GET

Login page
```
- "/account"
```
GET
Headers:
- Authorization: Bearer <token>

Account page
```
- "/css/\*.css"
```
GET

css file
```
- "/js/\*.js"
```
GET

js file
```
### API
- "/api/login"
```
POST
Body: LoginRequest
Headers:
- Authorization: Bearer <token>

Body: LoginResponse
```
- "/api/signup"
```
POST
Body: SingupRequest
Headers:
- Authorization: Bearer <token>

Body: SignupResponse
```
- "/api/logout"
```
GET
Headers:
- Authorization: Bearer <token>
```
- "/api/refresh"
```
POST
Headers:
- Authorization: Bearer <token>
Body: RefreshRequest
```
- "/api/id"
```
GET
Headers:
- Authorization: Bearer <token>

Body: IdResponse
```
- "/api/change_username"
```
PATCH
Headers:
- Authorization: Bearer <token>
Body: ChangeUsernameRequest
```
- "/api/change_password"
```
PATCH
Headers:
- Authorization: Bearer <token>
Body: ChangePasswordRequest
```
- "/api/delete_account"
```
DELETE
Headers:
- Authorization: Bearer <token>
```
- "/api/update"
```
GET

Body: UpdateResponse
```