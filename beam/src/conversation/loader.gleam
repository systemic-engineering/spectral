//// Loader — compiles ETF to BEAM modules and loads them.
////
//// Takes the ETF bytes from the Rust NIF (Erlang Abstract Format),
//// compiles them via compile:forms/1, and loads with code:load_binary/3.

/// Load ETF bytes as a BEAM module. Returns the module name on success.
@external(erlang, "loader_ffi", "load_etf_module")
pub fn load_etf_module(etf: BitArray) -> Result(String, String)

/// Check if a module is loaded on the BEAM.
@external(erlang, "loader_ffi", "is_loaded")
pub fn is_loaded(module: String) -> Bool

/// Get a loaded module's Lens dependencies.
@external(erlang, "loader_ffi", "get_lenses")
pub fn get_lenses(module: String) -> Result(List(String), String)

/// Get a loaded module's extends (parent domains).
@external(erlang, "loader_ffi", "get_extends")
pub fn get_extends(module: String) -> Result(List(String), String)

/// Get a loaded module's required properties.
@external(erlang, "loader_ffi", "get_requires")
pub fn get_requires(module: String) -> Result(List(String), String)

/// Get a loaded module's invariant properties.
@external(erlang, "loader_ffi", "get_invariants")
pub fn get_invariants(module: String) -> Result(List(String), String)

/// Get a loaded module's ensures (postcondition) properties.
@external(erlang, "loader_ffi", "get_ensures")
pub fn get_ensures(module: String) -> Result(List(String), String)

/// Purge and delete a loaded module from the BEAM.
/// Idempotent — safe to call even if the module was never loaded.
/// Use this to clean up after an enforcement failure.
@external(erlang, "loader_ffi", "purge_module")
pub fn purge_module(module: String) -> Nil
