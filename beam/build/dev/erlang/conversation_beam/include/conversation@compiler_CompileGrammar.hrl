-record(compile_grammar, {
    source :: binary(),
    reply :: gleam@erlang@process:subject({ok,
            conversation@trace:trace(conversation@compiler:compiled_domain())} |
        {error, binary()})
}).
