use embedded_svc::{ipv4, wifi};
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};
use esp_idf_sys::EspError;
use model::MacAddress;

pub async fn init(wifi: &mut AsyncWifi<EspWifi<'_>>) -> Result<MacAddress, EspError> {
    wifi.start().await?;
    log::info!("Wi-Fi started");

    let config = 'scan: loop {
        log::info!("starting new round of scanning");
        for wifi::AccessPointInfo { ssid, signal_strength, auth_method, .. } in wifi.scan().await? {
            let Some(name) = ssid.strip_prefix("DRIPPY_") else {
                log::warn!("skipping {ssid} [{signal_strength}]");
                continue;
            };
            log::info!("found network {name}");
            break 'scan wifi::Configuration::Client(wifi::ClientConfiguration {
                password: name.into(),
                ssid,
                auth_method,
                ..Default::default()
            });
        }
    };

    wifi.set_configuration(&config)?;
    wifi.connect().await?;
    log::info!("successfully connected to network");

    wifi.wait_netif_up().await?;
    let netif = wifi.wifi().sta_netif();
    let ipv4::IpInfo { ip, subnet, dns, secondary_dns } = netif.get_ip_info()?;
    match (dns, secondary_dns) {
        (Some(a), Some(b)) => log::info!("{ip} connected to {subnet} with DNS providers {a} and {b}"),
        (Some(dns), None) | (None, Some(dns)) => log::info!("{ip} connected to {subnet} with DNS provider {dns}"),
        _ => log::info!("{ip} connected to {subnet} without DNS providers"),
    }

    let mac = netif.get_mac()?;
    Ok(MacAddress(mac))
}
