platform "false-interpreter"
    requires {} { main : Str -> Task {} [] }
    exposes []
    packages {}
    imports [Task.{ Task }]
    provides [mainForHost]
                # A random roc comment here
mainForHost : Str -> Task {} []
mainForHost = \file -> main file
