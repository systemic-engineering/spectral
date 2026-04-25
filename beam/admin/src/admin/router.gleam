/// Router — dispatches requests to handlers.
///
/// Routes map to gen_prism operations on the token graph:
///   GET  /tokens       → focus(token_graph) → list of token nodes
///   GET  /tokens/:name → zoom(token_graph, name) → single token detail
///   PUT  /theme        → refract(theme_update) → rematerialized tokens
///   GET  /health       → status check

import gleam/http.{Get, Put}
import gleam/string_tree
import wisp.{type Request, type Response}
import admin/views

/// Handle an incoming HTTP request.
pub fn handle_request(req: Request, _secret: String) -> Response {
  case wisp.path_segments(req) {
    // GET / — dashboard
    [] -> dashboard()

    // GET /health — health check
    ["health"] -> health()

    // GET /tokens — list all tokens
    ["tokens"] ->
      case req.method {
        Get -> list_tokens()
        _ -> wisp.method_not_allowed([Get])
      }

    // GET /tokens/:name — single token
    ["tokens", name] ->
      case req.method {
        Get -> get_token(name)
        _ -> wisp.method_not_allowed([Get])
      }

    // GET/PUT /theme — theme state
    ["theme"] ->
      case req.method {
        Get -> get_theme()
        Put -> update_theme()
        _ -> wisp.method_not_allowed([Get, Put])
      }

    // 404
    _ -> wisp.not_found()
  }
}

fn dashboard() -> Response {
  let body = views.dashboard_html()
  wisp.html_response(body, 200)
}

fn health() -> Response {
  let body = string_tree.from_string("{\"status\":\"ok\"}")
  wisp.json_response(body, 200)
}

fn list_tokens() -> Response {
  let body = views.tokens_json()
  wisp.json_response(body, 200)
}

fn get_token(name: String) -> Response {
  let body = views.token_json(name)
  wisp.json_response(body, 200)
}

fn get_theme() -> Response {
  let body = views.theme_json()
  wisp.json_response(body, 200)
}

fn update_theme() -> Response {
  // TODO: parse request body, update theme via gen_prism:refract
  let body = string_tree.from_string("{\"status\":\"updated\"}")
  wisp.json_response(body, 200)
}
