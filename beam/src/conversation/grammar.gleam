//// Grammar — type vocabulary extraction from .conv source.
////
//// Lightweight string parsing to extract grammar blocks and their type
//// definitions. No full parser — just enough to build a type vocabulary
//// for assertion evaluation.

import gleam/dict.{type Dict}
import gleam/list
import gleam/set.{type Set}
import gleam/string

/// A parsed grammar's type vocabulary.
pub type Grammar {
  Grammar(domain: String, types: Dict(String, Set(String)))
}

/// Extract the domain name from a grammar.
pub fn domain(grammar: Grammar) -> String {
  grammar.domain
}

/// Check if a grammar contains a specific variant under a type path.
/// Empty type_path checks the default (unnamed) type.
pub fn has_variant(grammar: Grammar, type_path: String, variant: String) -> Bool {
  case dict.get(grammar.types, type_path) {
    Ok(variants) -> set.contains(variants, variant)
    Error(_) -> False
  }
}

/// Parse a grammar block from .conv source text.
/// Finds `grammar @name { ... }`, extracts type definitions.
pub fn from_source(source: String) -> Result(Grammar, String) {
  let lines = string.split(source, "\n")
  case find_grammar_start(lines) {
    Ok(#(domain_name, rest)) -> {
      let type_lines = collect_grammar_body(rest, 1)
      let types = parse_type_lines(type_lines)
      Ok(Grammar(domain: domain_name, types: types))
    }
    Error(msg) -> Error(msg)
  }
}

/// Find the `grammar @name {` or `abstract grammar @name {` line
/// and return the domain name + remaining lines.
fn find_grammar_start(
  lines: List(String),
) -> Result(#(String, List(String)), String) {
  case lines {
    [] -> Error("No grammar block found")
    [line, ..rest] -> {
      let trimmed = string.trim(line)
      let grammar_rest = case string.starts_with(trimmed, "abstract grammar @") {
        True -> Ok(string.drop_start(trimmed, 18))
        False ->
          case string.starts_with(trimmed, "grammar @") {
            True -> Ok(string.drop_start(trimmed, 9))
            False -> Error(Nil)
          }
      }
      case grammar_rest {
        Ok(after_at) -> {
          let without_brace =
            after_at
            |> string.replace(" {", "")
            |> string.replace("{", "")
            |> string.trim
          let domain_name = case string.split_once(without_brace, " extends ") {
            Ok(#(name, _)) -> string.trim(name)
            Error(_) -> without_brace
          }
          Ok(#(domain_name, rest))
        }
        Error(_) -> find_grammar_start(rest)
      }
    }
  }
}

/// Collect lines inside the grammar block, tracking brace depth.
fn collect_grammar_body(lines: List(String), depth: Int) -> List(String) {
  case lines {
    [] -> []
    _ if depth <= 0 -> []
    [line, ..rest] -> {
      let opens = count_char(line, "{")
      let closes = count_char(line, "}")
      let new_depth = depth + opens - closes
      case new_depth <= 0 {
        True -> []
        False -> [line, ..collect_grammar_body(rest, new_depth)]
      }
    }
  }
}

/// Count occurrences of a character in a string.
fn count_char(s: String, char: String) -> Int {
  let parts = string.split(s, char)
  list.length(parts) - 1
}

/// Parse type definition lines into a Dict(String, Set(String)).
fn parse_type_lines(lines: List(String)) -> Dict(String, Set(String)) {
  list.fold(lines, dict.new(), fn(acc, line) {
    let trimmed = string.trim(line)
    case string.starts_with(trimmed, "type") {
      True -> {
        case parse_type_line(trimmed) {
          Ok(#(name, variants)) -> dict.insert(acc, name, variants)
          Error(_) -> acc
        }
      }
      False -> acc
    }
  })
}

/// Parse a single type line.
/// `type = a | b | c` → ("", {"a", "b", "c"})
/// `type name = a | b` → ("name", {"a", "b"})
fn parse_type_line(line: String) -> Result(#(String, Set(String)), Nil) {
  let after_type = string.drop_start(line, 4) |> string.trim_start
  case string.split_once(after_type, "=") {
    Ok(#(before_eq, after_eq)) -> {
      let name = string.trim(before_eq)
      let variants =
        after_eq
        |> string.split("|")
        |> list.map(string.trim)
        |> list.filter(fn(s) { s != "" })
        |> set.from_list
      Ok(#(name, variants))
    }
    Error(_) -> Error(Nil)
  }
}
