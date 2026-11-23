### Added: a login form for delegated authentication (`DelegatedAuth::login_form`)

- **API:** `DelegatedAuth::new(COOKIE, validate).login_form(|form| async { … }).login_page(|error| el(…))`.
  `login_form` claims `POST /login`: the closure receives the submitted
  `LoginForm` (`field("password")`…) and returns the session value to store —
  the authority's token — or the message to show. rwire sets that cookie
  (`Path=/; HttpOnly; SameSite=Lax`) and redirects to `next` (query string or
  hidden field, only when it is a path on the app), else `/`. A refusal
  re-renders the page with the message.
- `GET`/`POST /logout` clears the cookie and sends back to `/login`, so an app
  no longer needs a second server for its login page. `on_logout(|value| async)`
  receives the session value first, so the authority can retire it (a hub
  deleting its own session) instead of leaving it valid until it expires; the
  cookie is cleared whatever the hook answers, after at most
  `LOGOUT_HOOK_TIMEOUT` (3 s).
- Anonymous page requests get the login page with a `200` instead of a redirect
  to `login_url`; WebSocket handshakes are still refused (`401`), and a session
  the validator refuses keeps the same fail-closed behaviour.
- `login_page` draws the page with the design system; `capsule::serve_app_page`
  renders any app-drawn page with the app theme.
- Without `login_form`, `login_url` and the built-in login card behave exactly
  as before.
