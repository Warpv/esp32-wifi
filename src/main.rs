use esp_idf_svc::{
    hal::{peripheral::Peripheral, peripherals::Peripherals}, 
    eventloop::EspSystemEventLoop, 
    nvs::{EspNvsPartition, NvsDefault, EspDefaultNvsPartition}, 
    wifi::{EspWifi, AsyncWifi, Configuration, AuthMethod, ClientConfiguration, self},
    timer::{Task, EspTimerService, EspTaskTimerService},
    ping::EspPing, http::{server::EspHttpServer, client::Request}
};
use std::{
    thread::sleep,
    time::Duration
};

const SSID: &str = "YORE_SSID";
const PASS: &str = "YORE_PASS";


fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();
    
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("buy world!");

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take().unwrap();
    let timer_service = EspTaskTimerService::new().unwrap();
    let _wifi = wifi(
        peripherals.modem,
        sysloop,
        Some(EspDefaultNvsPartition::take().unwrap()),
        timer_service,
    );

    let mut server = EspHttpServer::new(&Default::default()).expect("Failed to create HTTP server");

    server.fn_handler("/", esp_idf_svc::http::Method::Get, |req| {
            Ok({
                let mut response = req.into_ok_response().expect("Failed to create response");
                response.write("hi princess".as_bytes()).expect("Failed to set body");
            })
        }).expect("Failed to register handler");


    loop {
        sleep(Duration::from_secs(1));
    }
}


pub async fn wifi(
    modem: impl Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
    nvs: Option<EspNvsPartition<NvsDefault>>,
    timer_service: EspTimerService<Task>,
) -> anyhow::Result<AsyncWifi<EspWifi<'static>>> {

    use futures::executor::block_on;

    let mut wifi = AsyncWifi::wrap(
        EspWifi::new(modem, sysloop.clone(), nvs)?,
        sysloop,
        timer_service.clone(),
    )?;

    block_on(connect_wifi(&mut wifi))?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    println!("Wifi DHCP info: {:?}", ip_info);
    
    EspPing::default().ping(ip_info.subnet.gateway, &esp_idf_svc::ping::Configuration::default())?;
    Ok(wifi)

}


async fn connect_wifi(wifi: &mut AsyncWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: PASS.into(),
        channel: None,
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start().await?;
    use log::info;
    info!("Wifi started");

    wifi.connect().await?;
    info!("Wifi connected");

    wifi.wait_netif_up().await?;
    info!("Wifi netif up");

    Ok(())
}

    
