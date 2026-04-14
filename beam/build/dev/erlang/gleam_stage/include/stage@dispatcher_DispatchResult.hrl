-record(dispatch_result, {
    assignments :: list({any(), list(any())}),
    buffered :: list(any()),
    state :: stage@dispatcher:dispatcher_state(any())
}).
