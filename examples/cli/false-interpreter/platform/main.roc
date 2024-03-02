platform "false-interpreter-test"
    requires {} { main : Str -> Task {} [] }
    exposes []
    packages {}
    imports [Task.{ Task }]
    provides [mainForHost]
                    ## Test doc comment
mainForHost : Str -> Task {} []
mainForHost = \file -> main file
