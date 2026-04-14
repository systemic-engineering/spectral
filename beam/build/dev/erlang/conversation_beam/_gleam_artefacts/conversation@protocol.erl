-module(conversation@protocol).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/conversation/protocol.gleam").
-export_type([spec/0, arm/0, pattern/0, op/0]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Protocol types — the contract between conversation AST and BEAM runtime.\n"
    "\n"
    " A conversation specifies desired BEAM state. The runtime converges toward it.\n"
    " These types define what the conversation says. The runtime decides how to get there.\n"
).

-type spec() :: {'case', binary(), list(arm())} |
    {branch, list(arm())} |
    {'when', op(), binary(), binary(), spec()} |
    {desired_state, binary(), binary()} |
    pass.

-type arm() :: {arm, pattern(), spec()}.

-type pattern() :: {cmp, op(), binary()} | wildcard.

-type op() :: gt | lt | gte | lte | eq | ne.


