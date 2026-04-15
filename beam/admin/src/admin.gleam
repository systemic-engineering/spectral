/// spectral admin — garden design system admin panel.
///
/// A native Gleam web server that serves the token graph admin interface.
/// Routes:
///   GET  /              — admin dashboard (HTML)
///   GET  /tokens        — token list (JSON)
///   GET  /tokens/:name  — single token detail (JSON)
///   GET  /theme         — current theme state (JSON)
///   PUT  /theme         — update theme axes (JSON)
///   GET  /health        — health check (JSON)

import gleam/erlang/process
import mist
import wisp
import admin/router

pub fn main() {
  let secret_key_base = wisp.random_string(64)
  let handler = router.handle_request(_, secret_key_base)

  let assert Ok(_) =
    handler
    |> mist.new
    |> mist.port(3000)
    |> mist.start_http

  process.sleep_forever()
}
