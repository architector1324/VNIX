use super::driver::CLI;


pub fn vnix_entry(cli: &mut dyn CLI) {
    cli.reset().expect("cannot reset cli!");
    cli.println(core::format_args!("Hello, vnix ®!"));

    loop {

    }
}
