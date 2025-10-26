// SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! # TCP server example
//!
//! This example shows how to start a server and implement basic register
//! read/write operations.

use std::{
    future,
    mem::transmute,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::net::TcpListener;

use tokio_modbus::{
    prelude::*,
    server::tcp::{Server, accept_tcp_connection},
};

#[derive(Clone, Debug, Default)]
#[repr(C)]
struct InstantaneousData {
    v_l1_n: i32,
    v_l2_n: i32,
    v_l3_n: i32,

    v_l1_l2: i32,
    v_l2_l3: i32,
    v_l3_l1: i32,

    a_l1: i32,
    a_l2: i32,
    a_l3: i32,

    w_l1: i32,
    w_l2: i32,
    w_l3: i32,

    va_l1: i32,
    va_l2: i32,
    va_l3: i32,

    var_l1: i32,
    var_l2: i32,
    var_l3: i32,

    v_l_n_sum: i32,
    v_l_l_sum: i32,
    w_sum: i32,
    va_sum: i32,
    var_sum: i32,

    pf_l1: i16,
    pf_l2: i16,
    pf_l3: i16,
    pf_sum: i16,

    phase_sequence: i16,

    hz: u16,

    kwh_plus_total: i32,
    kvarh_plus_total: i32,

    dmd_w_sum: i32,
    dmd_w_sum_max: i32,

    kwh_plus_par: i32,
    kvarh_plus_par: i32,

    kwh_plus_l1: i32,
    kwh_plus_l2: i32,
    kwh_plus_l3: i32,

    kwh_plus_t1: i32,
    kwh_plus_t2: i32,
    kwh_plus_t3: i32,
    kwh_plus_t4: i32,

    kwh_neg_total: i32,
}

struct ExampleService {
    instantaneous_data: Arc<Mutex<InstantaneousData>>,
}

impl tokio_modbus::server::Service for ExampleService {
    type Request = Request<'static>;
    type Response = Response;
    type Exception = ExceptionCode;
    type Future = future::Ready<Result<Self::Response, Self::Exception>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        println!("{req:X?}");

        let res = match req {
            Request::ReadHoldingRegisters(addr, cnt) => {
                // https://www.gavazziautomation.com/fileadmin/images/PIM/OTHERSTUFF/COMPRO/EM24_E1_CP.pdf
                match (addr, cnt) {
                    (0x000B, 1) => Ok(Response::ReadHoldingRegisters(vec![0x0670])), // Carlo Gavazzi identification code: EM24DINAV23XE1X
                    (0xA000, 1) => Ok(Response::ReadHoldingRegisters(vec![0x0007])), // Type of application: H
                    (0x0302, 1) => Ok(Response::ReadHoldingRegisters(vec![0x101E])), // Version and revision code of measurement module: 1.0.30
                    (0x0304, 1) => Ok(Response::ReadHoldingRegisters(vec![0x101E])), // Version and revision code of communication module: 1.0.30
                    (0x1002, 1) => Ok(Response::ReadHoldingRegisters(vec![0x0000])), // Measuring system: 3P.N
                    (0x5000, 7) => Ok(Response::ReadHoldingRegisters(
                        b"BY24600320011\0"
                            .chunks_exact(2)
                            .map(|word| u16::from_be_bytes(word.try_into().unwrap()))
                            .collect(),
                    )), // Serial number
                    (0xA100, 1) => Ok(Response::ReadHoldingRegisters(vec![0x0000])), // Front selector status: 0
                    (0x0000, 80) => Ok(Response::ReadHoldingRegisters({
                        let mut data = self.instantaneous_data.lock().unwrap();
                        data.w_l1 += 10;
                        let data = data.clone();
                        let words = unsafe { transmute::<_, [u16; 80]>(data) };
                        words.to_vec()
                    })), // Instantaneous data
                    _ => Err(ExceptionCode::IllegalFunction),
                }
            }
            _ => {
                println!(
                    "SERVER: Exception::IllegalFunction - Unimplemented function code in request: {req:?}"
                );
                Err(ExceptionCode::IllegalFunction)
            }
        };
        future::ready(res)
    }
}

impl ExampleService {
    fn new() -> Self {
        Self {
            instantaneous_data: Arc::new(Mutex::new(InstantaneousData {
                v_l1_n: 230_0,
                v_l2_n: 230_1,
                v_l3_n: 230_2,

                a_l1: -1_000,
                a_l2: 0,
                a_l3: 10_000,

                w_l1: -1_0,
                w_l2: 0,
                w_l3: 10_0,

                w_sum: 9_0,
                kwh_plus_total: 500_0,
                kwh_neg_total: 600_0,

                kwh_plus_l1: 10_0,
                kwh_plus_l2: 11_0,
                kwh_plus_l3: -12_0,

                ..Default::default()
            })),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let socket_addr = "0.0.0.0:502".parse().unwrap();

    server_context(socket_addr).await
}

async fn server_context(socket_addr: SocketAddr) -> anyhow::Result<()> {
    println!("Starting up server on {socket_addr}");
    let listener = TcpListener::bind(socket_addr).await?;
    let server = Server::new(listener);
    let new_service = |_socket_addr| Ok(Some(ExampleService::new()));
    let on_connected = |stream, socket_addr| async move {
        accept_tcp_connection(stream, socket_addr, new_service)
    };
    let on_process_error = |err| {
        eprintln!("{err}");
    };
    server.serve(&on_connected, on_process_error).await?;
    Ok(())
}
