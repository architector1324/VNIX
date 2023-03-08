# Vnix pre-alpha v1.0

## Features

1. [x] Simple units type system:
    - [x] basic (`none`, `bool`, `byte`, `int`, `dec`, `str`)
    - [x] collections (`pair`, `list`, `map`)
    - [x] complex (`ref`, `stream`)
2. [x] Vnix message notation [vxmn] (`{<unit>:<unit> ...}`)
3. [ ] Service:
    - [x] send/recv msg communication
    - [x] message handling
    - [ ] logging
4. [ ] Users and security:
    - [x] **user** is and abstraction over messages and services instances, represents as 2 crypto-key pairs (for encryption and signing)
    - [x] messages are owned by user (have a user's **digital signature**)
    - [x] services are owned by user (create and verify messages by user)
    - [ ] messages are encrypted outside kernel reach (on disk or external network)
    - [ ] services policy (determines service instance behaviour with messages from another user)
5. [ ] Services network:
    - [x] internal (communication with messages inside kernel)
    - [ ] external (communication with messages outside kernel by the internet using **ipv6**)
6. [x] Powerful arbitary numbers math calculations (with `math.calc` service)
7. [ ] Tensor math computation (with service `math.tensor`)
8. [ ] Console and graphical user interface (**ui** on `io.term`)
9. [x] System-wide unit-based database (`io.store`)
10. [ ] Powerful parsing system (with `etc.parser` and `etc.ast`)
11. [ ] State machines (with `etc.fsm`)
12. [x] Time control (with `time.chrono`)

## Services

1. [ ] I/O:
    - [ ] `io.term` - interacting user with terminal
    - [x] `io.store` - store messages on disk/ram database
2. [ ] Math:
    - [x] `math.calc` - numbers calculation
3. [x] System:
    - [x] `sys.usr` - users management
    - [x] `sys.task` - run task from message
    - [x] `sys.hw` - hardware management
4. [ ] Graphics:
    - [x] `gfx.2d` - generate 2d image
    - [ ] `gfx.3d` - generate image with shading
    - [ ] `gfx.rt` - generate image with raytracer
5. [x] Time:
    - [x] `time.chrono` - time control
6. [x] Test:
    - [x] `test.echo` - echo service
    - [x] `test.dump` - simple test printing service
5. [ ] Other:
    - [ ] `etc.parser` - parser generator
    - [ ] `etc.ast` - tree transformer
    - [ ] `etc.fsm` - finite state machine

## Applications

1. [x] lambda - interactive shell for realtime task creation and execution
2. [ ] me - simple message creator
3. [ ] zen - graphical desktop environment
