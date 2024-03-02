platform "false-interpreter"
    requires {} { main : Str -> Task {} [] }
    exposes []
    packages {}
    imports [Task.{ Task }]
    provides [mainForHost]
             ## Test doc comment again
mainForHost : Str -> Task {} []
mainForHost = \file -> main file
