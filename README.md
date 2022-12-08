![](./doc/vnix_logo.png)

This operating system is a proof of concept, i made it just for fun. Now it has a very draft kernel written in [Rust](https://www.rust-lang.org/).

If you don't understand something on this page or see that some info in unclear, you can chat me by contacts below. Also I welcome any support.

My Contacts:
- email: olegsajaxov@yandex.ru
- patreon: https://www.patreon.com/architector1324
- discord: Architector#9979

**Virtual Networking Information and Computing System (vnix)** – it is an operating system with non-Turing virtual networking architecture following principles:

- Conceptual:
  - All information is wrapped into messages and it always has owner, protected by cryptography (see note below).
  - Interact with OS and inside itself is provided by exchanging messages - no programming, no libraries or binaries at all.
  - Logically OS see no difference between single machine and compute cluster (see below).

`Note:` This doesn't mean a lack of anonymity. Rather, the owner can always confirm his authorship (see below).

- Technical:
  - **Operating System** is an set of *applications*, that provide *user* to control the computer or cluster. 
  - **Application** is an abstraction over a network of *services* that communicating by *messages* (very similar to [actors model](https://en.wikipedia.org/wiki/Actor_model)). Services may work on different machines, so applications may use distributed computing out of the box (using **tcp/[ipv6](https://en.wikipedia.org/wiki/IPv6)**).
  - **Service** is an interface to OS functionality that solves only one task and does it well. Unlike [microservice](https://en.wikipedia.org/wiki/Microservices), service solves a group of a very similar subtasks that are a variant of the same task. For example, output text, graphics, user interface, etc. to terminal.
  - **Message** is an information package and services communication unit, that consists of data *units* (very similar to [json](https://www.json.org), but more powerful).
  - **Unit** is an minimal information unit, that represents some data, like numbers, strings, lists, lazy data generators and etc.
  - **User** is an mathematical abstraction over pair of crypto-keys. Unlike in other systems, you create user once and use it on any device. Any message has a user's digital signature and every service is owned by some user.


![](./doc/concept.svg)

```
# ipv6 of current device:`aee3:3033:f655:4b00:10fd:fac0:634b:09a4`
# Example of application:
{
    app:`Example`
    task:{
      {val:123}:[{serv:`math.int`}]
      {serv:`math.int`}:[
          {serv:`io.term`}
          {
              serv:`io.term`
              adr:`569b:7761:af2d:6a81:4830:9c51:9d9d:9b78`
          }
      ]
    }
}
```


## Goals
- Let user make the computer to do what he want in the easiest way. No programming - only creating messages.
- Provide a very simple and in the same time powerful operating system.
- Once made software should work on any device, regardless of its architecture.
- Connect all your devices to compute cluster with zero efforts.

## Design
- **Vnix kernel** is a one big init process on the **unix-like** host kernel (**linux** for now). It emulates services network and use host kernel for low-level interaction with hardware.
- User space is platform-independent and all software should work on any device.
- Formally, **vnix** is a virtual OS. Logically, it works on compute cluster in the same way as on one machine.

![](./doc/design.svg)

Vnix kernel:
- Musl libc: POSIX-compatible C library.
- Host: Go runtime.
- Core: units, messages, users, services base, etc...
- Services: `io.term`, `math.int`, etc...

## How to build

1. Clone the repo:
```bash
git clone https://github.com/architector1324/vnix
cd vnix
```

2. Build vnix kernel for target arch:
```bash
rustup target add x86_64-unknown-linux-musl
cargo build --target=x86_64-unknown-linux-musl
```
```bash
rustup target add arm-unknown-linux-musleabi
cargo build --target=arm-unknown-linux-musleabi
```

## FAQ
- [Current progress](./PROGRESS.md).
- Take a look at draft [messages cookbook](./doc/message-cookbook.md).

Comming soon ...