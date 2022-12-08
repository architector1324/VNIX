# Vnix pre-alpha v0.1
## Features

1. Simple units type system:
    - basic (`none`, `bool`, `byte`, `int`, `float`, `str`)
    - collections (`pair`, `list`, `msg`)
2. Arbitrary messages (`{<unit>:<unit> ...}`)
3. Service:
    - lazy instances generation by id
    - instance state
    - instance messages mailbox
4. Users and security:
    - **user** is and abstraction over messages and services instances, represents as 2 crypto-key pairs (for encryption and signing)
    - messages are owned by user (have a user's **digital signature**)
    - services are owned by user (create and verify messages by user)
    - messages are encrypted outside kernel reach (on disk or external network)
    - services policy (determines service instance behaviour with messages from another user)
5. Services network:
    - internal (communication with messages inside kernel)
    - external (communication with messages outside kernel by the internet using **ipv6**)
6. Simple data generation (with `gen.*` services)
7. Powerful integer math (with `gen.int` service)
9. Simple tensor generation (with services `gen.logic`, `gen.byte`, `gen.int`, `gen.float`)
9. Simple user interface (**ui** on `io.term`)
10. System-wide k/v database (`io.store`)
11. Powerful parsing system (with `etc.parser` and `etc.ast`)
12. State machines (with `etc.fsm`)
13. Time control (with `etc.chrono`)

## Services

1. I/O:
    - `io.term` - interacting user with terminal
    - `io.store` - store messages on disk
2. Data generators:
    - `gen.logic`
    - `gen.byte`
    - `gen.int`
    - `gen.float`
    - `gen.str`
3. System:
    - `sys.task` - services network deployment
    - `sys.user` - users control
    - `sys.kern` - kernel control
4. Other:
    - `etc.parser` - parser generator
    - `etc.ast` - tree transformer
    - `etc.fsm` - finite state machine
    - `etc.chrono` - time control


## Applications

1. lambda - interactive shell for realtime task creation and execution
2. me - simple message creator