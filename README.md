# What

Toy for adapting a local FUSE file system to a RESTful service

# Why

Practicing Rust and learning how to adapting Python in the language

# How

                                                          ┌───────────────────────┐
                                                          │    RESTful service    │
                                                          │                       │
                                                          └───────────────────────┘
                                                                      ▲
                                                                      │
──────────────────────────────────────────────────────────────────────┼────────────────────────────────────────────────────────
                                                                      │
                                                                      │                   extend this for
                                                                      │                  specific service
                                                       ┌─────────────────────────────┐                      ╔════════════════════╗
                                                       │    abstract hooks method    │          ┌───────────║    Python class    ║
                                                       │                             │          │           ╚════════════════════╝
                                                       │  (before/after committing   │◁─────────┤
                                                       │     RESTful operations)     │          │           ╔════════════════════╗
                                                       └─────────────────────────────┘          └───────────║     Rust class     ║
                                                                      □                                     ╚════════════════════╝
                                                                      │
                                                                      │
                      ┌───────────────────────┐        ┌─────────────────────────────┐
                      │                       │        │                             │
                      │   PyO3: restfs_lib    │        │    restfs_lib.Filesystem    │
                      │     (Rust Python)     │───────□│   (Rust as Python class)    │──┐
                      │                       │        │                             │  │
                      └───────────────────────┘        └─────────────────────────────┘  │
                                  │                                                     │
                                  │                       do call RESTful service       │
                                  ▼                   turn results to file operations   │
                      ┌───────────────────────┐                                         │
                      │                       │                                         │
                      │      crate fuse       │                                         │
                      │        (Rust)         │◀────────────────────────────────────────┘
                      │                       │
                      └───────────────────────┘
                                  │
                                  │
  ────────────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────
                                  │
                                  ▼
                      ┌───────────────────────┐
                      │      FUSE MacOSX      │
                      │                       │
                      └───────────────────────┘


