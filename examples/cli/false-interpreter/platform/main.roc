platform "false-interpreter-test"
    requires {} { main : Str -> Task {} [] }
    exposes []
    packages {}
    imports [Task.{ Task }]
    provides [mainForHost]

mainForHost : Str -> Task {} []
mainForHost = \file -> main file
