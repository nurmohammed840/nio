use std::panic::{RefUnwindSafe, UnwindSafe};

fn _join_handle_is_unwind_safe() {
    _is_unwind_safe::<nio::JoinHandle<()>>();
}

fn _net_types_are_unwind_safe() {
    _is_unwind_safe::<nio::net::TcpListener>();
    _is_unwind_safe::<nio::net::TcpReader>();
    _is_unwind_safe::<nio::net::TcpWriter>();
    _is_unwind_safe::<nio::net::TcpConnection>();
    _is_unwind_safe::<nio::net::TcpStream>();
    _is_unwind_safe::<nio::net::UdpSocket>();
}

fn _is_unwind_safe<T: UnwindSafe + RefUnwindSafe>() {}
