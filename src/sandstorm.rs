extern crate sandcrust;

extern crate nix;
extern crate libc;
extern crate errno;

use nix::unistd::getpid;
use libc::{readlink, c_char};
use std::ffi::CString;
use errno::errno;

use sandcrust::sandbox_me;


pub fn get_mnt_ns() {
    let pid = getpid();
    // FIXME some nicer way to build a path?
    let pathstr = "/proc/".to_string() + &pid.to_string() + "/ns/mnt";
    let path = CString::new(pathstr).unwrap();

    // jeez this is ugly as fuck
    let mut x: Vec<c_char> = vec![0; 256];
    let slice = x.as_mut_slice();
    let bufptr = slice.as_mut_ptr();

    unsafe {
        if readlink(path.as_ptr(), bufptr, 255) > 0 {
            let contents = CString::from_raw(bufptr).into_string().unwrap();
            println!("mnt ns: {}", contents);
        } else {
            let e = errno();
            println!("read failed: {}", e);
        }
    }
}


pub fn main() {
    sandbox_me(get_mnt_ns);
}
