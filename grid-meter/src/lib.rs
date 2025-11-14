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

// https://www.gavazziautomation.com/fileadmin/images/PIM/OTHERSTUFF/COMPRO/EM24_E1_CP.pdf
// All relevant fields
// {
//     v_l1_n: 230_0, // 230V
//     v_l2_n: 230_1,
//     v_l3_n: 230_2,

//     a_l1: -1_000, // -1A
//     a_l2: 0,
//     a_l3: 10_000,

//     w_l1: -1_0, // -1W
//     w_l2: 0,
//     w_l3: 10_0,

//     w_sum: 9_0, // 9W
//     kwh_plus_total: 500_0, // 500kWh
//     kwh_neg_total: 600_0,

//     kwh_plus_l1: 10_0, // 10kWh
//     kwh_plus_l2: 11_0,
//     kwh_plus_l3: -12_0,

//     ..Default::default()
// }
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct InstantaneousData {
    pub v_l1_n: i32,
    pub v_l2_n: i32,
    pub v_l3_n: i32,

    pub v_l1_l2: i32,
    pub v_l2_l3: i32,
    pub v_l3_l1: i32,

    pub a_l1: i32,
    pub a_l2: i32,
    pub a_l3: i32,

    pub w_l1: i32,
    pub w_l2: i32,
    pub w_l3: i32,

    pub va_l1: i32,
    pub va_l2: i32,
    pub va_l3: i32,

    pub var_l1: i32,
    pub var_l2: i32,
    pub var_l3: i32,

    pub v_l_n_sum: i32,
    pub v_l_l_sum: i32,
    pub w_sum: i32,
    pub va_sum: i32,
    pub var_sum: i32,

    pub pf_l1: i16,
    pub pf_l2: i16,
    pub pf_l3: i16,
    pub pf_sum: i16,

    pub phase_sequence: i16,

    pub hz: u16,

    pub kwh_plus_total: i32,
    pub kvarh_plus_total: i32,

    pub dmd_w_sum: i32,
    pub dmd_w_sum_max: i32,

    pub kwh_plus_par: i32,
    pub kvarh_plus_par: i32,

    pub kwh_plus_l1: i32,
    pub kwh_plus_l2: i32,
    pub kwh_plus_l3: i32,

    pub kwh_plus_t1: i32,
    pub kwh_plus_t2: i32,
    pub kwh_plus_t3: i32,
    pub kwh_plus_t4: i32,

    pub kwh_neg_total: i32,
}

struct GridMeterService {
    instantaneous_data: Arc<Mutex<InstantaneousData>>,
    measuring_system: MeasuringSystem,
    serial_number: &'static [u8],
}

impl tokio_modbus::server::Service for GridMeterService {
    type Request = Request<'static>;
    type Response = Response;
    type Exception = ExceptionCode;
    type Future = future::Ready<Result<Self::Response, Self::Exception>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let res = match req {
            Request::ReadHoldingRegisters(addr, cnt) => {
                // https://www.gavazziautomation.com/fileadmin/images/PIM/OTHERSTUFF/COMPRO/EM24_E1_CP.pdf
                match (addr, cnt) {
                    (0x000B, 1) => Ok(Response::ReadHoldingRegisters(vec![0x0670])), // Carlo Gavazzi identification code: EM24DINAV23XE1X
                    (0xA000, 1) => Ok(Response::ReadHoldingRegisters(vec![0x0007])), // Type of application: H
                    (0x0302, 1) => Ok(Response::ReadHoldingRegisters(vec![0x101E])), // Version and revision code of measurement module: 1.0.30
                    (0x0304, 1) => Ok(Response::ReadHoldingRegisters(vec![0x101E])), // Version and revision code of communication module: 1.0.30
                    (0x1002, 1) => Ok(Response::ReadHoldingRegisters(vec![
                        self.measuring_system as u16,
                    ])), // Measuring system
                    (0x5000, 7) => Ok(Response::ReadHoldingRegisters(
                        self.serial_number
                            .chunks_exact(2)
                            .map(|word| u16::from_be_bytes(word.try_into().unwrap()))
                            .collect(),
                    )), // Serial number, e.g.: b"BY24600320011\0"
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

impl GridMeterService {
    fn new(
        instantaneous_data: Arc<Mutex<InstantaneousData>>,
        measuring_system: MeasuringSystem,
        serial_number: &'static [u8],
    ) -> Self {
        Self {
            instantaneous_data,
            measuring_system,
            serial_number,
        }
    }
}

pub async fn run_grid_meter_server(
    socket_addr: SocketAddr,
    instantaneous_data: Arc<Mutex<InstantaneousData>>,
    measuring_system: MeasuringSystem,
    serial_number: &'static [u8],
) -> anyhow::Result<()> {
    println!("Starting up grid meter server on {socket_addr}");
    let listener = TcpListener::bind(socket_addr).await?;
    let server = Server::new(listener);
    let new_service = |_socket_addr| {
        Ok(Some(GridMeterService::new(
            instantaneous_data.clone(),
            measuring_system,
            serial_number,
        )))
    };
    let on_connected = |stream, socket_addr| async move {
        accept_tcp_connection(stream, socket_addr, new_service)
    };
    let on_process_error = |err| {
        eprintln!("{err}");
    };
    server.serve(&on_connected, on_process_error).await?;
    Ok(())
}

#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum MeasuringSystem {
    /// 3P.N
    Setup3PN = 0,
    /// 3P.1
    Setup3P1 = 1,
    /// 2P
    Setup2P = 2,
    /// 1P
    Setup1P = 3,
    /// 3P
    Setup3P = 4,
}
