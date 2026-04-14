-record('when', {
    op :: conversation@protocol:op(),
    path :: binary(),
    literal :: binary(),
    then :: conversation@protocol:spec()
}).
