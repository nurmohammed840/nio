use std::panic::{RefUnwindSafe, UnwindSafe};

fn _join_handle_is_unwind_safe() {
    _is_unwind_safe::<nio_rt::JoinHandle<()>>();
}

fn _net_types_are_unwind_safe() {
    _is_unwind_safe::<nio_rt::net::TcpListener>();
    _is_unwind_safe::<nio_rt::net::TcpReader>();
    _is_unwind_safe::<nio_rt::net::TcpWriter>();
    _is_unwind_safe::<nio_rt::net::TcpConnection>();
    _is_unwind_safe::<nio_rt::net::TcpStream>();
    _is_unwind_safe::<nio_rt::net::UdpSocket>();
}

fn _is_unwind_safe<T: UnwindSafe + RefUnwindSafe>() {}
