import admin/views
import gleam/string_tree
import gleeunit
import gleeunit/should

pub fn main() {
  gleeunit.main()
}

pub fn dashboard_html_not_empty_test() {
  views.dashboard_html()
  |> string_tree.to_string
  |> fn(s) { s != "" }
  |> should.be_true
}

pub fn tokens_json_not_empty_test() {
  views.tokens_json()
  |> string_tree.to_string
  |> fn(s) { s != "" }
  |> should.be_true
}

pub fn token_json_not_empty_test() {
  views.token_json("background")
  |> string_tree.to_string
  |> fn(s) { s != "" }
  |> should.be_true
}

pub fn theme_json_not_empty_test() {
  views.theme_json()
  |> string_tree.to_string
  |> fn(s) { s != "" }
  |> should.be_true
}
