//! Low level networking ffi bindings

extern crate libc;

use self::libc::{sockaddr, sockaddr_in, sockaddr_in6, in_addr, in6_addr, c_int, c_char, socklen_t, AF_INET, AF_INET6};
use std::mem::{size_of, transmute};
use std::net;
use std::ffi;

/*
 const char *
     inet_ntop(int af, const void * restrict src, char * restrict dst,
         socklen_t size);
*/
extern {
    fn getnameinfo(sa: *const sockaddr, salen: socklen_t, 
                   host: *mut c_char, hostlen: socklen_t, 
                   serv: *mut c_char, servlen: socklen_t, 
                   flags: c_int) -> c_int;
}



#[cfg(target_os = "linux")]
#[cfg(target_os = "android")]
fn new_sockaddr_in(port: u16, addr: in_addr) -> sockaddr_in {
    sockaddr_in {
        sin_family: AF_INET as u16,
        sin_port: port,
        sin_addr: addr,
        sin_zero: [0; 8]
    }
}
#[cfg(target_os = "macos")]
fn new_sockaddr_in(port: u16, addr: in_addr) -> sockaddr_in {
    sockaddr_in {
        sin_len: size_of::<sockaddr_in>() as u8,
        sin_family: AF_INET as u8,
        sin_port: port,
        sin_addr: addr,
        sin_zero: [0; 8]
    }
}

#[cfg(target_os = "linux")]
#[cfg(target_os = "android")]
fn new_sockaddr_in6(port: u16, addr: in6_addr) -> sockaddr_in6 {
    sockaddr_in6 {
        sin6_family: AF_INET6 as u16,
        sin6_port: port,
        sin6_flowinfo: 0,
        sin6_addr: addr,
        sin6_scope_id: 0,
    }
}
#[cfg(target_os = "macos")]
fn new_sockaddr_in6(port: u16, addr: in6_addr) -> sockaddr_in6 {
    sockaddr_in6 {
        sin6_len: size_of::<sockaddr_in6>() as u8,
        sin6_family: AF_INET6 as u8,
        sin6_port: port,
        sin6_flowinfo: 0,
        sin6_addr: addr,
        sin6_scope_id: 0,
    }
}

//static NI_NOFQDN   : c_int = 0x00000001;
//static NI_NUMERICHOST  : c_int = 0x00000002;
//static NI_NAMEREQD : c_int = 0x00000004;
//static NI_NUMERICSERV  : c_int = 0x00000008;
//static NI_DGRAM    : c_int = 0x00000010;
/// Returns the hostname for an ip address
/// TODO: make this safe, see manpage
const HOSTLEN: usize = 80;
pub fn get_nameinfo(peer_socket: net::SocketAddr) -> String {
    let port = peer_socket.port();
    let mut buf = [0; HOSTLEN];
    let _ = unsafe {
        match peer_socket {
            net::SocketAddr::V4(addr) => {
                let [a, b, c, d] = addr.ip().octets();
                let addr = in_addr {
                    s_addr: (a as u32) << 24 
                          | (b as u32) << 16 
                          | (c as u32) << 8 
                          | (d as u32)
                };
                let sockaddr = new_sockaddr_in(port, addr);
                getnameinfo(transmute(&sockaddr), size_of::<sockaddr_in>() as socklen_t, 
                            buf.as_mut_ptr() as *mut i8, HOSTLEN as u32, transmute(0usize), 0, 0)
            },
            net::SocketAddr::V6(addr) => {
                let [a, b, c, d, e, f, g, h] = addr.ip().segments();
                let addr =  transmute([a, b, c, d, e, f, g, h]);
                let sockaddr = new_sockaddr_in6(port, addr);
                getnameinfo(transmute(&sockaddr), size_of::<sockaddr_in6>() as socklen_t, 
                            buf.as_mut_ptr() as *mut i8, HOSTLEN as u32, transmute(0usize), 0, 0)
            },
        }
   
    };
    unsafe {String::from_utf8_lossy(ffi::CStr::from_ptr(buf.as_ptr()).to_bytes()).into_owned()}

}

