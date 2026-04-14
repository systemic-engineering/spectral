-record(producer_consumer_stage, {
    consumer_subject :: gleam@erlang@process:subject(stage@producer_consumer:p_c_message(any(), any())),
    producer_sub :: gleam@erlang@process:subject(stage@internal@message:producer_message(any()))
}).
