platform "false-interpreter-test"
    requires {} { main : Str -> Task {} [] }
    exposes []
    packages {}
    imports [Task.{ Task }]
    provides [mainForHost]
# Test@!
mainForHost : Str -> Task {} []
mainForHost = \file -> main file
                 ## dsalkjdslajdsa