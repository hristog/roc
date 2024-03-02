platform "effects-test"
    requires {} { main : Effect.Effect {} }
    exposes []
    packages {}
    imports [Effect]
    provides [mainForHost]

mainForHost : Effect.Effect {}
mainForHost = main
