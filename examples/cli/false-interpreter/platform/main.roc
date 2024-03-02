platform "false-interpreter"
    requires {} { main : Str -> Task {} [] }
    exposes []
    packages {}
    imports [Task.{ Task }]
    provides [mainForHost]
                         ## Comment test comment!
mainForHost : Str -> Task {} []
mainForHost = \file -> main file
