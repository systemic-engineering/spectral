-module(stage@error).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/stage/error.gleam").
-export_type([stage_error/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Error types for stage operations.\n"
    "\n"
    " Value types that carry meaning, not strings. The error tells you\n"
    " *what* failed. OTP logging tells you *why*.\n"
).

-type stage_error() :: start_failed | {subscribe_failed, binary()} | timeout.


