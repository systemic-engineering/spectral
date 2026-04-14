-module(stage@internal@message).
-compile([no_auto_import, nowarn_unused_vars, nowarn_unused_function, nowarn_nomatch, inline]).
-define(FILEPATH, "src/stage/internal/message.gleam").
-export_type([subscription_ref/1, producer_message/1, consumer_message/1]).

-if(?OTP_RELEASE >= 27).
-define(MODULEDOC(Str), -moduledoc(Str)).
-define(DOC(Str), -doc(Str)).
-else.
-define(MODULEDOC(Str), -compile([])).
-define(DOC(Str), -compile([])).
-endif.

?MODULEDOC(
    " Internal protocol messages for the stage pipeline.\n"
    "\n"
    " These messages flow between stage actors to implement the demand-driven\n"
    " backpressure protocol. They are not part of the public API.\n"
).

-type subscription_ref(FJJ) :: {subscription_ref,
        gleam@erlang@process:subject(consumer_message(FJJ)),
        gleam@erlang@process:subject(producer_message(FJJ)),
        gleam@erlang@process:pid_()}.

-type producer_message(FJK) :: {subscribe,
        gleam@erlang@process:subject(consumer_message(FJK)),
        gleam@erlang@process:subject({ok, subscription_ref(FJK)} |
            {error, binary()}),
        gleam@option:option(integer())} |
    {unsubscribe, gleam@erlang@process:subject(producer_message(FJK))} |
    {ask_demand, gleam@erlang@process:subject(producer_message(FJK)), integer()} |
    {emit, list(FJK)}.

-type consumer_message(FJL) :: {send_events,
        gleam@erlang@process:subject(producer_message(FJL)),
        list(FJL)}.


