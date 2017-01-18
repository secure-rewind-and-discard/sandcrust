pub extern crate nix;

extern crate bincode;
extern crate rustc_serialize;

extern crate sandheap;

// this is needed because e.g. fork is exposed in the macro, while the functions from other crates are not
pub use nix as sandcrust_nix;

use std::os::unix::io::FromRawFd;

use sandheap as sandbox;

// needed as a wrapper for all the imported uses
#[doc(hidden)]
pub struct Sandcrust {
    file_in: ::std::fs::File,
    file_out: ::std::fs::File,
}

impl Sandcrust {
        pub fn new() -> Sandcrust {
            let (fd_out, fd_in) = sandcrust_nix::unistd::pipe().unwrap();
            Sandcrust {
                file_in: unsafe { ::std::fs::File::from_raw_fd(fd_in) },
                file_out: unsafe { ::std::fs::File::from_raw_fd(fd_out) },
            }
    }

    pub fn setup_child(&self) {
        sandbox::setup();
    }

    pub fn join_child(&self, child: sandcrust_nix::libc::pid_t) {
        match sandcrust_nix::sys::wait::waitpid(child, None) {
            Ok(_) => {}
            Err(e) => println!("sandcrust waitpid() failed with error {}", e),
        }
    }

    pub fn put_var_in_fifo<T: ::rustc_serialize::Encodable>(&mut self, var: T) {
       ::bincode::rustc_serialize::encode_into(&var, &mut self.file_in, ::bincode::SizeLimit::Infinite).unwrap();
    }

    pub fn restore_var_from_fifo<T: ::rustc_serialize::Decodable>(&mut self, var: &mut T) {
        *var = ::bincode::rustc_serialize::decode_from(&mut self.file_out, ::bincode::SizeLimit::Infinite).unwrap();
    }

    pub fn decode_retval<T: ::rustc_serialize::Decodable>(&mut self) -> Option<T> {
        ::bincode::rustc_serialize::decode_from(&mut self.file_out, ::bincode::SizeLimit::Infinite).unwrap()
    }
}


// FIXME: somehow refactor
#[macro_export]
macro_rules! store_vars {
    ($sandcrust:ident, &mut $head:ident) => { $sandcrust.put_var_in_fifo($head); };
    ($sandcrust:ident, &mut $head:ident, $($tail:tt)*) => {
        $sandcrust.put_var_in_fifo($head);
        store_vars!($sandcrust, $($tail)*);
    };
    ($sandcrust:ident, &$head:ident) => { };
    ($sandcrust:ident, &$head:ident, $($tail:tt)+) => {
        store_vars!($sandcrust, $($tail)*);
    };
    ($sandcrust:ident, $head:ident) => { };
    ($sandcrust:ident, $head:ident, $($tail:tt)+) => {
        store_vars!($sandcrust, $($tail)*);
    };
    ($sandcrust:ident, ) => {};
}


#[macro_export]
macro_rules! restore_vars {
    // only restore mut types
    ($sandcrust:ident, &mut $head:ident) => {
        $sandcrust.restore_var_from_fifo(&mut $head);
    };
    ($sandcrust:ident, &mut $head:ident, $($tail:tt)*) => {
        $sandcrust.restore_var_from_fifo(&mut $head);
        restore_vars!($sandcrust, $($tail)*);
    };
    ($sandcrust:ident, &$head:ident) => { };
    ($sandcrust:ident, &$head:ident, $($tail:tt)+) => { restore_vars!($sandcrust, $($tail)*); };
    ($sandcrust:ident, $head:ident) => { };
    ($sandcrust:ident, $head:ident, $($tail:tt)+) => { restore_vars!($sandcrust, $($tail)*); };
    ($sandcrust:ident, ) => {};
}


#[macro_export]
macro_rules! sandbox_me {
    // FIXME
    // handle no arg and/or ret val cases here
    // and more: use $crate

    // potentially args, no retval
     ($f:ident($($x:tt)*)) => {{
        let mut sandcrust = Sandcrust::new();
        let child: sandcrust_nix::libc::pid_t = match sandcrust_nix::unistd::fork() {
            Ok(sandcrust_nix::unistd::ForkResult::Parent { child, .. }) => {
                restore_vars!(sandcrust, $($x)*);
                child
            },
            Ok(sandcrust_nix::unistd::ForkResult::Child) => {
                sandcrust.setup_child();
               let retval = $f($($x)*);
                store_vars!(sandcrust, $($x)*);
                // test for an empty ( () ) return value and construct enum accordingly
                let retval_option = if ::std::mem::size_of_val(&retval) > 0 { Some(retval) } else { None };
                sandcrust.put_var_in_fifo(&retval_option);
                ::std::process::exit(0);
            }
            Err(e) => panic!("sandcrust: fork() failed with error {}", e),
        };
        let retval = match sandcrust.decode_retval() {
            Some(value) => value,
            None => 0,
        };
        sandcrust.join_child(child);
        retval
     }};
}
