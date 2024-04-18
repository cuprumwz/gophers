#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

// extern crate alloc;
// use core::mem::MaybeUninit;
// use esp_backtrace as _;
// use esp_hal::{clock::ClockControl, peripherals::Peripherals, prelude::*, Delay};
// use esp_println::println;

// use esp_wifi::{initialize, EspWifiInitFor};

// use esp_hal::{systimer::SystemTimer, Rng};
// #[global_allocator]
// static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

// fn init_heap() {
//     const HEAP_SIZE: usize = 32 * 1024;
//     static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

//     unsafe {
//         ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
//     }
// }
// #[entry]
// fn main() -> ! {
//     init_heap();
//     let peripherals = Peripherals::take();
//     let system = peripherals.SYSTEM.split();

//     let clocks = ClockControl::max(system.clock_control).freeze();
//     let mut delay = Delay::new(&clocks);

//     // setup logger
//     // To change the log_level change the env section in .cargo/config.toml
//     // or remove it and set ESP_LOGLEVEL manually before running cargo run
//     // this requires a clean rebuild because of https://github.com/rust-lang/cargo/issues/10358
//     esp_println::logger::init_logger_from_env();
//     log::info!("Logger is setup");
//     println!("Hello world!");
//     let timer = SystemTimer::new(peripherals.SYSTIMER).alarm0;
//     let _init = initialize(
//         EspWifiInitFor::Wifi,
//         timer,
//         Rng::new(peripherals.RNG),
//         system.radio_clock_control,
//         &clocks,
//     )
//     .unwrap();
//     loop {
//         println!("Loop...");
//         delay.delay_ms(500u32);
//     }
// }

//======================================================================

// use embassy_net::tcp::TcpSocket;
// use embassy_net::{
//     Config, IpListenEndpoint, Ipv4Address, Ipv4Cidr, Stack, StackResources, StaticConfigV4,
// };
// use esp_hal as hal;

// use embassy_executor::Spawner;
// use embassy_time::{Duration, Timer};
// use esp_backtrace as _;
// use esp_println::{print, println};
// use esp_wifi::wifi::{AccessPointConfiguration, ClientConfiguration, Configuration};
// use esp_wifi::wifi::{
//     WifiApDevice, WifiController, WifiDevice, WifiEvent, WifiStaDevice, WifiState,
// };
// use esp_wifi::{initialize, EspWifiInitFor};
// use hal::clock::ClockControl;
// use hal::Rng;
// use hal::{embassy, peripherals::Peripherals, prelude::*, timer::TimerGroup};
// use static_cell::make_static;

// // const SSID: &str = env!("SSID");
// // const PASSWORD: &str = env!("PASSWORD");
// const SSID: &str = "tribersr99";
// const PASSWORD: &str = "test00";

// #[main]
// async fn main(spawner: Spawner) -> ! {
//     #[cfg(feature = "log")]
//     esp_println::logger::init_logger(log::LevelFilter::Info);

//     let peripherals = Peripherals::take();

//     let system = peripherals.SYSTEM.split();
//     let clocks = ClockControl::max(system.clock_control).freeze();

//     #[cfg(target_arch = "xtensa")]
//     let timer = hal::timer::TimerGroup::new(peripherals.TIMG1, &clocks).timer0;
//     #[cfg(target_arch = "riscv32")]
//     let timer = hal::systimer::SystemTimer::new(peripherals.SYSTIMER).alarm0;
//     let init = initialize(
//         EspWifiInitFor::Wifi,
//         timer,
//         Rng::new(peripherals.RNG),
//         system.radio_clock_control,
//         &clocks,
//     )
//     .unwrap();

//     let wifi = peripherals.WIFI;
//     let (wifi_ap_interface, wifi_sta_interface, mut controller) =
//         esp_wifi::wifi::new_ap_sta(&init, wifi).unwrap();

//     let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
//     embassy::init(&clocks, timer_group0);

//     let ap_config = Config::ipv4_static(StaticConfigV4 {
//         address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 2, 1), 24),
//         gateway: Some(Ipv4Address::from_bytes(&[192, 168, 2, 1])),
//         dns_servers: Default::default(),
//     });
//     let sta_config = Config::dhcpv4(Default::default());

//     let seed = 1234; // very random, very secure seed

//     // Init network stacks
//     let ap_stack = &*make_static!(Stack::new(
//         wifi_ap_interface,
//         ap_config,
//         make_static!(StackResources::<3>::new()),
//         seed
//     ));
//     let sta_stack = &*make_static!(Stack::new(
//         wifi_sta_interface,
//         sta_config,
//         make_static!(StackResources::<3>::new()),
//         seed
//     ));

//     let client_config = Configuration::Mixed(
//         ClientConfiguration {
//             ssid: SSID.try_into().unwrap(),
//             password: PASSWORD.try_into().unwrap(),
//             ..Default::default()
//         },
//         AccessPointConfiguration {
//             ssid: "esp-wifi".try_into().unwrap(),
//             ..Default::default()
//         },
//     );
//     controller.set_configuration(&client_config).unwrap();

//     spawner.spawn(connection(controller)).ok();
//     spawner.spawn(ap_task(&ap_stack)).ok();
//     spawner.spawn(sta_task(&sta_stack)).ok();

//     loop {
//         if sta_stack.is_link_up() {
//             break;
//         }
//         println!("Waiting for IP...");
//         Timer::after(Duration::from_millis(500)).await;
//     }
//     loop {
//         if ap_stack.is_link_up() {
//             break;
//         }
//         Timer::after(Duration::from_millis(500)).await;
//     }
//     println!("Connect to the AP `esp-wifi` and point your browser to http://192.168.2.1:8080/");
//     println!("Use a static IP in the range 192.168.2.2 .. 192.168.2.255, use gateway 192.168.2.1");

//     let mut ap_rx_buffer = [0; 1536];
//     let mut ap_tx_buffer = [0; 1536];

//     let mut ap_socket = TcpSocket::new(&ap_stack, &mut ap_rx_buffer, &mut ap_tx_buffer);
//     ap_socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

//     let mut sta_rx_buffer = [0; 1536];
//     let mut sta_tx_buffer = [0; 1536];

//     let mut sta_socket = TcpSocket::new(&sta_stack, &mut sta_rx_buffer, &mut sta_tx_buffer);
//     sta_socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

//     loop {
//         println!("Wait for connection...");
//         let r = ap_socket
//             .accept(IpListenEndpoint {
//                 addr: None,
//                 port: 8080,
//             })
//             .await;
//         println!("Connected...");

//         if let Err(e) = r {
//             println!("connect error: {:?}", e);
//             continue;
//         }

//         use embedded_io_async::Write;

//         let mut buffer = [0u8; 1024];
//         let mut pos = 0;
//         loop {
//             match ap_socket.read(&mut buffer).await {
//                 Ok(0) => {
//                     println!("AP read EOF");
//                     break;
//                 }
//                 Ok(len) => {
//                     let to_print =
//                         unsafe { core::str::from_utf8_unchecked(&buffer[..(pos + len)]) };

//                     if to_print.contains("\r\n\r\n") {
//                         print!("{}", to_print);
//                         println!();
//                         break;
//                     }

//                     pos += len;
//                 }
//                 Err(e) => {
//                     println!("AP read error: {:?}", e);
//                     break;
//                 }
//             };
//         }

//         if sta_stack.is_link_up() {
//             let remote_endpoint = (Ipv4Address::new(142, 250, 185, 115), 80);
//             println!("connecting...");
//             let r = sta_socket.connect(remote_endpoint).await;
//             if let Err(e) = r {
//                 println!("STA connect error: {:?}", e);
//                 continue;
//             }

//             use embedded_io_async::Write;
//             let r = sta_socket
//                 .write_all(b"GET / HTTP/1.0\r\nHost: www.mobile-j.de\r\n\r\n")
//                 .await;

//             if let Err(e) = r {
//                 println!("STA write error: {:?}", e);

//                 let r = ap_socket
//                     .write_all(
//                         b"HTTP/1.0 500 Internal Server Error\r\n\r\n\
//                         <html>\
//                             <body>\
//                                 <h1>Hello Rust! Hello esp-wifi! STA failed to send request.</h1>\
//                             </body>\
//                         </html>\r\n\
//                         ",
//                     )
//                     .await;
//                 if let Err(e) = r {
//                     println!("AP write error: {:?}", e);
//                 }
//             } else {
//                 let r = sta_socket.flush().await;
//                 if let Err(e) = r {
//                     println!("STA flush error: {:?}", e);
//                 } else {
//                     println!("connected!");
//                     let mut buf = [0; 1024];
//                     loop {
//                         match sta_socket.read(&mut buf).await {
//                             Ok(0) => {
//                                 println!("STA read EOF");
//                                 break;
//                             }
//                             Ok(n) => {
//                                 let r = ap_socket.write_all(&buf[..n]).await;
//                                 if let Err(e) = r {
//                                     println!("AP write error: {:?}", e);
//                                     break;
//                                 }
//                             }
//                             Err(e) => {
//                                 println!("STA read error: {:?}", e);
//                                 break;
//                             }
//                         }
//                     }
//                 }
//             }

//             sta_socket.close();
//         } else {
//             let r = ap_socket
//                 .write_all(
//                     b"HTTP/1.0 200 OK\r\n\r\n\
//                     <html>\
//                         <body>\
//                             <h1>Hello Rust! Hello esp-wifi! STA is not connected.</h1>\
//                         </body>\
//                     </html>\r\n\
//                     ",
//                 )
//                 .await;
//             if let Err(e) = r {
//                 println!("AP write error: {:?}", e);
//             }
//         }

//         let r = ap_socket.flush().await;
//         if let Err(e) = r {
//             println!("AP flush error: {:?}", e);
//         }
//         Timer::after(Duration::from_millis(1000)).await;

//         ap_socket.close();
//         Timer::after(Duration::from_millis(1000)).await;

//         ap_socket.abort();
//     }
// }

// #[embassy_executor::task]
// async fn connection(mut controller: WifiController<'static>) {
//     println!("start connection task");
//     println!("Device capabilities: {:?}", controller.get_capabilities());

//     println!("Starting wifi");
//     controller.start().await.unwrap();
//     println!("Wifi started!");

//     loop {
//         match esp_wifi::wifi::get_ap_state() {
//             WifiState::ApStarted => {
//                 println!("About to connect...");

//                 match controller.connect().await {
//                     Ok(_) => {
//                         // wait until we're no longer connected
//                         controller.wait_for_event(WifiEvent::StaDisconnected).await;
//                         println!("STA disconnected");
//                     }
//                     Err(e) => {
//                         println!("Failed to connect to wifi: {e:?}");
//                         Timer::after(Duration::from_millis(5000)).await
//                     }
//                 }
//             }
//             _ => return,
//         }
//     }
// }

// #[embassy_executor::task]
// async fn ap_task(stack: &'static Stack<WifiDevice<'static, WifiApDevice>>) {
//     stack.run().await
// }

// #[embassy_executor::task]
// async fn sta_task(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
//     stack.run().await
// }

//======================================================================

// use embassy_net::tcp::TcpSocket;
// use embassy_net::{
//     Config, IpListenEndpoint, Ipv4Address, Ipv4Cidr, Stack, StackResources, StaticConfigV4,
// };
// use esp_hal as hal;

// use embassy_executor::Spawner;
// use esp_println::{print, println};
// use esp_wifi::initialize;
// use hal::clock::ClockControl;
// use hal::peripheral;
// use hal::{embassy, peripherals::Peripherals, prelude::*, timer::TimerGroup};

// #[main]
// async fn main(spawner: Spawner) -> ! {
//     #[cfg(feature = "log")]
//     esp_println::logger::init_logger(log::LevelFilter::Info);

//     let peripherals = Peripherals::take();

//     let system = peripherals.SYSTEM.split();
//     let clocks = ClockControl::max(system.clock_control).freeze();

//     let timer = hal::systimer::SystemTimer::new(peripherals.SYSTIMER).alarm0;

//     let init = initialize(
//         esp_wifi::EspWifiInitFor::Wifi,
//         timer,
//         hal::Rng::new(peripherals.RNG),
//         system.radio_clock_control,
//         &clocks
//     ).unwrap();

//     let (wifi_ap_interface, wifi_sta_interface, mut controller) =
//         esp_wifi::wifi::new_ap_sta(&init, peripherals.WIFI).unwrap();

//     let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
//     embassy::init(&clocks, timer_group0);


//     loop {

//     }
// }

//======================================================================

use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, Ipv4Address, Stack, StackResources};

use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_println::println;
use esp_wifi::wifi::{ClientConfiguration, Configuration};
use esp_wifi::wifi::{WifiController, WifiDevice, WifiEvent, WifiStaDevice, WifiState};
use esp_wifi::{initialize, EspWifiInitFor};
use esp_hal as hal;
use hal::clock::ClockControl;
use hal::Rng;
use hal::{embassy, peripherals::Peripherals, prelude::*, timer::TimerGroup};
use static_cell::make_static;

// const SSID: &str = env!("SSID");
// const PASSWORD: &str = env!("PASSWORD");
const SSID: &str = "tribersr01";
const PASSWORD: &str = "password";

#[main]
async fn main(spawner: Spawner) -> ! {
    #[cfg(feature = "log")]
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let peripherals = Peripherals::take();

    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    let timer = hal::systimer::SystemTimer::new(peripherals.SYSTIMER).alarm0;
    let init = initialize(
        EspWifiInitFor::Wifi,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let wifi = peripherals.WIFI;
    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timer_group0);

    let config = Config::dhcpv4(Default::default());

    let seed = 1234; // very random, very secure seed

    // Init network stack
    let stack = &*make_static!(Stack::new(
        wifi_interface,
        config,
        make_static!(StackResources::<3>::new()),
        seed
    ));

    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(&stack)).ok();

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    loop {
        Timer::after(Duration::from_millis(1_000)).await;

        let mut socket = TcpSocket::new(&stack, &mut rx_buffer, &mut tx_buffer);

        socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

        let remote_endpoint = (Ipv4Address::new(142, 250, 185, 115), 80);
        println!("connecting...");
        let r = socket.connect(remote_endpoint).await;
        if let Err(e) = r {
            println!("connect error: {:?}", e);
            continue;
        }
        println!("connected!");
        let mut buf = [0; 1024];
        loop {
            use embedded_io_async::Write;
            let r = socket
                .write_all(b"GET / HTTP/1.0\r\nHost: www.mobile-j.de\r\n\r\n")
                .await;
            if let Err(e) = r {
                println!("write error: {:?}", e);
                break;
            }
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    println!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    println!("read error: {:?}", e);
                    break;
                }
            };
            println!("{}", core::str::from_utf8(&buf[..n]).unwrap());
        }
        Timer::after(Duration::from_millis(3000)).await;
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.get_capabilities());
    loop {
        match esp_wifi::wifi::get_wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start().await.unwrap();
            println!("Wifi started!");
        }
        println!("About to connect...");

        match controller.connect().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}
