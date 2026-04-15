/// Views — HTML and JSON response bodies.
///
/// Skeleton responses. Real data comes from gen_prism operations on the token
/// graph once the runtime is wired.

import gleam/string_tree.{type StringTree}

/// Admin dashboard HTML.
pub fn dashboard_html() -> StringTree {
  string_tree.from_string(
    "<!DOCTYPE html>
<html>
<head><title>spectral admin</title></head>
<body>
  <h1>spectral admin</h1>
  <nav>
    <a href=\"/tokens\">Tokens</a> |
    <a href=\"/theme\">Theme</a> |
    <a href=\"/health\">Health</a>
  </nav>
  <p>Garden design system admin panel.</p>
</body>
</html>",
  )
}

/// Token list as JSON.
pub fn tokens_json() -> StringTree {
  // Skeleton: empty token list. Will be populated via gen_prism:focus.
  string_tree.from_string("{\"tokens\":[]}")
}

/// Single token detail as JSON.
pub fn token_json(name: String) -> StringTree {
  string_tree.from_string(
    "{\"name\":\"" <> name <> "\",\"value\":null}",
  )
}

/// Current theme state as JSON.
pub fn theme_json() -> StringTree {
  string_tree.from_string(
    "{\"mode\":\"light\",\"density\":\"comfortable\",\"contrast\":4.5,\"scale\":1.0,\"motion\":\"full\"}",
  )
}
